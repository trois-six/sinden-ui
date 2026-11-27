use serde::Serialize;

use crate::{Event, Gun, Status};

/// The guns the driver drives, kept up to date from `status` and the
/// events of a subscription. Sorted by player.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Guns(Vec<Gun>);

impl Guns {
    /// The guns of a `status` result.
    #[must_use]
    pub fn from_status(status: Status) -> Self {
        let mut guns = Self(status.guns);
        guns.0.sort_by_key(|gun| gun.player);
        guns
    }

    /// Applies an event; returns whether anything changed.
    pub fn apply(&mut self, event: &Event) -> bool {
        match event {
            Event::GunAdded(gun) | Event::State(gun) => self.upsert(gun),
            Event::GunRemoved(gun) => {
                let before = self.0.len();
                self.0.retain(|g| g.player != gun.player);
                self.0.len() != before
            }
            // A firmware update shows through the gun's own events.
            Event::Firmware(_) | Event::Other(_) => false,
        }
    }

    /// The guns, sorted by player.
    #[must_use]
    pub fn as_slice(&self) -> &[Gun] {
        &self.0
    }

    fn upsert(&mut self, gun: &Gun) -> bool {
        match self.0.binary_search_by_key(&gun.player, |g| g.player) {
            Ok(index) if self.0[index] == *gun => false,
            Ok(index) => {
                self.0[index] = gun.clone();
                true
            }
            Err(index) => {
                self.0.insert(index, gun.clone());
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GunState;

    fn gun(player: u8, state: GunState) -> Gun {
        Gun {
            player,
            device: format!("/sys/devices/x/1-1.{player}"),
            vendor: Some(0x16c0),
            product: Some(0x0f01),
            port: format!("/dev/ttyACM{player}"),
            camera: String::new(),
            firmware: "1.9".into(),
            state,
            error: None,
        }
    }

    #[test]
    fn status_is_sorted() {
        let guns = Guns::from_status(Status {
            guns: vec![gun(2, GunState::Aiming), gun(1, GunState::Aiming)],
        });
        let players: Vec<u8> = guns.as_slice().iter().map(|g| g.player).collect();
        assert_eq!(players, [1, 2]);
    }

    #[test]
    fn add_update_remove() {
        let mut guns = Guns::default();
        assert!(guns.apply(&Event::GunAdded(gun(2, GunState::Aiming))));
        assert!(guns.apply(&Event::GunAdded(gun(1, GunState::Aiming))));
        assert!(
            !guns.apply(&Event::State(gun(1, GunState::Aiming))),
            "no change"
        );
        assert!(guns.apply(&Event::State(gun(1, GunState::Calibrating))));
        assert_eq!(guns.as_slice()[0].state, GunState::Calibrating);
        assert!(guns.apply(&Event::GunRemoved(gun(1, GunState::Stopped))));
        assert!(!guns.apply(&Event::GunRemoved(gun(1, GunState::Stopped))));
        assert!(!guns.apply(&Event::Other("x".into())));
        assert_eq!(guns.as_slice().len(), 1);
        assert_eq!(guns.as_slice()[0].player, 2);
    }
}
