// The Tauri commands and events of the Rust side (ui/src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Values } from "./config.ts";

/** The driver's gun states; anything else is shown as "unknown". */
export type GunState = "starting" | "aiming" | "aimed_away" | "calibrating" | "stopped" | "unknown";

export type Gun = {
  player: number;
  /** The gun's USB device (empty with an older driver). */
  device: string;
  vendor: number | null;
  product: number | null;
  port: string;
  camera: string;
  firmware: string;
  state: GunState;
  error: string | null;
};

export type ServiceStatus = {
  connected: boolean;
  driverVersion: string | null;
  api: number | null;
  /** The control API version this UI speaks. */
  apiSupported: number;
  guns: Gun[];
  error: string | null;
  endpoint: string;
};

export type ConfigSnapshot = { path: string; values: Values };

/** The gun a pair of USB identifiers makes (ui/src-tauri/src/firmware.rs). */
export type GunColour = "blue" | "red" | "black" | "player2";

export type IdInfo = { vendor: number; product: number; colour: GunColour | null };

/** The gun an update is for, as the status showed it when the file was picked. */
export type GunRef = { player: number; device: string; id: IdInfo | null; firmware: string };

/** A firmware file picked and checked, waiting for the user's confirmation. */
export type FirmwareFile = { id: number; fileName: string; ids: IdInfo[]; gun: GunRef };

/** Why a picked file cannot be used. */
export type PickError =
  | { kind: "no_gun" | "too_large" | "not_text" }
  | { kind: "read" | "hex"; detail: string };

export type FirmwarePhase = "bootloader" | "write" | "verify" | "restart" | "done" | "error" | "unknown";

export type FirmwareProgress = { device: string; phase: FirmwarePhase; progress: number; error: string | null };

/** A firmware update sent to the driver; `progress` is its last event. */
export type FirmwareJob = { fileName: string; gun: GunRef; force: boolean; progress: FirmwareProgress | null };

export const getStatus = (): Promise<ServiceStatus> => invoke("get_status");
export const getConfig = (): Promise<ConfigSnapshot> => invoke("get_config");
export const setConfig = (values: Values): Promise<ConfigSnapshot> => invoke("set_config", { values });
export const getAutostart = (): Promise<boolean> => invoke("get_autostart");
export const setAutostart = (enabled: boolean): Promise<void> => invoke("set_autostart", { enabled });
export const setLanguage = (lang: string): Promise<void> => invoke("set_language", { lang });

/** Opens the file picker for `player`'s gun; null when cancelled. Rejects with a PickError. */
export const pickFirmware = (player: number): Promise<FirmwareFile | null> => invoke("pick_firmware", { player });
export const startFirmwareUpdate = (id: number, force: boolean): Promise<FirmwareJob> =>
  invoke("start_firmware_update", { id, force });
export const retryFirmwareUpdate = (): Promise<FirmwareJob> => invoke("retry_firmware_update");
export const dismissFirmwareUpdate = (): Promise<void> => invoke("dismiss_firmware_update");
export const getFirmwareUpdate = (): Promise<FirmwareJob | null> => invoke("get_firmware_update");

export const onFirmware = (callback: (job: FirmwareJob | null) => void): Promise<UnlistenFn> =>
  listen<FirmwareJob | null>("firmware-changed", (event) => callback(event.payload));

export const onStatus = (callback: (status: ServiceStatus) => void): Promise<UnlistenFn> =>
  listen<ServiceStatus>("status-changed", (event) => callback(event.payload));
