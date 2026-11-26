use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gun {
    /// The player (1 or 2).
    pub player: u8,
    /// The serial port, e.g. `/dev/ttyACM0` or `COM3`.
    #[serde(default)]
    pub port: String,
    /// The camera device.
    #[serde(default)]
    pub camera: String,
    /// The gun's firmware version.
    #[serde(default)]
    pub firmware: String,
    /// What the gun is doing.
    pub state: GunState,
    /// The last error, if any.
    #[serde(default)]
    pub error: Option<String>,
}

/// What a gun is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GunState {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event", content = "gun", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// A gun was plugged and is now driven.
    GunAdded(Gun),
    /// A gun was unplugged.
    GunRemoved(Gun),
    /// A gun's state changed.
    State(Gun),
    /// An event this client does not know (a newer driver), by name.
    #[serde(skip)]
    Other(String),
}

/// The wire form of an event, before its name is checked.
#[derive(Deserialize)]
pub(crate) struct RawEvent {
    pub event: String,
    #[serde(default)]
    pub gun: Option<Gun>,
}

impl TryFrom<RawEvent> for Event {
    type Error = crate::Error;

    fn try_from(raw: RawEvent) -> Result<Self, Self::Error> {
        let need_gun = |gun: Option<Gun>| {
            gun.ok_or_else(|| {
                crate::Error::Protocol(format!("event '{}' without a gun", raw.event))
            })
        };
        Ok(match raw.event.as_str() {
            "gun_added" => Self::GunAdded(need_gun(raw.gun)?),
            "gun_removed" => Self::GunRemoved(need_gun(raw.gun)?),
            "state" => Self::State(need_gun(raw.gun)?),
            _ => Self::Other(raw.event),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let line = r#"{"event":"state","gun":{"player":1,"port":"COM3","camera":"","firmware":"1.9","state":"calibrating","error":null}}"#;
        let raw: RawEvent = serde_json::from_str(line).unwrap();
        let event = Event::try_from(raw).unwrap();
        assert!(matches!(&event, Event::State(g) if g.state == GunState::Calibrating));
        assert_eq!(serde_json::to_string(&event).unwrap(), line);
    }

    #[test]
    fn unknown_event_is_kept_by_name() {
        let raw: RawEvent = serde_json::from_str(r#"{"event":"firmware_update"}"#).unwrap();
        assert_eq!(
            Event::try_from(raw).unwrap(),
            Event::Other("firmware_update".into())
        );
    }

    #[test]
    fn known_event_without_gun_is_an_error() {
        let raw: RawEvent = serde_json::from_str(r#"{"event":"gun_added"}"#).unwrap();
        assert!(matches!(
            Event::try_from(raw),
            Err(crate::Error::Protocol(_))
        ));
    }
}
