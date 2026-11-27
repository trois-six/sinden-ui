//! Firmware updates from the settings window.
//!
//! The window asks for a file ([`pick_firmware`]): it is read and checked
//! here (Intel HEX, the USB identifiers it presents), kept until the user
//! confirms ([`start_firmware_update`]), then sent to the driver with
//! `firmware_update`. The driver's `firmware` events reach the job through
//! the monitor's subscription ([`on_progress`]); every change is sent to
//! the window as [`FIRMWARE_EVENT`]. A failed update can be retried with
//! the same file ([`retry_firmware_update`]).

// Tauri hands commands their arguments by value.
#![allow(clippy::needless_pass_by_value)]

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use sinden_control::firmware::{FirmwareImage, MAX_FILE_SIZE};
use sinden_control::{
    Client, FirmwarePhase, FirmwareProgress, FirmwareTarget, Gun, GunColour, UsbId,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::AppState;
use crate::i18n::{self, Text};

/// The event the settings window listens to: the job, or `null`.
pub const FIRMWARE_EVENT: &str = "firmware-changed";

/// A pair of USB identifiers and the gun they make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IdInfo {
    pub vendor: u16,
    pub product: u16,
    pub colour: Option<GunColour>,
}

impl From<UsbId> for IdInfo {
    fn from(id: UsbId) -> Self {
        Self {
            vendor: id.vendor,
            product: id.product,
            colour: id.colour(),
        }
    }
}

/// The gun an update is for, as the status showed it when the file was
/// picked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GunRef {
    pub player: u8,
    pub device: String,
    /// Its identifiers, when the driver reports them.
    pub id: Option<IdInfo>,
    pub firmware: String,
}

impl From<&Gun> for GunRef {
    fn from(gun: &Gun) -> Self {
        Self {
            player: gun.player,
            device: gun.device.clone(),
            id: gun.usb_id().map(IdInfo::from),
            firmware: gun.firmware.clone(),
        }
    }
}

/// A file picked and checked, waiting for the user's confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFile {
    /// Names this file in [`start_firmware_update`].
    pub id: u64,
    pub file_name: String,
    /// The Sinden identifiers the file presents; none for a file that is
    /// not a Sinden gun's.
    pub ids: Vec<IdInfo>,
    pub gun: GunRef,
}

/// Why a file cannot be used; the window words it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum PickError {
    /// The gun is not in the status any more.
    NoGun,
    /// The file cannot be read.
    Read(String),
    /// Larger than [`MAX_FILE_SIZE`].
    TooLarge,
    /// Not text.
    NotText,
    /// Not Intel HEX, or not an image for the gun's chip.
    Hex(String),
}

/// An update sent to the driver, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub file_name: String,
    pub gun: GunRef,
    pub force: bool,
    /// The last `firmware` event; `None` until the first.
    pub progress: Option<FirmwareProgress>,
    #[serde(skip)]
    hex: String,
}

impl Job {
    /// Whether the update is still running (or being requested).
    pub fn running(&self) -> bool {
        !self.progress.as_ref().is_some_and(|p| p.phase.is_final())
    }

    fn request(&self) -> Request {
        Request {
            target: if self.gun.device.is_empty() {
                // An older driver: no device in its status.
                FirmwareTarget::Player(self.gun.player)
            } else {
                FirmwareTarget::Device(self.gun.device.clone())
            },
            hex: self.hex.clone(),
            force: self.force,
        }
    }
}

/// What a `firmware_update` request carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub target: FirmwareTarget,
    pub hex: String,
    pub force: bool,
}

/// The file picked and the update, shared by the commands and the monitor.
#[derive(Debug, Default)]
pub struct Firmware {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    next_id: u64,
    picked: Option<(FirmwareFile, String)>,
    job: Option<Job>,
}

impl Firmware {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Checks a file's text and keeps it for `gun` until confirmed.
    pub fn pick(
        &self,
        file_name: String,
        hex: String,
        gun: &Gun,
    ) -> Result<FirmwareFile, PickError> {
        let image = FirmwareImage::parse(&hex).map_err(|e| PickError::Hex(e.to_string()))?;
        let mut inner = self.lock();
        inner.next_id += 1;
        let file = FirmwareFile {
            id: inner.next_id,
            file_name,
            ids: image.usb_ids().into_iter().map(IdInfo::from).collect(),
            gun: GunRef::from(gun),
        };
        inner.picked = Some((file.clone(), hex));
        Ok(file)
    }

