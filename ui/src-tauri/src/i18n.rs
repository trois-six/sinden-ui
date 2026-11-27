//! The tray's strings in English and French (the window has its own, in
//! ui/src/i18n.ts).

use sinden_control::{FirmwarePhase, Gun, GunState};

use crate::firmware::Job;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
}

impl Lang {
    /// From a BCP 47 tag such as `fr-FR`; English unless French.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag.split(['-', '_']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("fr") {
            Self::Fr
        } else {
            Self::En
        }
    }

    /// The system's language.
    pub fn from_system() -> Self {
        sys_locale::get_locale().map_or(Self::En, |tag| Self::from_tag(&tag))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Text {
    WindowTitle,
    OpenSettings,
    Quit,
    NoGun,
    Unreachable,
    PickFirmware,
}

pub fn text(lang: Lang, text: Text) -> &'static str {
    match (lang, text) {
        (_, Text::WindowTitle) => "Sinden Driver",
        (Lang::En, Text::OpenSettings) => "Open settings…",
        (Lang::Fr, Text::OpenSettings) => "Ouvrir les réglages…",
        (Lang::En, Text::Quit) => "Quit",
        (Lang::Fr, Text::Quit) => "Quitter",
        (Lang::En, Text::NoGun) => "No gun detected",
        (Lang::Fr, Text::NoGun) => "Aucun pistolet détecté",
        (Lang::En, Text::Unreachable) => "Driver not reachable",
        (Lang::Fr, Text::Unreachable) => "Pilote injoignable",
        (Lang::En, Text::PickFirmware) => "Firmware file (Intel HEX)",
        (Lang::Fr, Text::PickFirmware) => "Fichier de firmware (Intel HEX)",
    }
}

pub fn state_name(lang: Lang, state: GunState) -> &'static str {
    match (lang, state) {
        (Lang::En, GunState::Starting) => "starting",
        (Lang::Fr, GunState::Starting) => "démarrage",
        (Lang::En, GunState::Aiming) => "aiming",
        (Lang::Fr, GunState::Aiming) => "vise l'écran",
        (Lang::En, GunState::AimedAway) => "aimed away",
        (Lang::Fr, GunState::AimedAway) => "hors écran",
        (Lang::En, GunState::Calibrating) => "calibrating",
        (Lang::Fr, GunState::Calibrating) => "en calibration",
        (Lang::En, GunState::Stopped) => "stopped",
        (Lang::Fr, GunState::Stopped) => "arrêté",
        (Lang::En, _) => "unknown",
        (Lang::Fr, _) => "inconnu",
    }
}

/// One gun's line in the tray menu, e.g. `Player 1: aiming (COM3)`.
pub fn gun_line(lang: Lang, gun: &Gun) -> String {
    let (player, colon) = match lang {
        Lang::En => ("Player", ":"),
        Lang::Fr => ("Joueur", " :"),
    };
    let mut line = format!(
        "{player} {}{colon} {}",
        gun.player,
        state_name(lang, gun.state)
    );
    if !gun.port.is_empty() {
        line.push_str(" (");
        line.push_str(&gun.port);
        line.push(')');
    }
    if let Some(error) = &gun.error {
        line.push_str(" — ");
        line.push_str(error);
    }
    line
}

/// A firmware update's line in the tray menu while it runs or after it
/// failed, e.g. `Player 1: updating the firmware (45 %)`.
pub fn firmware_line(lang: Lang, job: &Job) -> Option<String> {
    let player = job.gun.player;
    let phase = job.progress.as_ref().map(|p| (p.phase, p.progress));
    Some(match (lang, phase) {
        (_, Some((FirmwarePhase::Done, _))) => return None,
        (Lang::En, Some((FirmwarePhase::Error, _))) => {
            format!("Player {player}: firmware update failed")
        }
        (Lang::Fr, Some((FirmwarePhase::Error, _))) => {
            format!("Joueur {player} : échec de la mise à jour du firmware")
        }
        (Lang::En, progress) => format!(
            "Player {player}: updating the firmware ({} %)",
            progress.map_or(0, |(_, p)| p)
        ),
        (Lang::Fr, progress) => format!(
            "Joueur {player} : mise à jour du firmware ({} %)",
            progress.map_or(0, |(_, p)| p)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gun(state: GunState) -> Gun {
        Gun {
            player: 1,
            device: r"USB\VID_16C0&PID_0F01\1".into(),
            vendor: Some(0x16c0),
            product: Some(0x0f01),
            port: "COM3".into(),
            camera: String::new(),
            firmware: "1.9".into(),
            state,
            error: None,
        }
    }

    #[test]
    fn languages() {
        assert_eq!(Lang::from_tag("fr-CA"), Lang::Fr);
        assert_eq!(Lang::from_tag("FR"), Lang::Fr);
        assert_eq!(Lang::from_tag("en_GB"), Lang::En);
        assert_eq!(Lang::from_tag("de"), Lang::En);
    }

    #[test]
    fn gun_lines() {
        assert_eq!(
            gun_line(Lang::En, &gun(GunState::Aiming)),
            "Player 1: aiming (COM3)"
        );
        assert_eq!(
            gun_line(Lang::Fr, &gun(GunState::AimedAway)),
            "Joueur 1 : hors écran (COM3)"
        );
        let mut broken = gun(GunState::Stopped);
        broken.port.clear();
        broken.error = Some("camera lost".into());
        assert_eq!(
            gun_line(Lang::En, &broken),
            "Player 1: stopped — camera lost"
        );
    }
}
