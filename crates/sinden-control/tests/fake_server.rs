//! The client against the fake driver, over a real Unix socket.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sinden_control::fake::FakeDriver;
use sinden_control::{
    Client, Endpoint, Error, Event, FirmwarePhase, FirmwareTarget, Gun, GunState, Guns,
};

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}.sock", std::process::id()))
}

fn gun(player: u8, state: GunState) -> Gun {
    Gun {
        player,
        device: format!("/sys/devices/fake/usb1/1-1.{player}"),
        vendor: Some(0x16c0),
        product: Some(if player == 1 { 0x0f01 } else { 0x0f02 }),
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

/// One Intel HEX record with its checksum.
fn record(address: u16, kind: u8, data: &[u8]) -> String {
    let [high, low] = address.to_be_bytes();
    let mut bytes = vec![u8::try_from(data.len()).unwrap(), high, low, kind];
    bytes.extend_from_slice(data);
    let sum = bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b));
    bytes.push(sum.wrapping_neg());
    bytes.iter().fold(":".to_owned(), |mut line, b| {
        let _ = write!(line, "{b:02X}");
        line
    }) + "\r\n"
}

/// A firmware file presenting `16c0:<product>`.
fn firmware_file(product: u16) -> String {
    let [p0, p1] = product.to_le_bytes();
    record(
        0,
        0x00,
        &[0x0c, 0x94, 0x5c, 0x00, 0x12, 0x01, 0xc0, 0x16, p0, p1],
    ) + &record(0, 0x01, &[])
}

/// The events until a firmware update ends, applied to `guns`; then the
/// events until the gun is back with a firmware version.
fn follow_update(
    events: &mut sinden_control::Subscription,
    guns: &mut Guns,
    player: u8,
) -> Vec<FirmwarePhase> {
    let mut phases = Vec::new();
    loop {
        let event = events.next_event().unwrap();
        guns.apply(&event);
        if let Event::Firmware(progress) = &event {
            phases.push(progress.phase);
            if progress.phase == FirmwarePhase::Error {
                return phases;
            }
        }
        if phases.last() == Some(&FirmwarePhase::Done)
            && guns
                .as_slice()
                .iter()
                .any(|g| g.player == player && !g.firmware.is_empty())
        {
            return phases;
        }
    }
}

#[test]
fn firmware_update_through_its_events() {
    let driver = FakeDriver::spawn(
        socket("firmware"),
        config(),
        vec![gun(1, GunState::Aiming), gun(2, GunState::Aiming)],
    )
    .unwrap();
    let endpoint = Endpoint::new(driver.path());
    let (status, mut events) = Client::connect_with_timeout(&endpoint, None)
        .unwrap()
        .subscribe()
        .unwrap();
    let mut guns = Guns::from_status(status);
    wait_until("the subscriber", || driver.subscriber_count() == 1);

    let mut client = Client::connect(&endpoint).unwrap();
    let device = gun(1, GunState::Aiming).device;
    let started = client
        .firmware_update(
            &FirmwareTarget::Device(device.clone()),
            &firmware_file(0x0f01),
            false,
        )
        .unwrap();
    assert_eq!(started.device, device);
    assert!(started.started);

    let phases = follow_update(&mut events, &mut guns, 1);
    assert_eq!(phases.first(), Some(&FirmwarePhase::Bootloader));
    assert_eq!(phases.last(), Some(&FirmwarePhase::Done));
    assert!(phases.contains(&FirmwarePhase::Write) && phases.contains(&FirmwarePhase::Verify));
    let back = guns.as_slice().iter().find(|g| g.player == 1).unwrap();
    assert_eq!(
        (back.device.as_str(), back.firmware.as_str()),
        (device.as_str(), "2.1")
    );
    assert_eq!(driver.firmware_calls().len(), 1);
    assert!(!driver.firmware_calls()[0].force);
}

#[test]
fn firmware_update_of_another_colour_needs_force() {
    let driver =
        FakeDriver::spawn(socket("colour"), config(), vec![gun(1, GunState::Aiming)]).unwrap();
    let mut client = Client::connect(&Endpoint::new(driver.path())).unwrap();
    // Player 1 is Blue (0f01); a Red file.
    let red = firmware_file(0x0f02);
    match client.firmware_update(&FirmwareTarget::Player(1), &red, false) {
        Err(Error::Service(message)) => {
            assert!(message.contains("refused without force"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    match client.firmware_update(&FirmwareTarget::Player(1), "not hex", true) {
        Err(Error::Service(message)) => assert!(message.contains("line 1"), "{message}"),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(driver.firmware_calls().len(), 0);
    client
        .firmware_update(&FirmwareTarget::Player(1), &red, true)
        .unwrap();
    assert!(driver.firmware_calls()[0].force);
}

#[test]
fn a_failed_firmware_update_is_retried_by_device() {
    let driver =
        FakeDriver::spawn(socket("retry"), config(), vec![gun(1, GunState::Aiming)]).unwrap();
    driver.fail_next_firmware_update("page 12 read back wrong; the gun waits in its bootloader");
    let endpoint = Endpoint::new(driver.path());
    let (status, mut events) = Client::connect_with_timeout(&endpoint, None)
        .unwrap()
        .subscribe()
        .unwrap();
    let mut guns = Guns::from_status(status);
    wait_until("the subscriber", || driver.subscriber_count() == 1);
    let mut client = Client::connect(&endpoint).unwrap();
    let device = FirmwareTarget::Device(gun(1, GunState::Aiming).device);
    let blue = firmware_file(0x0f01);

    client.firmware_update(&device, &blue, false).unwrap();
    let phases = follow_update(&mut events, &mut guns, 1);
    assert_eq!(phases.last(), Some(&FirmwarePhase::Error));
    assert_eq!(guns.as_slice(), [], "the gun left the status");
    // Not reachable by player any more, only by its device.
    assert!(
        client
            .firmware_update(&FirmwareTarget::Player(1), &blue, false)
            .is_err()
    );
    client.firmware_update(&device, &blue, false).unwrap();
    let phases = follow_update(&mut events, &mut guns, 1);
    assert_eq!(phases.last(), Some(&FirmwarePhase::Done));
    assert_eq!(guns.as_slice()[0].firmware, "2.1");
}
