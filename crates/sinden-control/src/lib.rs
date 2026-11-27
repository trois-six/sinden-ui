//! Client of the `sinden-driver` control API.
//!
//! The driver service listens on a local endpoint (a Unix socket on Linux
//! and macOS, a named pipe on Windows; see [`Endpoint`]). The protocol is
//! one JSON object per line, UTF-8, request then response:
//!
//! ```text
//! -> {"id": 1, "method": "status", "params": {}}
//! <- {"id": 1, "result": {"guns": [...]}}
//! <- {"id": 2, "error": "..."}
//! ```
//!
//! `subscribe` answers with the current status (`{"guns": [...]}`); after
//! it, that connection only carries events, one per line, without `id`:
//! `{"event": "gun_added" | "gun_removed" | "state", "gun": {...}}` (the
//! driver coalesces them into differences between snapshots), and a
//! firmware update's `{"event": "firmware", "device": ..., "phase": ...,
//! "progress": ..., "error": ...}`. Other requests go over another
//! connection.
//!
//! `firmware_update` sends a firmware file of Sinden's to a gun; the
//! [`firmware`] module checks the file and tells which gun it is for
//! first.
//!
//! ```no_run
//! use sinden_control::{Client, Endpoint};
//!
//! let mut client = Client::connect(&Endpoint::from_env_or_default())?;
//! for gun in client.status()?.guns {
//!     println!("player {}: {:?}", gun.player, gun.state);
//! }
//! # Ok::<(), sinden_control::Error>(())
//! ```

mod client;
mod endpoint;
mod error;
#[cfg(unix)]
pub mod fake;
pub mod firmware;
mod guns;
mod types;

pub use client::{Client, MAX_REQUEST_LINE, Subscription};
pub use endpoint::{ENDPOINT_ENV, Endpoint};
pub use error::Error;
pub use firmware::{GunColour, UsbId};
pub use guns::Guns;
pub use types::{
    ConfigSnapshot, Event, FirmwarePhase, FirmwareProgress, FirmwareStarted, FirmwareTarget, Gun,
    GunState, Status, VersionInfo,
};

/// The control API version this client speaks.
pub const API_VERSION: u32 = 1;

/// A specialised `Result` for the control API.
pub type Result<T, E = Error> = std::result::Result<T, E>;
