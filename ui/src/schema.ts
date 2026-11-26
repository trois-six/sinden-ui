// Every configuration key of LightgunMono.exe.config the UI edits, grouped
// as the settings window shows them. Keys, defaults and value meanings
// follow the driver's configuration reference; `LeftHanded` is the
// handedness key of the roadmap (§ 2).
//
// Per-player keys: player 1 edits `Key`, player 2 edits `KeyP2` (which
// falls back to `Key` when absent from the file).

import type { MsgKey } from "./i18n.ts";

export type Option = { value: string; label: MsgKey };

export type Field =
  | { key: string; kind: "bool"; label: MsgKey; def: string; help?: MsgKey }
  | { key: string; kind: "select"; label: MsgKey; def: string; options: Option[]; help?: MsgKey }
  | {
      key: string;
      kind: "number";
      label: MsgKey;
      def: string;
      min?: number;
      max?: number;
      step?: number;
      /** Empty is allowed and means "not set". */
      optional?: boolean;
      help?: MsgKey;
    }
  | { key: string; kind: "text"; label: MsgKey; def: string; placeholder?: string; pattern?: string; help?: MsgKey }
  /** `ColourMatchRGB`: "B,R,G" in the file, a colour picker in the UI. */
  | { key: string; kind: "colour"; label: MsgKey; def: string; help?: MsgKey };

export type Section = { id: string; title: MsgKey; fields: Field[] };

const bool = (key: string, label: MsgKey, def: string, help?: MsgKey): Field => ({
  key,
  kind: "bool",
  label,
  def,
  help,
});

const num = (
  key: string,
  label: MsgKey,
  def: string,
  extra: { min?: number; max?: number; step?: number; optional?: boolean; help?: MsgKey } = {},
): Field => ({ key, kind: "number", label, def, ...extra });

