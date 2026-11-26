// The Tauri commands and events of the Rust side (ui/src-tauri/src/commands.rs).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Values } from "./config.ts";

/** The driver's gun states; anything else is shown as "unknown". */
export type GunState = "starting" | "aiming" | "aimed_away" | "calibrating" | "stopped" | "unknown";

export type Gun = {
  player: number;
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

export const getStatus = (): Promise<ServiceStatus> => invoke("get_status");
export const getConfig = (): Promise<ConfigSnapshot> => invoke("get_config");
export const setConfig = (values: Values): Promise<ConfigSnapshot> => invoke("set_config", { values });
export const getAutostart = (): Promise<boolean> => invoke("get_autostart");
export const setAutostart = (enabled: boolean): Promise<void> => invoke("set_autostart", { enabled });
export const setLanguage = (lang: string): Promise<void> => invoke("set_language", { lang });

export const onStatus = (callback: (status: ServiceStatus) => void): Promise<UnlistenFn> =>
  listen<ServiceStatus>("status-changed", (event) => callback(event.payload));
