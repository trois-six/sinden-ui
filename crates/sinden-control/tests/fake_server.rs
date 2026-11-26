//! The client against the fake driver, over a real Unix socket.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sinden_control::fake::FakeDriver;
use sinden_control::{Client, Endpoint, Event, Gun, GunState, Guns};

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}.sock", std::process::id()))
}

fn gun(player: u8, state: GunState) -> Gun {
    Gun {
        player,
        port: format!("/dev/ttyACM{}", player - 1),
        camera: format!("/dev/video{}", 2 * (player - 1)),
        firmware: "1.9".into(),
        state,
        error: None,
    }
}

fn config() -> BTreeMap<String, String> {
    [
        ("Joystick", "0"),
        ("LeftHanded", "0"),
        ("ResourceMode", "medium"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect()
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn requests() {
    let driver =
        FakeDriver::spawn(socket("requests"), config(), vec![gun(1, GunState::Aiming)]).unwrap();
    let mut client = Client::connect(&Endpoint::new(driver.path())).unwrap();

    let version = client.version().unwrap();
    assert_eq!(version.api, sinden_control::API_VERSION);

    let status = client.status().unwrap();
    assert_eq!(status.guns, vec![gun(1, GunState::Aiming)]);

    let before = client.get_config().unwrap();
    assert_eq!(before.values["Joystick"], "0");

    let change = BTreeMap::from([
        ("Joystick".to_owned(), "1".to_owned()),
        ("LeftHandedP2".to_owned(), "1".to_owned()),
    ]);
    let after = client.set_config(&change).unwrap();
    assert_eq!(after.values["Joystick"], "1");
    assert_eq!(after.values["LeftHandedP2"], "1");
    assert_eq!(after.values["ResourceMode"], "medium", "other keys kept");
    assert_eq!(driver.set_config_calls(), vec![change]);
    assert_eq!(client.get_config().unwrap(), after);
}

#[test]
fn unknown_method_is_a_service_error() {
    let driver = FakeDriver::spawn(socket("unknown"), config(), vec![]).unwrap();
    let mut client = Client::connect(&Endpoint::new(driver.path())).unwrap();
    let err = client
        .call::<serde_json::Value>("reboot", &serde_json::json!({}))
        .unwrap_err();
    assert!(matches!(err, sinden_control::Error::Service(_)), "{err:?}");
    assert!(!err.is_unreachable());
    // The connection is still usable.
    assert_eq!(client.status().unwrap().guns, []);
}

#[test]
fn subscription_follows_the_guns() {
    let driver = FakeDriver::spawn(
        socket("subscribe"),
        config(),
        vec![gun(1, GunState::Aiming)],
    )
    .unwrap();
    let endpoint = Endpoint::new(driver.path());

    let (status, mut events) = Client::connect_with_timeout(&endpoint, None)
        .unwrap()
        .subscribe()
        .unwrap();
    let mut guns = Guns::from_status(status);
    assert_eq!(guns.as_slice(), [gun(1, GunState::Aiming)]);
    wait_until("the subscriber", || driver.subscriber_count() == 1);

    driver.emit(&Event::GunAdded(gun(2, GunState::Aiming)));
    driver.emit(&Event::State(gun(1, GunState::AimedAway)));
    driver.emit(&Event::GunRemoved(gun(2, GunState::Stopped)));
    for _ in 0..3 {
        guns.apply(&events.next_event().unwrap());
    }
    assert_eq!(guns.as_slice(), [gun(1, GunState::AimedAway)]);

    // The driver's view agrees.
    let status = Client::connect(&endpoint).unwrap().status().unwrap();
    assert_eq!(Guns::from_status(status), guns);

    // The subscription ends when the driver goes away.
    drop(driver);
    assert!(events.next().is_none());
}

#[test]
fn missing_endpoint_is_unreachable() {
    let err = Client::connect(&Endpoint::new(socket("nobody-listens"))).unwrap_err();
    assert!(err.is_unreachable(), "{err:?}");
}