    /// Starts the job of the file `id`: what to send, and the job it
    /// replaces (to put back if the driver refuses).
    pub fn begin(&self, id: u64, force: bool) -> Result<(Request, Option<Job>), String> {
        let mut inner = self.lock();
        if inner.job.as_ref().is_some_and(Job::running) {
            return Err("a firmware update is already running".into());
        }
        let Some((file, hex)) = inner.picked.as_ref().filter(|(file, _)| file.id == id) else {
            return Err("this file is not the one picked: pick it again".into());
        };
        if file.ids.is_empty() {
            return Err("this file is not a Sinden gun's firmware".into());
        }
        let job = Job {
            file_name: file.file_name.clone(),
            gun: file.gun.clone(),
            force,
            progress: None,
            hex: hex.clone(),
        };
        let request = job.request();
        Ok((request, inner.job.replace(job)))
    }

    /// Starts the failed job again, with the same file and gun.
    pub fn retry(&self) -> Result<(Request, Option<Job>), String> {
        let mut inner = self.lock();
        let Some(job) = inner.job.as_mut().filter(|job| !job.running()) else {
            return Err("no failed firmware update to retry".into());
        };
        let previous = job.clone();
        job.progress = None;
        Ok((job.request(), Some(previous)))
    }

    /// The driver accepted the request: the update runs on `device`.
    pub fn started(&self, device: String) -> Option<Job> {
        let mut inner = self.lock();
        let job = inner.job.as_mut()?;
        job.gun.device = device;
        Some(job.clone())
    }

    /// The driver refused the request: the previous job is back.
    pub fn refused(&self, previous: Option<Job>) -> Option<Job> {
        let mut inner = self.lock();
        inner.job = previous;
        inner.job.clone()
    }

    /// A `firmware` event; the job if it changed.
    pub fn on_progress(&self, progress: &FirmwareProgress) -> Option<Job> {
        let mut inner = self.lock();
        let job = inner.job.as_mut()?;
        if job.gun.device != progress.device || job.progress.as_ref() == Some(progress) {
            return None;
        }
        job.progress = Some(progress.clone());
        Some(job.clone())
    }

    /// The subscription ended: a running job will get no more events.
    pub fn connection_lost(&self, error: &str) -> Option<Job> {
        let mut inner = self.lock();
        let job = inner.job.as_mut().filter(|job| job.running())?;
        job.progress = Some(FirmwareProgress {
            device: job.gun.device.clone(),
            phase: FirmwarePhase::Error,
            progress: job.progress.as_ref().map_or(0, |p| p.progress),
            error: Some(format!(
                "the connection to the driver was lost during the update ({error}); once it is back, retry the update"
            )),
        });
        Some(job.clone())
    }

    /// Forgets a job that is over; a running one is kept.
    pub fn dismiss(&self) -> bool {
        let mut inner = self.lock();
        if inner.job.as_ref().is_some_and(|job| !job.running()) {
            inner.job = None;
            return true;
        }
        false
    }

    pub fn job(&self) -> Option<Job> {
        self.lock().job.clone()
    }
}

/// Reads a text file of at most `limit` bytes.
pub fn read_text(path: &Path, limit: u64) -> Result<String, PickError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|e| PickError::Read(e.to_string()))?;
    if bytes.len() as u64 > limit {
        return Err(PickError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| PickError::NotText)
}

fn publish<R: Runtime>(app: &AppHandle<R>, job: Option<&Job>) {
    let _ = app.emit(FIRMWARE_EVENT, job);
    let state = app.state::<AppState>();
    crate::tray::update(app, &state.monitor.status());
}

/// A `firmware` event of the monitor's subscription.
pub fn on_progress<R: Runtime>(app: &AppHandle<R>, progress: &FirmwareProgress) {
    let state = app.state::<AppState>();
    if let Some(job) = state.firmware.on_progress(progress) {
        publish(app, Some(&job));
    }
}

/// The monitor lost the driver.
pub fn connection_lost<R: Runtime>(app: &AppHandle<R>, error: &str) {
    let state = app.state::<AppState>();
    if let Some(job) = state.firmware.connection_lost(error) {
        publish(app, Some(&job));
    }
}

