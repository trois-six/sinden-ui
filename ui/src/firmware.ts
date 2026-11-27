// Pure firmware-update logic: what the confirmation dialog shows and
// allows, and where an update is. No DOM, no Tauri, so that `node --test`
// covers it.

import type { FirmwareFile, FirmwareJob, Gun, GunColour, PickError } from "./backend.ts";
import type { MsgKey } from "./i18n.ts";

type UsbId = { vendor: number; product: number };

const hex4 = (n: number): string => n.toString(16).padStart(4, "0");

/** `16c0:0f01`. */
export function formatId(id: UsbId): string {
  return `${hex4(id.vendor)}:${hex4(id.product)}`;
}

export function colourKey(colour: GunColour | null | undefined): MsgKey {
  return colour ? `colour.${colour}` : "colour.unknown";
}

export type Confirmation =
  /** A file with no Sinden identifiers (GunCon): never written. */
  | { kind: "refused" }
  | {
      kind: "confirm";
      /** Whether the file is for the gun's own identifiers; null when the driver does not report them. */
      sameGun: boolean | null;
      /** Another colour: the user must say so, and the request is forced. */
      needsForce: boolean;
    };

export function confirmation(file: FirmwareFile): Confirmation {
  if (file.ids.length === 0) return { kind: "refused" };
  const gun = file.gun.id;
  const sameGun = gun ? file.ids.some((id) => id.vendor === gun.vendor && id.product === gun.product) : null;
  return { kind: "confirm", sameGun, needsForce: sameGun === false };
}

/** Whether "Update" may be pressed. */
export function canConfirm(c: Confirmation, acknowledged: boolean, sending: boolean): boolean {
  return c.kind === "confirm" && !sending && (!c.needsForce || acknowledged);
}

/** The `force` of the request: only for another colour the user acknowledged. */
export function forceFor(c: Confirmation, acknowledged: boolean): boolean {
  return c.kind === "confirm" && c.needsForce && acknowledged;
}

const RUNNING_PHASES = new Set(["bootloader", "write", "verify", "restart"]);

export type JobView =
  /** Requested, no event yet. */
  | { stage: "starting"; progress: number; label: MsgKey }
  | { stage: "running"; progress: number; label: MsgKey }
  /** Done, the gun not back in the status (or its firmware not read) yet. */
  | { stage: "waiting"; progress: number; label: MsgKey }
  | { stage: "done"; progress: number; firmware: string; previous: string }
  | { stage: "error"; progress: number; error: string };

/** Where `job` is, the gun's return included. */
export function jobView(job: FirmwareJob, guns: Gun[]): JobView {
  const p = job.progress;
  if (!p) return { stage: "starting", progress: 0, label: "firmware.phase.starting" };
  const progress = Math.max(0, Math.min(100, p.progress));
  if (p.phase === "error") return { stage: "error", progress, error: p.error ?? "" };
  if (p.phase === "done") {
    const back = guns.find((g) =>
      job.gun.device ? g.device === job.gun.device : g.player === job.gun.player,
    );
    if (back && back.firmware) {
      return { stage: "done", progress: 100, firmware: back.firmware, previous: job.gun.firmware };
    }
    return { stage: "waiting", progress: 100, label: "firmware.waiting" };
  }
  const label = (RUNNING_PHASES.has(p.phase) ? `firmware.phase.${p.phase}` : "firmware.phase.unknown") as MsgKey;
  return { stage: "running", progress, label };
}

/** Whether an update is being requested or running: the guns' other actions wait. */
export function isBusy(job: FirmwareJob | null): boolean {
  return job !== null && (job.progress === null || (job.progress.phase !== "done" && job.progress.phase !== "error"));
}

const PICK_ERRORS = new Set(["no_gun", "read", "too_large", "not_text", "hex"]);

/** The message of a pickFirmware rejection: a PickError, or anything else. */
export function pickErrorMessage(err: unknown): { key: MsgKey; params: Record<string, string> } {
  if (typeof err === "object" && err !== null && "kind" in err && PICK_ERRORS.has(String(err.kind))) {
    const e = err as PickError;
    return { key: `firmware.error.${e.kind}` as MsgKey, params: { detail: "detail" in e ? e.detail : "" } };
  }
  return { key: "firmware.error.read", params: { detail: String(err) } };
}
