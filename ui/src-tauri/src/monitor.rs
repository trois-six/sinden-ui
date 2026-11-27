//! Follows the driver: reads `version`, then keeps the guns up to date
//! from a `subscribe` connection (its first answer is the status). When
//! the driver is not reachable it retries every few seconds.

use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use sinden_control::{API_VERSION, Client, Endpoint, Error, Event, Gun, Guns};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// The event the settings window listens to.
pub const STATUS_EVENT: &str = "status-changed";

const RETRY: Duration = Duration::from_secs(3);

/// What the UI knows about the service.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub connected: bool,
    pub driver_version: Option<String>,
    pub api: Option<u32>,
    pub api_supported: u32,
    pub guns: Vec<Gun>,
    pub error: Option<String>,
    pub endpoint: String,
}

impl ServiceStatus {
    fn disconnected(endpoint: &Endpoint, error: Option<String>) -> Self {
        Self {
            connected: false,
            driver_version: None,
            api: None,
            api_supported: API_VERSION,
            guns: Vec::new(),
            error,
            endpoint: endpoint.to_string(),
        }
    }
}

/// The endpoint and the last known status.
#[derive(Debug)]
pub struct Monitor {
    endpoint: Endpoint,
    status: Mutex<ServiceStatus>,
}

impl Default for Monitor {
    fn default() -> Self {
        let endpoint = Endpoint::from_env_or_default();
        let status = Mutex::new(ServiceStatus::disconnected(&endpoint, None));
        Self { endpoint, status }
    }
}

impl Monitor {
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn status(&self) -> ServiceStatus {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set(&self, status: &ServiceStatus) -> bool {
        let mut current = self.status.lock().unwrap_or_else(PoisonError::into_inner);
        let changed = current.connected != status.connected
            || current.guns != status.guns
            || current.driver_version != status.driver_version
            || current.error != status.error;
        *current = status.clone();
        changed
    }
}

/// Starts following the driver on a background thread.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    thread::Builder::new()
        .name("driver-monitor".into())
        .spawn(move || {
            loop {
                let error = session(&app).err();
                crate::firmware::connection_lost(
                    &app,
                    &error
                        .as_ref()
                        .map_or_else(|| Error::Closed.to_string(), ToString::to_string),
                );
                let endpoint = app.state::<crate::AppState>().monitor.endpoint().clone();
                publish(
                    &app,
                    &ServiceStatus::disconnected(&endpoint, error.map(|e| e.to_string())),
                );
                thread::sleep(RETRY);
            }
        })
        .expect("cannot start the monitor thread");
}

/// One connection to the driver, until it fails or closes.
fn session<R: Runtime>(app: &AppHandle<R>) -> Result<(), Error> {
    let endpoint = app.state::<crate::AppState>().monitor.endpoint().clone();
    let version = Client::connect(&endpoint)?.version()?;
    // `subscribe` answers with the current status, then only sends events
    // on its connection.
    let (status, mut events) = Client::connect_with_timeout(&endpoint, None)?.subscribe()?;
    let mut guns = Guns::from_status(status);

    let status = |guns: &Guns| ServiceStatus {
        connected: true,
        driver_version: Some(version.driver.clone()),
        api: Some(version.api),
        api_supported: API_VERSION,
        guns: guns.as_slice().to_vec(),
        error: None,
        endpoint: endpoint.to_string(),
    };
    publish(app, &status(&guns));
    loop {
        let event = events.next_event()?;
        if let Event::Firmware(progress) = &event {
            crate::firmware::on_progress(app, progress);
        }
        if guns.apply(&event) {
            publish(app, &status(&guns));
        }
    }
}

fn publish<R: Runtime>(app: &AppHandle<R>, status: &ServiceStatus) {
    let state = app.state::<crate::AppState>();
    if state.monitor.set(status) {
        crate::tray::update(app, status);
        let _ = app.emit(STATUS_EVENT, status);
    }
}
