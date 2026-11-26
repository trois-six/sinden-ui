//! A fake driver serving the control API on a Unix socket, for tests and
//! for developing the UI without a gun or the driver.
//!
//! It keeps the configuration in memory and answers `version`, `status`,
//! `get_config`, `set_config` and `subscribe` as the driver would.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use serde_json::{Value, json};

use crate::{ConfigSnapshot, Event, Gun, Guns, Status};

#[derive(Debug, Default)]
struct Shared {
    config: BTreeMap<String, String>,
    guns: Guns,
    subscribers: Vec<UnixStream>,
    set_config_calls: Vec<BTreeMap<String, String>>,
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
        let mut shared = lock(&self.shared);
        shared.guns.apply(event);
        let mut line = serde_json::to_vec(event).expect("events serialise");
        line.push(b'\n');
        shared
            .subscribers
            .retain_mut(|s| s.write_all(&line).is_ok());
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

fn serve(stream: &UnixStream, shared: &Mutex<Shared>) {
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
