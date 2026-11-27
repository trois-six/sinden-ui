import assert from "node:assert/strict";
import { test } from "node:test";

import type { FirmwareFile, FirmwareJob, FirmwarePhase, Gun } from "./backend.ts";
import {
  canConfirm,
  colourKey,
  confirmation,
  forceFor,
  formatId,
  isBusy,
  jobView,
  pickErrorMessage,
} from "./firmware.ts";
import { MESSAGES } from "./i18n.ts";

const DEVICE = "/sys/devices/platform/usb1/1-1/1-1.2";
const BLUE = { vendor: 0x16c0, product: 0x0f01, colour: "blue" as const };
const RED = { vendor: 0x16c0, product: 0x0f02, colour: "red" as const };

const file = (ids: FirmwareFile["ids"], gunId: FirmwareFile["gun"]["id"] = BLUE): FirmwareFile => ({
  id: 1,
  fileName: "LightgunFirmwareBlue.hex",
  ids,
  gun: { player: 1, device: DEVICE, id: gunId, firmware: "1.9" },
});

const gun = (firmware: string, device = DEVICE): Gun => ({
  player: 1,
  device,
  vendor: 0x16c0,
  product: 0x0f01,
  port: "/dev/ttyACM0",
  camera: "/dev/video0",
  firmware,
  state: "aiming",
  error: null,
});

const job = (phase: FirmwarePhase | null, progress = 0, error: string | null = null): FirmwareJob => ({
  fileName: "LightgunFirmwareBlue.hex",
  gun: { player: 1, device: DEVICE, id: BLUE, firmware: "1.9" },
  force: false,
  progress: phase === null ? null : { device: DEVICE, phase, progress, error },
});

test("identifiers and colours", () => {
  assert.equal(formatId(BLUE), "16c0:0f01");
  assert.equal(formatId({ vendor: 0x16d0, product: 0x39 }), "16d0:0039");
  assert.equal(colourKey("player2"), "colour.player2");
  assert.equal(colourKey(null), "colour.unknown");
  assert.equal(colourKey(undefined), "colour.unknown");
});

test("the gun's own colour needs no acknowledgement", () => {
  const c = confirmation(file([BLUE]));
  assert.deepEqual(c, { kind: "confirm", sameGun: true, needsForce: false });
  assert.ok(canConfirm(c, false, false));
  assert.ok(!canConfirm(c, false, true), "not while the request is sent");
  assert.equal(forceFor(c, true), false, "never forced for the same colour");
});

test("another colour needs the checkbox, and forces the request", () => {
  const c = confirmation(file([RED]));
  assert.deepEqual(c, { kind: "confirm", sameGun: false, needsForce: true });
  assert.ok(!canConfirm(c, false, false));
  assert.ok(canConfirm(c, true, false));
  assert.equal(forceFor(c, false), false);
  assert.equal(forceFor(c, true), true);
});

test("the same product from another vendor is another gun", () => {
  const c = confirmation(file([{ ...BLUE, vendor: 0x16d0 }]));
  assert.equal(c.kind === "confirm" && c.needsForce, true);
});

test("a file with several identifiers matches if one is the gun's", () => {
  assert.deepEqual(confirmation(file([RED, BLUE])), { kind: "confirm", sameGun: true, needsForce: false });
});

test("a gun whose identifiers are unknown leaves the check to the driver", () => {
  const c = confirmation(file([RED], null));
  assert.deepEqual(c, { kind: "confirm", sameGun: null, needsForce: false });
  assert.ok(canConfirm(c, false, false));
});

test("a GunCon file (no identifiers) is refused", () => {
  const c = confirmation(file([]));
  assert.deepEqual(c, { kind: "refused" });
  assert.ok(!canConfirm(c, true, false));
  assert.equal(forceFor(c, true), false);
});

test("progress through the phases", () => {
  assert.deepEqual(jobView(job(null), []), { stage: "starting", progress: 0, label: "firmware.phase.starting" });
  assert.deepEqual(jobView(job("bootloader", 0), []), {
    stage: "running",
    progress: 0,
    label: "firmware.phase.bootloader",
  });
  assert.deepEqual(jobView(job("write", 32), []), { stage: "running", progress: 32, label: "firmware.phase.write" });
  assert.deepEqual(jobView(job("verify", 80), []), { stage: "running", progress: 80, label: "firmware.phase.verify" });
  assert.deepEqual(jobView(job("restart", 97), []), {
    stage: "running",
    progress: 97,
    label: "firmware.phase.restart",
  });
  assert.deepEqual(jobView(job("unknown", 150), []), {
    stage: "running",
    progress: 100,
    label: "firmware.phase.unknown",
  });
});

test("done once the gun is back with its firmware read", () => {
  const done = job("done", 100);
  assert.equal(jobView(done, []).stage, "waiting", "not back yet");
  assert.equal(jobView(done, [gun("")]).stage, "waiting", "back, firmware not read yet");
  assert.equal(jobView(done, [gun("2.1", "/sys/devices/other")]).stage, "waiting", "another gun");
  assert.deepEqual(jobView(done, [gun("2.1")]), { stage: "done", progress: 100, firmware: "2.1", previous: "1.9" });
  // An older driver without devices: by player.
  const older = { ...done, gun: { ...done.gun, device: "" } };
  assert.equal(jobView(older, [gun("2.1", "")]).stage, "done");
});

test("an error keeps its text and where it stopped", () => {
  assert.deepEqual(jobView(job("error", 41, "page 3 read back wrong"), [gun("1.9")]), {
    stage: "error",
    progress: 41,
    error: "page 3 read back wrong",
  });
});

test("busy while requested or running, not once over", () => {
  assert.equal(isBusy(null), false);
  assert.equal(isBusy(job(null)), true);
  assert.equal(isBusy(job("write", 10)), true);
  assert.equal(isBusy(job("restart", 97)), true);
  assert.equal(isBusy(job("done", 100)), false);
  assert.equal(isBusy(job("error", 3, "x")), false);
});

test("pick errors", () => {
  assert.deepEqual(pickErrorMessage({ kind: "too_large" }), { key: "firmware.error.too_large", params: { detail: "" } });
  assert.deepEqual(pickErrorMessage({ kind: "hex", detail: "line 3: bad checksum" }), {
    key: "firmware.error.hex",
    params: { detail: "line 3: bad checksum" },
  });
  assert.deepEqual(pickErrorMessage("ipc failed"), { key: "firmware.error.read", params: { detail: "ipc failed" } });
  assert.deepEqual(pickErrorMessage({ kind: "nonsense" }), {
    key: "firmware.error.read",
    params: { detail: "[object Object]" },
  });
});

test("every message key the logic returns exists", () => {
  const keys = [
    ...(["blue", "red", "black", "player2", null] as const).map(colourKey),
    ...(["bootloader", "write", "verify", "restart", "unknown"] as const).map((phase) => {
      const view = jobView(job(phase, 10), []);
      return view.stage === "running" ? view.label : "";
    }),
    ...["no_gun", "read", "too_large", "not_text", "hex"].map((kind) => pickErrorMessage({ kind, detail: "" }).key),
    "firmware.phase.starting",
    "firmware.waiting",
  ];
  for (const key of keys) assert.ok(key in MESSAGES.en, key);
});