/// Asks for a firmware file for `player`'s gun and checks it; `None` when
/// the user cancels.
#[tauri::command]
pub async fn pick_firmware<R: Runtime>(
    app: AppHandle<R>,
    window: WebviewWindow<R>,
    state: State<'_, AppState>,
    player: u8,
) -> Result<Option<FirmwareFile>, PickError> {
    let find_gun = |state: &AppState| {
        state
            .monitor
            .status()
            .guns
            .into_iter()
            .find(|gun| gun.player == player)
            .ok_or(PickError::NoGun)
    };
    find_gun(&state)?;
    let title = i18n::text(
        *state.lang.lock().unwrap_or_else(PoisonError::into_inner),
        Text::PickFirmware,
    );
    let dialog = app
        .dialog()
        .file()
        .set_title(title)
        .add_filter("Intel HEX", &["hex"])
        .set_parent(&window);
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = dialog.blocking_pick_file() else {
            return Ok(None);
        };
        let path = path
            .into_path()
            .map_err(|e| PickError::Read(e.to_string()))?;
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        read_text(&path, MAX_FILE_SIZE).map(|text| Some((name, text)))
    })
    .await
    .map_err(|e| PickError::Read(e.to_string()))??;
    let Some((name, text)) = picked else {
        return Ok(None);
    };
    // The gun as it is now: the dialog may have stayed open a while.
    let gun = find_gun(&state)?;
    state.firmware.pick(name, text, &gun).map(Some)
}

/// Sends the file `id` to its gun; `force` writes another colour's file.
#[tauri::command]
pub async fn start_firmware_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    id: u64,
    force: bool,
) -> Result<Job, String> {
    let begun = state.firmware.begin(id, force)?;
    send(&app, &state, begun).await
}

/// Starts the failed update again.
#[tauri::command]
pub async fn retry_firmware_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Job, String> {
    let begun = state.firmware.retry()?;
    send(&app, &state, begun).await
}

/// Forgets an update that is over.
#[tauri::command]
pub fn dismiss_firmware_update<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) {
    if state.firmware.dismiss() {
        publish(&app, None);
    }
}

#[tauri::command]
pub fn get_firmware_update(state: State<'_, AppState>) -> Option<Job> {
    state.firmware.job()
}

