// Pure configuration logic: per-player keys, effective values, the set of
// pending edits and the ColourMatchRGB conversion. No DOM, no Tauri, so
// that `node --test` covers it.

import type { Field } from "./schema.ts";

export type Values = Record<string, string>;
export type Player = 1 | 2;

/** The key a player's value is stored under: `Key` or `KeyP2`. */
export function playerKey(key: string, player: Player): string {
  return player === 2 ? `${key}P2` : key;
}

/**
 * The value a player sees for `key`: its own key, else (player 2) the
 * shared key, else the default.
 */
export function effectiveValue(values: Values, key: string, player: Player | null, def: string): string {
  const own = player === null ? key : playerKey(key, player);
  if (own in values) return values[own];
  if (player === 2 && key in values) return values[key];
  return def;
}

/** Edits not yet sent to the driver, each with the value it replaced. */
export class Draft {
  #edits = new Map<string, { from: string; to: string }>();

  /** Records `key` going from `from` (as shown) to `to`. */
  set(key: string, from: string, to: string): void {
    const original = this.#edits.get(key)?.from ?? from;
    if (original === to) this.#edits.delete(key);
    else this.#edits.set(key, { from: original, to });
  }

  /** The pending value of `key`, if edited. */
  get(key: string): string | undefined {
    return this.#edits.get(key)?.to;
  }

  /** The values to send with `set_config`. */
  changes(): Values {
    return Object.fromEntries([...this.#edits].map(([key, edit]) => [key, edit.to]));
  }

  get size(): number {
    return this.#edits.size;
  }

  clear(): void {
    this.#edits.clear();
  }
}

/** Whether `value` is acceptable for `field`. */
export function isValid(field: Field, value: string): boolean {
  switch (field.kind) {
    case "bool":
      return value === "0" || value === "1";
    case "select":
      return field.options.some((o) => o.value === value);
    case "number": {
      if (value.trim() === "") return field.optional === true;
      const n = Number(value);
      if (!Number.isFinite(n)) return false;
      if (field.min !== undefined && n < field.min) return false;
      if (field.max !== undefined && n > field.max) return false;
      return true;
    }
    case "text":
      return field.pattern === undefined || value === "" || new RegExp(field.pattern).test(value);
    case "colour":
      return bgrToHex(value) !== null;
  }
}

const byte = (s: string): number | null => {
  if (!/^\s*\d{1,3}\s*$/.test(s)) return null;
  const n = Number(s);
  return n <= 255 ? n : null;
};

const hex2 = (n: number): string => n.toString(16).padStart(2, "0");

/** `ColourMatchRGB` ("B,R,G", Sinden's order) to `#rrggbb`; null if invalid. */
export function bgrToHex(value: string): string | null {
  const parts = value.split(",").map(byte);
  if (parts.length !== 3 || parts.some((p) => p === null)) return null;
  const [b, r, g] = parts as number[];
  return `#${hex2(r)}${hex2(g)}${hex2(b)}`;
}

/** `#rrggbb` to `ColourMatchRGB` ("B,R,G"). */
export function hexToBgr(hex: string): string {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) throw new Error(`not a #rrggbb colour: ${hex}`);
  const [r, g, b] = [m[1], m[2], m[3]].map((h) => parseInt(h, 16));
  return `${b},${r},${g}`;
}
