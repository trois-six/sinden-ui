//! Serves a fake driver on `$SINDEN_DRIVER_SOCKET` (or the first argument)
//! for developing the UI without a gun: two guns whose state changes every
//! few seconds, and an in-memory configuration.

#[cfg(unix)]
fn main() {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use sinden_control::fake::FakeDriver;
    use sinden_control::{ENDPOINT_ENV, Event, Gun, GunState};

    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var(ENDPOINT_ENV).ok())
        .unwrap_or_else(|| panic!("give a socket path as argument or in ${ENDPOINT_ENV}"));
    let gun = |player: u8, state| Gun {
        player,
        port: format!("/dev/ttyACM{}", player - 1),
        camera: format!("/dev/video{}", 2 * (player - 1)),
        firmware: "1.9".into(),
        state,
        error: None,
    };
    let config: BTreeMap<String, String> = [
        ("Joystick", "0"),
        ("LeftHanded", "0"),
        ("ResourceMode", "medium"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let driver = FakeDriver::spawn(
        &path,
        config,
        vec![gun(1, GunState::Aiming), gun(2, GunState::Aiming)],
    )
    .expect("cannot bind the socket");
    eprintln!("fake driver listening on {path}");
    let states = [
        GunState::Aiming,
        GunState::AimedAway,
        GunState::Calibrating,
        GunState::Aiming,
    ];
    for state in states.iter().cycle() {
        std::thread::sleep(Duration::from_secs(3));
        driver.emit(&Event::State(gun(1, *state)));
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("the fake server is only available on Unix");
    std::process::exit(1);
}