async fn send<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    (request, previous): (Request, Option<Job>),
) -> Result<Job, String> {
    publish(app, state.firmware.job().as_ref());
    let endpoint = state.monitor.endpoint().clone();
    let sent = tauri::async_runtime::spawn_blocking(move || {
        Client::connect(&endpoint)?.firmware_update(&request.target, &request.hex, request.force)
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|result| result.map_err(|e| e.to_string()));
    match sent {
        Ok(started) => {
            let job = state.firmware.started(started.device);
            publish(app, job.as_ref());
            job.ok_or_else(|| "the update was forgotten".to_owned())
        }
        Err(error) => {
            let job = state.firmware.refused(previous);
            publish(app, job.as_ref());
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use sinden_control::GunState;

    use super::*;

    fn record(address: u16, kind: u8, data: &[u8]) -> String {
        let [high, low] = address.to_be_bytes();
        let mut bytes = vec![u8::try_from(data.len()).unwrap(), high, low, kind];
        bytes.extend_from_slice(data);
        let sum = bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b));
        bytes.push(sum.wrapping_neg());
        let digits: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
        format!(":{}\n", digits.concat())
    }

    fn file_for(product: Option<u16>) -> String {
        let mut data = vec![0x0c, 0x94, 0x12, 0x01];
        if let Some(product) = product {
            data.extend([0xc0, 0x16]);
            data.extend(product.to_le_bytes());
        }
        record(0, 0x00, &data) + &record(0, 0x01, &[])
    }

    fn gun() -> Gun {
        Gun {
            player: 1,
            device: "/sys/devices/x/1-1.2".into(),
            vendor: Some(0x16c0),
            product: Some(0x0f01),
            port: "/dev/ttyACM0".into(),
            camera: "/dev/video0".into(),
            firmware: "1.9".into(),
            state: GunState::Aiming,
            error: None,
        }
    }

    fn progress(phase: FirmwarePhase, percent: u8) -> FirmwareProgress {
        FirmwareProgress {
            device: gun().device,
            phase,
            progress: percent,
            error: (phase == FirmwarePhase::Error).then(|| "page 3 read back wrong".to_owned()),
        }
    }

    #[test]
    fn a_picked_file_says_which_gun_it_is_for() {
        let firmware = Firmware::default();
        let file = firmware
            .pick(
                "LightgunFirmwareRed.hex".into(),
                file_for(Some(0x0f02)),
                &gun(),
            )
            .unwrap();
        assert_eq!(file.file_name, "LightgunFirmwareRed.hex");
        assert_eq!(
            file.ids,
            [IdInfo {
                vendor: 0x16c0,
                product: 0x0f02,
                colour: Some(GunColour::Red)
            }]
        );
        assert_eq!(file.gun.id.unwrap().colour, Some(GunColour::Blue));
        assert_eq!(file.gun.firmware, "1.9");
        let json = serde_json::to_value(&file).unwrap();
        assert_eq!(json["fileName"], "LightgunFirmwareRed.hex");
        assert_eq!(json["ids"][0]["colour"], "red");
        assert_eq!(json["gun"]["id"]["product"], 0x0f01);
    }

    #[test]
    fn bad_files_are_refused_before_anything_is_sent() {
        let firmware = Firmware::default();
        let error = firmware
            .pick("x.hex".into(), "hello".into(), &gun())
            .unwrap_err();
        assert_eq!(
            error,
            PickError::Hex("line 1: not an Intel HEX record".into())
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({"kind": "hex", "detail": "line 1: not an Intel HEX record"})
        );
        assert_eq!(
            serde_json::to_value(PickError::TooLarge).unwrap(),
            serde_json::json!({"kind": "too_large"})
        );
        // A GunCon file: picked (the window says why it cannot be used),
        // never started.
        let guncon = firmware
            .pick("LightgunFirmwareGunCon.hex".into(), file_for(None), &gun())
            .unwrap();
        assert_eq!(guncon.ids, []);
        assert!(firmware.begin(guncon.id, true).is_err());
        assert_eq!(firmware.job(), None);
    }

    #[test]
    fn an_update_from_start_to_done() {
        let firmware = Firmware::default();
        let hex = file_for(Some(0x0f01));
        let file = firmware
            .pick("Blue.hex".into(), hex.clone(), &gun())
            .unwrap();
        assert!(firmware.begin(file.id + 1, false).is_err(), "another file");
        let (request, previous) = firmware.begin(file.id, false).unwrap();
        assert_eq!(
            request,
            Request {
                target: FirmwareTarget::Device("/sys/devices/x/1-1.2".into()),
                hex,
                force: false
            }
        );
        assert_eq!(previous, None);
        assert!(firmware.job().unwrap().running());
        assert!(firmware.begin(file.id, false).is_err(), "one at a time");
        assert!(!firmware.dismiss(), "a running job stays");

        firmware.started("/sys/devices/x/1-1.2".into()).unwrap();
        let mut other = progress(FirmwarePhase::Write, 10);
        other.device = "/sys/devices/x/1-1.3".into();
        assert_eq!(firmware.on_progress(&other), None, "another gun's update");
        let job = firmware
            .on_progress(&progress(FirmwarePhase::Write, 30))
            .unwrap();
        assert_eq!(job.progress.unwrap().progress, 30);
        assert_eq!(
            firmware.on_progress(&progress(FirmwarePhase::Write, 30)),
            None,
            "no change"
        );
        let done = firmware
            .on_progress(&progress(FirmwarePhase::Done, 100))
            .unwrap();
        assert!(!done.running());
        let json = serde_json::to_value(&done).unwrap();
        assert_eq!(json["progress"]["phase"], "done");
        assert!(json.get("hex").is_none(), "the file stays on the Rust side");
        assert!(firmware.dismiss());
        assert_eq!(firmware.job(), None);
    }

    #[test]
    fn a_failed_update_is_retried_with_the_same_file() {
        let firmware = Firmware::default();
        let file = firmware
            .pick("Red.hex".into(), file_for(Some(0x0f02)), &gun())
            .unwrap();
        assert!(firmware.retry().is_err(), "nothing to retry");
        let (first, _) = firmware.begin(file.id, true).unwrap();
        firmware.on_progress(&progress(FirmwarePhase::Bootloader, 0));
        let failed = firmware
            .on_progress(&progress(FirmwarePhase::Error, 41))
            .unwrap();
        assert!(!failed.running());

        let (again, previous) = firmware.retry().unwrap();
        assert_eq!(again, first, "same gun, file and force");
        assert_eq!(previous.as_ref(), Some(&failed));
        assert_eq!(firmware.job().unwrap().progress, None);
        // The driver refuses the retry: the failure is shown again.
        assert_eq!(firmware.refused(previous), Some(failed));
    }

    #[test]
    fn a_refused_start_puts_the_previous_job_back() {
        let firmware = Firmware::default();
        let file = firmware
            .pick("Blue.hex".into(), file_for(Some(0x0f01)), &gun())
            .unwrap();
        let (_, previous) = firmware.begin(file.id, false).unwrap();
        assert_eq!(firmware.refused(previous), None);
        assert_eq!(firmware.job(), None);
        // The file is still there to try again.
        assert!(firmware.begin(file.id, false).is_ok());
    }

    #[test]
    fn an_older_driver_is_asked_by_player() {
        let firmware = Firmware::default();
        let mut older = gun();
        older.device.clear();
        older.vendor = None;
        let file = firmware
            .pick("Blue.hex".into(), file_for(Some(0x0f01)), &older)
            .unwrap();
        assert_eq!(file.gun.id, None);
        let (request, _) = firmware.begin(file.id, false).unwrap();
        assert_eq!(request.target, FirmwareTarget::Player(1));
        firmware.started("COM5".into());
        assert!(
            firmware
                .on_progress(&FirmwareProgress {
                    device: "COM5".into(),
                    ..progress(FirmwarePhase::Write, 5)
                })
                .is_some()
        );
    }

    #[test]
    fn losing_the_driver_ends_a_running_update() {
        let firmware = Firmware::default();
        assert_eq!(firmware.connection_lost("closed"), None);
        let file = firmware
            .pick("Blue.hex".into(), file_for(Some(0x0f01)), &gun())
            .unwrap();
        firmware.begin(file.id, false).unwrap();
        firmware.on_progress(&progress(FirmwarePhase::Verify, 70));
        let lost = firmware
            .connection_lost("the driver closed the connection")
            .unwrap();
        let progress = lost.progress.unwrap();
        assert_eq!(
            (progress.phase, progress.progress),
            (FirmwarePhase::Error, 70)
        );
        assert!(progress.error.unwrap().contains("retry"));
        assert!(firmware.retry().is_ok());
    }

    #[test]
    fn the_tray_line_follows_the_job() {
        use crate::i18n::{Lang, firmware_line};

        let firmware = Firmware::default();
        let file = firmware
            .pick("Blue.hex".into(), file_for(Some(0x0f01)), &gun())
            .unwrap();
        firmware.begin(file.id, false).unwrap();
        let line = |lang| firmware_line(lang, &firmware.job().unwrap());
        assert_eq!(
            line(Lang::En).as_deref(),
            Some("Player 1: updating the firmware (0 %)")
        );
        firmware.on_progress(&progress(FirmwarePhase::Write, 45));
        assert_eq!(
            line(Lang::Fr).as_deref(),
            Some("Joueur 1 : mise à jour du firmware (45 %)")
        );
        firmware.on_progress(&progress(FirmwarePhase::Error, 50));
        assert_eq!(
            line(Lang::En).as_deref(),
            Some("Player 1: firmware update failed")
        );
        firmware.retry().unwrap();
        firmware.on_progress(&progress(FirmwarePhase::Done, 100));
        assert_eq!(line(Lang::En), None, "the gun's own line says the rest");
    }

    #[test]
    fn files_are_read_up_to_a_limit() {
        // Under the build's own directory, not the system's.
        let dir = Path::new(env!("OUT_DIR")).join(format!("firmware-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.hex");
        std::fs::write(&path, "0123456789").unwrap();
        assert_eq!(read_text(&path, 10).unwrap(), "0123456789");
        assert_eq!(read_text(&path, 9), Err(PickError::TooLarge));
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(read_text(&path, 10), Err(PickError::NotText));
        assert!(matches!(
            read_text(&dir.join("missing.hex"), 10),
            Err(PickError::Read(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
