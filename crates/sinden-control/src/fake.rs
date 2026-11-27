//! A fake driver serving the control API on a Unix socket, for tests and
//! for developing the UI without a gun or the driver.
//!
//! It keeps the configuration in memory and answers `version`, `status`,
//! `get_config`, `set_config`, `subscribe` and `firmware_update` as the
//! driver would. A firmware update is simulated: the file is checked as
//! the driver does (Intel HEX, its USB identifiers against the gun's
//! unless forced), then the gun leaves the status, the `firmware` events
//! go through their phases and the gun comes back with a new firmware
//! version (or the update fails, see
//! [`FakeDriver::fail_next_firmware_update`]).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::firmware::FirmwareImage;
use crate::{
    ConfigSnapshot, Event, FirmwarePhase, FirmwareProgress, Gun, GunState, Guns, Status, UsbId,
};

/// A `firmware_update` the fake driver accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareCall {
    /// The gun's device.
    pub device: String,
    /// The length of the file's text.
    pub hex_len: usize,
    /// Whether it was forced.
    pub force: bool,
}

#[derive(Debug)]
struct FirmwareSim {
    step: Duration,
    new_version: String,
    fail_next: Option<String>,
    running: bool,
    /// A gun a failed update left "in its bootloader": out of the status,
    /// reachable again by its device.
    waiting: Option<Gun>,
    calls: Vec<FirmwareCall>,
}

impl Default for FirmwareSim {
    fn default() -> Self {
        Self {
            step: Duration::ZERO,
            new_version: "2.1".into(),
            fail_next: None,
            running: false,
            waiting: None,
            calls: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
struct Shared {
    config: BTreeMap<String, String>,
    guns: Guns,
    subscribers: Vec<UnixStream>,
    set_config_calls: Vec<BTreeMap<String, String>>,
    firmware: FirmwareSim,
}

/// A running fake driver; stops accepting when dropped (the socket file is
/// removed).
#[derive(Debug)]
pub struct FakeDriver {
    path: PathBuf,
    shared: Arc<Mutex<Shared>>,
}

impl FakeDriver {
    /// Starts serving on `path` (an existing file there is replaced).
    pub fn spawn(
        path: impl AsRef<Path>,
        config: BTreeMap<String, String>,
        guns: Vec<Gun>,
    ) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let shared = Arc::new(Mutex::new(Shared {
            config,
            guns: Guns::from_status(Status { guns }),
            ..Shared::default()
        }));
        let accept_shared = Arc::clone(&shared);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&accept_shared);
                thread::spawn(move || serve(&stream, &shared));
            }
        });
        Ok(Self { path, shared })
    }

    /// The socket path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Applies `event` to the guns and sends it to every subscriber.
    pub fn emit(&self, event: &Event) {
        emit(&self.shared, event);
    }

    /// The time between two events of a simulated firmware update (none
    /// by default).
    pub fn set_firmware_step(&self, step: Duration) {
        lock(&self.shared).firmware.step = step;
    }

    /// The firmware version a gun comes back with (`2.1` by default).
    pub fn set_new_firmware_version(&self, version: impl Into<String>) {
        lock(&self.shared).firmware.new_version = version.into();
    }

    /// Makes the next firmware update fail during its write, with `error`;
    /// the gun then stays out of the status, as in its bootloader, until an
    /// update of its device succeeds.
    pub fn fail_next_firmware_update(&self, error: impl Into<String>) {
        lock(&self.shared).firmware.fail_next = Some(error.into());
    }

    /// The `firmware_update` requests accepted so far.
    #[must_use]
    pub fn firmware_calls(&self) -> Vec<FirmwareCall> {
        lock(&self.shared).firmware.calls.clone()
    }

    /// The guns in the status, sorted by player.
    #[must_use]
    pub fn guns(&self) -> Vec<Gun> {
        lock(&self.shared).guns.as_slice().to_vec()
    }

    /// The number of connected subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        lock(&self.shared).subscribers.len()
    }

    /// The `values` of every `set_config` received so far.
    #[must_use]
    pub fn set_config_calls(&self) -> Vec<BTreeMap<String, String>> {
        lock(&self.shared).set_config_calls.clone()
    }
}

impl Drop for FakeDriver {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        for subscriber in &lock(&self.shared).subscribers {
            let _ = subscriber.shutdown(std::net::Shutdown::Both);
        }
    }
}

fn lock(shared: &Mutex<Shared>) -> std::sync::MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

fn emit(shared: &Mutex<Shared>, event: &Event) {
    let mut shared = lock(shared);
    shared.guns.apply(event);
    let mut line = serde_json::to_vec(event).expect("events serialise");
    line.push(b'\n');
    shared
        .subscribers
        .retain_mut(|s| s.write_all(&line).is_ok());
}

fn serve(stream: &UnixStream, shared: &Arc<Mutex<Shared>>) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    for line in BufReader::new(read_half).lines() {
        let Ok(line) = line else { return };
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            let _ = writeln!(writer, "{}", json!({"id": null, "error": "invalid JSON"}));
            continue;
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = match method {
            "subscribe" => {
                let mut shared = lock(shared);
                // The driver answers with the current status, then sends
                // only events on this connection.
                let status = json!({"guns": shared.guns});
                if writeln!(writer, "{}", json!({"id": id, "result": status})).is_ok()
                    && let Ok(subscriber) = stream.try_clone()
                {
                    shared.subscribers.push(subscriber);
                }
                continue;
            }
            "version" => Ok(json!({"driver": "0.0.0-fake", "api": crate::API_VERSION})),
            "status" => Ok(json!({"guns": lock(shared).guns})),
            "get_config" => Ok(snapshot(&lock(shared))),
            "set_config" => set_config(&mut lock(shared), request.get("params")),
            "firmware_update" => firmware_update(shared, request.get("params")),
            other => Err(format!("unknown method '{other}'")),
        };
        let line = match response {
            Ok(result) => json!({"id": id, "result": result}),
            Err(error) => json!({"id": id, "error": error}),
        };
        if writeln!(writer, "{line}").is_err() {
            return;
        }
    }
}

