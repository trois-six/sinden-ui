use std::collections::BTreeMap;

use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::firmware::UsbId;

/// Result of `version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    /// The driver's version, e.g. `0.2.0`.
    pub driver: String,
    /// The control API version, see [`crate::API_VERSION`].
    pub api: u32,
}

/// Result of `status`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The guns the driver drives.
    pub guns: Vec<Gun>,
}

/// One gun, as `status` and the events describe it.
///
/// Fields the driver reports as `null` (not known yet) are empty strings
/// or `None`; fields it does not send (an older driver) take the same
/// values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gun {
    /// The player (1 or 2).
    pub player: u8,
    /// The gun's USB device, stable while it stays in the same USB port
    /// (a sysfs directory on Linux, a device instance ID on Windows): what
    /// a firmware update names the gun by.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub device: String,
    /// The gun's USB vendor ID, when known.
    #[serde(default)]
    pub vendor: Option<u16>,
    /// The gun's USB product ID, when known: its colour.
    #[serde(default)]
    pub product: Option<u16>,
    /// The serial port, e.g. `/dev/ttyACM0` or `COM3`.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub port: String,
    /// The camera device.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub camera: String,
    /// The gun's firmware version, e.g. `2.1`.
    #[serde(default, deserialize_with = "null_as_empty")]
    pub firmware: String,
    /// What the gun is doing.
    pub state: GunState,
    /// The last error, if any.
    #[serde(default)]
    pub error: Option<String>,
}

impl Gun {
    /// The gun's USB identifiers, when the driver knows both.
    #[must_use]
    pub fn usb_id(&self) -> Option<UsbId> {
        Some(UsbId {
            vendor: self.vendor?,
            product: self.product?,
        })
    }
}

/// A string the driver may send as `null` (not known yet).
fn null_as_empty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

/// What a gun is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GunState {
    /// Opening the gun's port and camera.
    Starting,
    /// Aiming at the screen.
    Aiming,
    /// Aimed away from the screen.
    AimedAway,
    /// In calibration mode.
    Calibrating,
    /// Stopped (error or shutting down).
    Stopped,
    /// A state this client does not know (a newer driver).
    #[serde(other)]
    Unknown,
}

/// Result of `get_config` and `set_config`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// The configuration file.
    pub path: String,
    /// Every key, defaults included.
    pub values: BTreeMap<String, String>,
}

/// An event of a `subscribe` connection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// A gun was plugged and is now driven.
    GunAdded(Gun),
    /// A gun was unplugged (or taken for a firmware update).
    GunRemoved(Gun),
    /// A gun's state changed.
    State(Gun),
    /// A firmware update's phase or progress changed.
    Firmware(FirmwareProgress),
    /// An event this client does not know (a newer driver), by name.
    Other(String),
}

impl Event {
    /// Decodes an event line (a JSON object with `event`).
    pub(crate) fn from_message(message: Map<String, Value>) -> crate::Result<Self> {
        let name = match message.get("event") {
            Some(Value::String(name)) => name.clone(),
            other => {
                return Err(crate::Error::Protocol(format!(
                    "an event's name must be a string, got {}",
                    other.map_or_else(|| "none".to_owned(), Value::to_string)
                )));
            }
        };
        let gun = |message: Map<String, Value>| -> crate::Result<Gun> {
            match message.get("gun") {
                None | Some(Value::Null) => Err(crate::Error::Protocol(format!(
                    "event '{name}' without a gun"
                ))),
                Some(gun) => Ok(Gun::deserialize(gun)?),
            }
        };
        Ok(match name.as_str() {
            "gun_added" => Self::GunAdded(gun(message)?),
            "gun_removed" => Self::GunRemoved(gun(message)?),
            "state" => Self::State(gun(message)?),
            "firmware" => Self::Firmware(FirmwareProgress::deserialize(Value::Object(message))?),
            _ => Self::Other(name),
        })
    }
}

/// The wire form: `{"event": "state", "gun": {...}}` for a gun's events,
/// `{"event": "firmware", "device": ..., "phase": ..., ...}` for a
/// firmware update's. [`Event::Other`] cannot be serialised.
impl Serialize for Event {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct GunEvent<'a> {
            event: &'static str,
            gun: &'a Gun,
        }
        #[derive(Serialize)]
        struct FirmwareEvent<'a> {
            event: &'static str,
            #[serde(flatten)]
            progress: &'a FirmwareProgress,
        }
        let (event, gun) = match self {
            Self::GunAdded(gun) => ("gun_added", gun),
            Self::GunRemoved(gun) => ("gun_removed", gun),
            Self::State(gun) => ("state", gun),
            Self::Firmware(progress) => {
                return FirmwareEvent {
                    event: "firmware",
                    progress,
                }
                .serialize(serializer);
            }
            Self::Other(name) => {
                return Err(S::Error::custom(format!(
                    "the unknown event '{name}' cannot be serialised"
                )));
            }
        };
        GunEvent { event, gun }.serialize(serializer)
    }
}

/// A `firmware` event: where a gun's firmware update is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareProgress {
    /// The gun's USB device (its `device` in the status).
    pub device: String,
    /// The phase of the update.
    pub phase: FirmwarePhase,
    /// 0 to 100.
    #[serde(default)]
    pub progress: u8,
    /// Why the update failed, with what the gun was left with
    /// (phase [`FirmwarePhase::Error`]).
    #[serde(default)]
    pub error: Option<String>,
}

