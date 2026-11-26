import assert from "node:assert/strict";
import { test } from "node:test";

import { bgrToHex, Draft, effectiveValue, hexToBgr, isValid, playerKey } from "./config.ts";
import { allKeys, buttonDefault, buttonKey, PLAYER_SECTIONS } from "./schema.ts";

test("player keys", () => {
  assert.equal(playerKey("LeftHanded", 1), "LeftHanded");
  assert.equal(playerKey("LeftHanded", 2), "LeftHandedP2");
});

test("player 2 inherits the shared key, then the default", () => {
  const values = { Joystick: "1" };
  assert.equal(effectiveValue(values, "Joystick", 2, "0"), "1");
  assert.equal(effectiveValue({ ...values, JoystickP2: "0" }, "Joystick", 2, "0"), "0");
  assert.equal(effectiveValue({}, "Joystick", 2, "0"), "0");
  assert.equal(effectiveValue({ JoystickP2: "1" }, "Joystick", 1, "0"), "0");
});

test("draft keeps the original value and drops edits reverted by hand", () => {
  const draft = new Draft();
  draft.set("Smooth", "0", "1");
  draft.set("Smooth", "1", "0");
  assert.equal(draft.size, 0);
  draft.set("RecoilStrength", "100", "80");
  draft.set("RecoilStrength", "80", "60");
  assert.deepEqual(draft.changes(), { RecoilStrength: "60" });
  assert.equal(draft.get("RecoilStrength"), "60");
  draft.clear();
  assert.equal(draft.size, 0);
});

test("ColourMatchRGB is B,R,G", () => {
  assert.equal(bgrToHex("255,255,255"), "#ffffff");
  assert.equal(bgrToHex("10,200,30"), "#c81e0a");
  assert.equal(hexToBgr("#c81e0a"), "10,200,30");
  assert.equal(bgrToHex("1,2"), null);
  assert.equal(bgrToHex("256,0,0"), null);
  assert.equal(bgrToHex("a,b,c"), null);
});

test("validation", () => {
  const field = (key: string) => PLAYER_SECTIONS.flatMap((s) => s.fields).find((f) => f.key === key)!;
  assert.ok(isValid(field("RecoilStrength"), "50"));
  assert.ok(!isValid(field("RecoilStrength"), "150"));
  assert.ok(isValid(field("CameraExposure"), ""), "optional numbers may be empty");
  assert.ok(!isValid(field("RecoilStrength"), ""));
  assert.ok(isValid(field("LeftHanded"), "1"));
  assert.ok(!isValid(field("LeftHanded"), "2"));
  assert.ok(isValid(field("CameraRes"), "640,480"));
  assert.ok(!isValid(field("CameraRes"), "640x480"));
});

test("button keys and defaults follow the configuration reference", () => {
  assert.equal(buttonKey("mouse", "Trigger", "action"), "ButtonTrigger");
  assert.equal(buttonKey("mouse", "FrontLeft", "mod"), "FrontLeftMod");
  assert.equal(buttonKey("joystick", "Trigger", "offscreen"), "JoystickModeButtonTriggerOffscreen");
  assert.equal(buttonKey("joystick", "Trigger", "offscreenMod"), "JoystickModeTriggerOffscreenMod");
  assert.equal(buttonDefault("mouse", "Trigger", "offscreen"), "3");
  assert.equal(buttonDefault("mouse", "RearRight", "action"), "13");
  assert.equal(buttonDefault("mouse", "RearRight", "offscreen"), "13");
  assert.equal(buttonDefault("joystick", "Trigger", "offscreen"), "95");
  assert.equal(buttonDefault("joystick", "Right", "action"), "103");
});

test("every key is unique", () => {
  const { player, global } = allKeys();
  const keys = [...player, ...global];
  assert.equal(new Set(keys).size, keys.length);
  assert.ok(keys.includes("LeftHanded"));
  assert.equal(player.length, PLAYER_SECTIONS.flatMap((s) => s.fields).length + 2 * 10 * 4);
});