/** Sections shown for each player, in order. */
export const PLAYER_SECTIONS: Section[] = [
  {
    id: "mode",
    title: "section.mode",
    fields: [
      {
        key: "Joystick",
        kind: "select",
        label: "field.Joystick",
        def: "0",
        options: [
          { value: "0", label: "mode.mouse" },
          { value: "1", label: "mode.joystick" },
        ],
        help: "help.Joystick",
      },
      {
        key: "LeftHanded",
        kind: "select",
        label: "field.LeftHanded",
        def: "0",
        options: [
          { value: "0", label: "hand.right" },
          { value: "1", label: "hand.left" },
        ],
        help: "help.LeftHanded",
      },
      {
        key: "GangstaSetting",
        kind: "select",
        label: "field.GangstaSetting",
        def: "2",
        options: [
          { value: "0", label: "grip.off" },
          { value: "1", label: "grip.left" },
          { value: "2", label: "grip.right" },
        ],
        help: "help.GangstaSetting",
      },
      {
        key: "ResourceMode",
        kind: "select",
        label: "field.ResourceMode",
        def: "medium",
        options: [
          { value: "max", label: "resource.max" },
          { value: "medium", label: "resource.medium" },
          { value: "low", label: "resource.low" },
        ],
        help: "help.ResourceMode",
      },
      bool("Smooth", "field.Smooth", "0", "help.Smooth"),
      num("AimedAwayDelay", "field.AimedAwayDelay", "5", { min: 0, step: 0.5, help: "help.AimedAwayDelay" }),
      bool("OffscreenReload", "field.OffscreenReload", "0"),
    ],
  },
  {
    id: "recoil",
    title: "section.recoil",
    fields: [
      bool("IAgreeRecoilTermsInLicense", "field.IAgreeRecoilTermsInLicense", "1"),
      bool("EnableRecoil", "field.EnableRecoil", "0"),
      num("RecoilStrength", "field.RecoilStrength", "100", { min: 0, max: 100 }),
      bool("RecoilTrigger", "field.RecoilTrigger", "1"),
      bool("RecoilTriggerOffscreen", "field.RecoilTriggerOffscreen", "0"),
      bool("RecoilPumpActionOnEvent", "field.RecoilPumpActionOnEvent", "0"),
      bool("RecoilPumpActionOffEvent", "field.RecoilPumpActionOffEvent", "0"),
      bool("RecoilFrontLeft", "field.RecoilFrontLeft", "0"),
      bool("RecoilFrontRight", "field.RecoilFrontRight", "0"),
      bool("RecoilBackLeft", "field.RecoilBackLeft", "0"),
      bool("RecoilBackRight", "field.RecoilBackRight", "0"),
      {
        key: "TriggerRecoilNormalOrRepeat",
        kind: "select",
        label: "field.TriggerRecoilNormalOrRepeat",
        def: "0",
        options: [
          { value: "0", label: "recoil.single" },
          { value: "1", label: "recoil.auto" },
        ],
      },
      num("AutoRecoilStrength", "field.AutoRecoilStrength", "40", { min: 0, max: 100 }),
      num("AutoRecoilStartDelay", "field.AutoRecoilStartDelay", "0", { min: 0 }),
      num("AutoRecoilDelayBetweenPulses", "field.AutoRecoilDelayBetweenPulses", "13", { min: 0 }),
    ],
  },
  {
    id: "camera",
    title: "section.camera",
    fields: [
      {
        key: "CameraExposureAuto",
        kind: "select",
        label: "field.CameraExposureAuto",
        def: "1",
        options: [
          { value: "1", label: "exposure.manual" },
          { value: "3", label: "exposure.auto" },
        ],
      },
      num("CameraExposure", "field.CameraExposure", "", { min: 0, optional: true, help: "help.CameraExposure" }),
      num("CameraBrightness", "field.CameraBrightness", "100", { min: 0 }),
      num("CameraContrast", "field.CameraContrast", "50", { min: 0 }),
      {
        key: "CameraRes",
        kind: "text",
        label: "field.CameraRes",
        def: "320,240",
        placeholder: "320,240",
        pattern: "^\\d{2,4},\\d{2,4}$",
        help: "help.CameraRes",
      },
    ],
  },
  {
    id: "aim",
    title: "section.aim",
    fields: [
      num("OffsetX", "field.OffsetX", "0", { step: 0.1 }),
      num("OffsetY", "field.OffsetY", "0", { step: 0.1 }),
      num("OffsetXRatio", "field.OffsetXRatio", "1", { step: 0.01 }),
      num("OffsetYRatio", "field.OffsetYRatio", "1", { step: 0.01 }),
      num("CalibrateX", "field.CalibrateX", "", { step: 0.01, optional: true, help: "help.Calibrate" }),
      num("CalibrateY", "field.CalibrateY", "", { step: 0.01, optional: true, help: "help.Calibrate" }),
      bool("EnableCalibration", "field.EnableCalibration", "1"),
      bool("AutoSaveCalibrationInLightgun", "field.AutoSaveCalibrationInLightgun", "1"),
    ],
  },
  {
    id: "vision",
    title: "section.vision",
    fields: [
      { key: "ColourMatchRGB", kind: "colour", label: "field.ColourMatchRGB", def: "255,255,255", help: "help.ColourMatchRGB" },
      num("ColourMatchRange", "field.ColourMatchRange", "220", { min: 0, max: 1000 }),
      bool("MatchOnlyWherePointing", "field.MatchOnlyWherePointing", "0", "help.MatchOnlyWherePointing"),
    ],
  },
  {
    id: "devices",
    title: "section.devices",
    fields: [
      { key: "SerialPortWrite", kind: "text", label: "field.SerialPortWrite", def: "", placeholder: "auto", help: "help.SerialPortWrite" },
      { key: "SerialPortSecondary", kind: "text", label: "field.SerialPortSecondary", def: "", placeholder: "—" },
      { key: "VideoDevice", kind: "text", label: "field.VideoDevice", def: "", placeholder: "auto" },
    ],
  },
];

/** Keys that apply to the whole driver (no P2 variant). */
export const GLOBAL_SECTIONS: Section[] = [
  {
    id: "logging",
    title: "section.logging",
    fields: [
      {
        key: "ConsoleOutputVerbose0to2",
        kind: "select",
        label: "field.ConsoleOutputVerbose0to2",
        def: "0",
        options: [
          { value: "0", label: "log.info" },
          { value: "1", label: "log.debug" },
          { value: "2", label: "log.trace" },
        ],
      },
    ],
  },
];

// ---- Buttons -------------------------------------------------------------

export type ButtonMode = "mouse" | "joystick";

/** The gun's buttons, in the order the settings window lists them. */
export const BUTTONS = [
  "Trigger",
  "PumpAction",
  "FrontLeft",
  "FrontRight",
  "RearLeft",
  "RearRight",
  "Up",
  "Down",
  "Left",
  "Right",
] as const;
export type Button = (typeof BUTTONS)[number];

