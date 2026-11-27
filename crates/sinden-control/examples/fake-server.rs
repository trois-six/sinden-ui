//! Serves a fake driver on `$SINDEN_DRIVER_SOCKET` (or the first argument)
//! for developing the UI without a gun: two guns (Blue and Red) whose
//! state changes every few seconds, and an in-memory configuration.
//!
//! A firmware update takes about ten seconds; with
//! `SINDEN_FAKE_FIRMWARE_FAIL=1` the first one fails during its write (and
//! the gun waits "in its bootloader" for a retry).

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
        device: format!("/sys/devices/fake/usb1/1-1.{player}"),
        vendor: Some(0x16c0),
        product: Some(if player == 1 { 0x0f01 } else { 0x0f02 }),
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
    driver.set_firmware_step(Duration::from_millis(400));
    if std::env::var_os("SINDEN_FAKE_FIRMWARE_FAIL").is_some_and(|v| v == "1") {
        driver.fail_next_firmware_update(
            "page 12 read back wrong (fake); the gun waits in its bootloader: update it again",
        );
    }
    eprintln!("fake driver listening on {path}");
    let states = [
        GunState::AimedAway,
        GunState::Calibrating,
        GunState::Starting,
        GunState::Aiming,
    ];
    for state in states.iter().cycle() {
        std::thread::sleep(Duration::from_secs(3));
        // Player 1's state, while it is in the status (not being updated).
        if let Some(mut current) = driver.guns().into_iter().find(|g| g.player == 1) {
            current.state = *state;
            driver.emit(&Event::State(current));
        }
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("the fake server is only available on Unix");
    std::process::exit(1);
}