fn snapshot(shared: &Shared) -> Value {
    serde_json::to_value(ConfigSnapshot {
        path: "fake://LightgunMono.exe.config".into(),
        values: shared.config.clone(),
    })
    .expect("snapshots serialise")
}

fn set_config(shared: &mut Shared, params: Option<&Value>) -> Result<Value, String> {
    let values = params
        .and_then(|p| p.get("values"))
        .and_then(Value::as_object)
        .ok_or("set_config needs params.values")?;
    let mut parsed = BTreeMap::new();
    for (key, value) in values {
        let value = value
            .as_str()
            .ok_or_else(|| format!("value of '{key}' is not a string"))?;
        parsed.insert(key.clone(), value.to_owned());
    }
    shared.config.extend(parsed.clone());
    shared.set_config_calls.push(parsed);
    Ok(snapshot(shared))
}

fn firmware_update(shared: &Arc<Mutex<Shared>>, params: Option<&Value>) -> Result<Value, String> {
    let params = params
        .and_then(Value::as_object)
        .ok_or("firmware_update needs params")?;
    let hex = params
        .get("hex")
        .and_then(Value::as_str)
        .ok_or("firmware_update needs params.hex")?;
    let force = params
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut state = lock(shared);
    let gun = match (params.get("device"), params.get("player")) {
        (Some(Value::String(device)), _) => state
            .guns
            .as_slice()
            .iter()
            .chain(&state.firmware.waiting)
            .find(|g| g.device == *device)
            .cloned()
            .ok_or_else(|| format!("no gun at {device}"))?,
        (None, Some(player)) => state
            .guns
            .as_slice()
            .iter()
            .find(|g| Some(u64::from(g.player)) == player.as_u64())
            .cloned()
            .ok_or_else(|| format!("no gun for player {player}"))?,
        _ => return Err("firmware_update needs params.device or params.player".into()),
    };
    if state.firmware.running {
        return Err("a firmware update is already running".into());
    }
    let image = FirmwareImage::parse(hex).map_err(|e| format!("not a firmware file: {e}"))?;
    let ids = image.usb_ids();
    let names = |ids: &[UsbId]| {
        ids.iter()
            .map(UsbId::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !force {
        match gun.usb_id() {
            _ if ids.is_empty() => {
                return Err(
                    "the image has no Sinden USB identifiers: refused without force".into(),
                );
            }
            Some(id) if !ids.contains(&id) => {
                return Err(format!(
                    "the image is for {} and the gun is {id}: refused without force",
                    names(&ids)
                ));
            }
            _ => {}
        }
    }
    state.firmware.running = true;
    state.firmware.calls.push(FirmwareCall {
        device: gun.device.clone(),
        hex_len: hex.len(),
        force,
    });
    let failure = state.firmware.fail_next.take();
    let step = state.firmware.step;
    let mut back = gun.clone();
    back.firmware.clone_from(&state.firmware.new_version);
    if let Some(&id) = ids.first() {
        back.vendor = Some(id.vendor);
        back.product = Some(id.product);
    }
    drop(state);

    let device = gun.device.clone();
    let shared = Arc::clone(shared);
    thread::spawn(move || simulate_update(&shared, &gun, back, failure.as_deref(), step));
    Ok(json!({"device": device, "started": true}))
}

/// The events of an update: the gun out, the phases, the gun back.
fn simulate_update(
    shared: &Mutex<Shared>,
    gun: &Gun,
    back: Gun,
    failure: Option<&str>,
    step: Duration,
) {
    let progress = |phase, progress, error: Option<&str>| {
        thread::sleep(step);
        emit(
            shared,
            &Event::Firmware(FirmwareProgress {
                device: gun.device.clone(),
                phase,
                progress,
                error: error.map(str::to_owned),
            }),
        );
    };
    let was_waiting = lock(shared).firmware.waiting.take().is_some();
    if !was_waiting {
        let mut removed = gun.clone();
        removed.state = GunState::Stopped;
        emit(shared, &Event::GunRemoved(removed));
    }
    progress(FirmwarePhase::Bootloader, 0, None);
    for percent in (5..=60).step_by(5) {
        if percent == 40
            && let Some(error) = failure
        {
            // Over before its last event, as for the driver: a client may
            // retry as soon as it sees the error.
            {
                let mut state = lock(shared);
                state.firmware.waiting = Some(gun.clone());
                state.firmware.running = false;
            }
            progress(FirmwarePhase::Error, percent, Some(error));
            return;
        }
        progress(FirmwarePhase::Write, percent, None);
    }
    for percent in (65..=95).step_by(10) {
        progress(FirmwarePhase::Verify, percent, None);
    }
    progress(FirmwarePhase::Restart, 97, None);
    lock(shared).firmware.running = false;
    progress(FirmwarePhase::Done, 100, None);
    // The gun driven again: its firmware read once it is set up.
    let mut added = back.clone();
    added.state = GunState::Starting;
    added.firmware.clear();
    added.port.clear();
    emit(shared, &Event::GunAdded(added));
    thread::sleep(step);
    let mut aiming = back;
    aiming.state = GunState::Aiming;
    emit(shared, &Event::State(aiming));
}