const MOUSE_DEFAULTS: Record<Button, number> = {
  Trigger: 1,
  PumpAction: 3,
  FrontLeft: 3,
  FrontRight: 2,
  RearLeft: 9,
  RearRight: 13,
  Up: 74,
  Down: 75,
  Left: 76,
  Right: 77,
};

const JOYSTICK_DEFAULTS: Record<Button, number> = {
  Trigger: 94,
  PumpAction: 95,
  FrontLeft: 96,
  FrontRight: 97,
  RearLeft: 98,
  RearRight: 99,
  Up: 100,
  Down: 101,
  Left: 102,
  Right: 103,
};

export type ButtonSlot = "action" | "mod" | "offscreen" | "offscreenMod";

/** The key of one button setting, e.g. `JoystickModeButtonTriggerOffscreen`. */
export function buttonKey(mode: ButtonMode, button: Button, slot: ButtonSlot): string {
  const prefix = mode === "joystick" ? "JoystickMode" : "";
  switch (slot) {
    case "action":
      return `${prefix}Button${button}`;
    case "mod":
      return `${prefix}${button}Mod`;
    case "offscreen":
      return `${prefix}Button${button}Offscreen`;
    case "offscreenMod":
      return `${prefix}${button}OffscreenMod`;
  }
}

/** The default of one button setting. */
export function buttonDefault(mode: ButtonMode, button: Button, slot: ButtonSlot): string {
  const table = mode === "joystick" ? JOYSTICK_DEFAULTS : MOUSE_DEFAULTS;
  switch (slot) {
    case "mod":
    case "offscreenMod":
      return "0";
    case "action":
      return String(table[button]);
    case "offscreen":
      if (button === "Trigger") return mode === "joystick" ? "95" : "3";
      return String(table[button]);
  }
}

// ---- Actions ---------------------------------------------------------------

/** An action of the driver's ordered list of output codes. */
export type Action = { code: number; label: MsgKey; arg?: string };

const range = (first: number, count: number, label: MsgKey, arg: (i: number) => string): Action[] =>
  Array.from({ length: count }, (_, i) => ({ code: first + i, label, arg: arg(i) }));
const letter = (base: string) => (i: number) => String.fromCharCode(base.charCodeAt(0) + i);

/**
 * The output codes, as the comment of Sinden's LightgunMono.exe.config
 * lists them (0 to 113). A value outside the list stays selectable by
 * number.
 */
export const KNOWN_ACTIONS: Action[] = [
  { code: 0, label: "action.none" },
  { code: 1, label: "action.mouseLeft" },
  { code: 2, label: "action.mouseMiddle" },
  { code: 3, label: "action.mouseRight" },
  { code: 4, label: "action.pauseMovement" },
  { code: 5, label: "action.turboLeft" },
  { code: 6, label: "action.turboReload" },
  { code: 7, label: "action.altB" },
  ...range(8, 10, "action.key", (i) => String(i)),
  ...range(18, 26, "action.keyUpper", letter("A")),
  ...range(44, 26, "action.key", letter("a")),
  { code: 70, label: "action.return" },
  { code: 71, label: "action.space" },
  { code: 72, label: "action.escape" },
  { code: 73, label: "action.tab" },
  { code: 74, label: "action.arrowUp" },
  { code: 75, label: "action.arrowDown" },
  { code: 76, label: "action.arrowLeft" },
  { code: 77, label: "action.arrowRight" },
  { code: 78, label: "action.key", arg: "+" },
  { code: 79, label: "action.key", arg: "," },
  { code: 80, label: "action.key", arg: "-" },
  { code: 81, label: "action.key", arg: "." },
  ...range(82, 12, "action.functionKey", (i) => String(i + 1)),
  ...range(94, 20, "action.joystickButton", (i) => String(i + 1)),
];

/** Every field key of the schema (per-player keys without suffix). */
export function allKeys(): { player: string[]; global: string[] } {
  const player = PLAYER_SECTIONS.flatMap((s) => s.fields.map((f) => f.key));
  for (const mode of ["mouse", "joystick"] as const) {
    for (const button of BUTTONS) {
      for (const slot of ["action", "mod", "offscreen", "offscreenMod"] as const) {
        player.push(buttonKey(mode, button, slot));
      }
    }
  }
  return { player, global: GLOBAL_SECTIONS.flatMap((s) => s.fields.map((f) => f.key)) };
}