/// The phases of a firmware update, in order; the last is `Done` or
/// `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FirmwarePhase {
    /// The gun reset into its bootloader.
    Bootloader,
    /// The pages being written.
    Write,
    /// The pages being read back.
    Verify,
    /// The bootloader left, the gun awaited.
    Restart,
    /// The gun is back with its new firmware.
    Done,
    /// The update failed.
    Error,
    /// A phase this client does not know (a newer driver).
    #[serde(other)]
    Unknown,
}

impl FirmwarePhase {
    /// Whether the update is over (`Done` or `Error`).
    #[must_use]
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Error)
    }
}

/// Which gun a firmware update is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareTarget {
    /// A gun's `device`, as the status gives it; also reaches a gun a
    /// failed update left waiting in its bootloader.
    Device(String),
    /// A player (1 or 2).
    Player(u8),
}

/// Result of `firmware_update`: the update started on that device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareStarted {
    /// The gun's USB device, which the `firmware` events name.
    pub device: String,
    /// Always `true` when the driver accepted the request.
    #[serde(default)]
    pub started: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(line: &str) -> crate::Result<Event> {
        Event::from_message(serde_json::from_str(line).unwrap())
    }

    #[test]
    fn gun_from_the_roadmap_example() {
        let gun: Gun = serde_json::from_str(
            r#"{"player": 1, "port": "/dev/ttyACM0", "camera": "/dev/video0",
                "firmware": "1.9", "state": "aimed_away", "error": null}"#,
        )
        .unwrap();
        assert_eq!(gun.player, 1);
        assert_eq!(gun.state, GunState::AimedAway);
        assert_eq!(gun.error, None);
    }

    #[test]
    fn unknown_state_and_missing_optional_fields() {
        let gun: Gun = serde_json::from_str(r#"{"player": 2, "state": "dreaming"}"#).unwrap();
        assert_eq!(gun.state, GunState::Unknown);
        assert_eq!(gun.port, "");
    }

    #[test]
    fn event_round_trip() {
        let line = r#"{"event":"state","gun":{"player":1,"device":"","vendor":null,"product":null,"port":"COM3","camera":"","firmware":"1.9","state":"calibrating","error":null}}"#;
        let event = event(line).unwrap();
        assert!(matches!(&event, Event::State(g) if g.state == GunState::Calibrating));
        assert_eq!(serde_json::to_string(&event).unwrap(), line);
    }

    #[test]
    fn gun_of_a_driver_with_usb_ids_and_nulls() {
        // A gun just added: its port, camera and firmware not known yet.
        let gun: Gun = serde_json::from_str(
            r#"{"player":1,"device":"/sys/devices/platform/soc/usb1/1-1/1-1.2","vendor":5824,
                "product":3896,"mode":"joystick","port":null,"camera":null,"firmware":null,
                "state":"starting","error":null}"#,
        )
        .unwrap();
        assert_eq!(gun.device, "/sys/devices/platform/soc/usb1/1-1/1-1.2");
        assert_eq!(
            gun.usb_id(),
            Some(UsbId {
                vendor: 0x16c0,
                product: 0x0f38
            })
        );
        assert_eq!((gun.port.as_str(), gun.firmware.as_str()), ("", ""));
        let older: Gun = serde_json::from_str(r#"{"player":2,"state":"aiming"}"#).unwrap();
        assert_eq!((older.device.as_str(), older.usb_id()), ("", None));
    }

    #[test]
    fn firmware_events() {
        let line = r#"{"event":"firmware","device":"/sys/devices/x/1-1.2","phase":"write","progress":32,"error":null}"#;
        let write = event(line).unwrap();
        assert_eq!(
            write,
            Event::Firmware(FirmwareProgress {
                device: "/sys/devices/x/1-1.2".into(),
                phase: FirmwarePhase::Write,
                progress: 32,
                error: None,
            })
        );
        assert_eq!(serde_json::to_string(&write).unwrap(), line);

        let failed = event(
            r#"{"event":"firmware","device":"d","phase":"error","progress":41,"error":"page 3 read back wrong"}"#,
        )
        .unwrap();
        assert!(matches!(&failed, Event::Firmware(p)
            if p.phase == FirmwarePhase::Error && p.phase.is_final()
                && p.error.as_deref() == Some("page 3 read back wrong")));

        let newer = event(r#"{"event":"firmware","device":"d","phase":"erase"}"#).unwrap();
        assert!(matches!(&newer, Event::Firmware(p)
            if p.phase == FirmwarePhase::Unknown && p.progress == 0 && !p.phase.is_final()));
        assert!(FirmwarePhase::Done.is_final());
        assert!(!FirmwarePhase::Restart.is_final());
    }

    #[test]
    fn a_firmware_event_without_its_fields_is_an_error() {
        assert!(matches!(
            event(r#"{"event":"firmware","progress":3}"#),
            Err(crate::Error::Json(_))
        ));
    }

    #[test]
    fn unknown_events_cannot_be_serialised() {
        assert!(serde_json::to_string(&Event::Other("x".into())).is_err());
    }

    #[test]
    fn unknown_event_is_kept_by_name() {
        assert_eq!(
            event(r#"{"event":"firmware_update"}"#).unwrap(),
            Event::Other("firmware_update".into())
        );
    }

    #[test]
    fn known_event_without_gun_is_an_error() {
        assert!(matches!(
            event(r#"{"event":"gun_added"}"#),
            Err(crate::Error::Protocol(_))
        ));
        assert!(matches!(
            event(r#"{"event":"state","gun":null}"#),
            Err(crate::Error::Protocol(_))
        ));
        assert!(matches!(
            event(r#"{"event":7}"#),
            Err(crate::Error::Protocol(_))
        ));
    }
}
