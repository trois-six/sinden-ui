// The settings window: guns' state, per-player settings, general settings.

import * as backend from "./backend.ts";
import type { ConfigSnapshot, ServiceStatus } from "./backend.ts";
import { bgrToHex, Draft, effectiveValue, hexToBgr, isValid, type Player, playerKey } from "./config.ts";
import { lang, type MsgKey, resolveLang, setLang, t } from "./i18n.ts";
import {
  BUTTONS,
  type ButtonMode,
  type ButtonSlot,
  buttonDefault,
  buttonKey,
  type Field,
  GLOBAL_SECTIONS,
  KNOWN_ACTIONS,
  PLAYER_SECTIONS,
  type Section,
} from "./schema.ts";

type Tab = "p1" | "p2" | "general";

const state = {
  status: null as ServiceStatus | null,
  snapshot: null as ConfigSnapshot | null,
  draft: new Draft(),
  invalid: new Set<string>(),
  tab: "p1" as Tab,
  langPref: readPref("lang") ?? "auto",
  message: null as { kind: "ok" | "error"; text: string } | null,
  autostart: null as boolean | null,
  saving: false,
};

// ---- Small DOM helper ----------------------------------------------------

type Attrs = Record<string, string | number | boolean | EventListener | undefined>;

function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: (Node | string | null | undefined | false)[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === false) continue;
    if (typeof value === "function") el.addEventListener(name.replace(/^on/, ""), value);
    else if (value === true) el.setAttribute(name, "");
    else el.setAttribute(name, String(value));
  }
  for (const child of children) {
    if (child === null || child === undefined || child === false) continue;
    el.append(child);
  }
  return el;
}

function readPref(name: string): string | null {
  try {
    return localStorage.getItem(name);
  } catch {
    return null;
  }
}

function writePref(name: string, value: string): void {
  try {
    localStorage.setItem(name, value);
  } catch {
    // Preferences are a convenience: ignore storage errors.
  }
}

const $ = (id: string): HTMLElement => document.getElementById(id)!;

// ---- Layout --------------------------------------------------------------

function renderShell(): void {
  document.documentElement.lang = lang();
  document.title = t("app.title");
  const app = $("app");
  app.replaceChildren(
    h(
      "header",
      { class: "top" },
      h("img", { src: "/icon.svg", alt: "", class: "logo", width: 32, height: 32 }),
      h("h1", {}, t("app.title")),
      h("div", { id: "service", class: "service" }),
      languagePicker(),
    ),
    h("section", { id: "guns", class: "guns", "aria-label": t("guns.title") }),
    h("nav", { id: "tabs", class: "tabs", role: "tablist" }),
    h("main", { id: "content" }),
    h("footer", { id: "footer", class: "footer" }),
  );
  renderStatus();
  renderTabs();
  renderContent();
  renderFooter();
}

function languagePicker(): HTMLElement {
  const select = h(
    "select",
    {
      "aria-label": t("app.language"),
      onchange: (e: Event) => {
        state.langPref = (e.target as HTMLSelectElement).value;
        writePref("lang", state.langPref);
        applyLanguage();
        renderShell();
      },
    },
    ...(["auto", "en", "fr"] as const).map((value) =>
      h("option", { value, selected: state.langPref === value }, t(`lang.${value}` as MsgKey)),
    ),
  );
  return h("label", { class: "lang" }, select);
}

function applyLanguage(): void {
  setLang(resolveLang(state.langPref, navigator.languages ?? [navigator.language]));
  backend.setLanguage(lang()).catch(() => {});
}

// ---- Service and guns ----------------------------------------------------

function renderStatus(): void {
  const status = state.status;
  const service = $("service");
  const guns = $("guns");
  if (!status) {
    service.replaceChildren();
    guns.replaceChildren();
    return;
  }
  service.className = `service ${status.connected ? "ok" : "down"}`;
  service.replaceChildren(
    h("span", { class: "dot", "aria-hidden": "true" }),
    status.connected
      ? t("service.connected", { version: status.driverVersion ?? "?" })
      : t("service.disconnected"),
  );
  service.title = status.error ?? status.endpoint;

  const cards = status.guns.map((gun) =>
    h(
      "div",
      { class: `gun state-${gun.state}` },
      h("strong", {}, t("gun.player", { player: gun.player })),
      h("span", { class: "badge" }, t(`state.${gun.state}` as MsgKey)),
      h(
        "small",
        {},
        [gun.port, gun.firmware ? t("gun.firmware", { firmware: gun.firmware }) : ""].filter(Boolean).join(" · "),
      ),
      gun.error ? h("small", { class: "error" }, gun.error) : null,
    ),
  );
  guns.replaceChildren(...(cards.length ? cards : [h("p", { class: "muted" }, status.connected ? t("guns.none") : "")]));
}

async function onStatus(status: ServiceStatus): Promise<void> {
  const wasConnected = state.status?.connected ?? false;
  state.status = status;
  renderStatus();
  if (status.connected && (!wasConnected || !state.snapshot)) {
    await loadConfig();
  } else if (!status.connected && wasConnected) {
    renderContent();
  }
}

async function loadConfig(): Promise<void> {
  try {
    state.snapshot = await backend.getConfig();
    if (state.draft.size === 0) state.invalid.clear();
  } catch (err) {
    state.message = { kind: "error", text: String(err) };
  }
  renderContent();
  renderFooter();
}

// ---- Tabs ----------------------------------------------------------------

function renderTabs(): void {
  const tabs: [Tab, string][] = [
    ["p1", t("tab.player", { player: 1 })],
    ["p2", t("tab.player", { player: 2 })],
    ["general", t("tab.general")],
  ];
  $("tabs").replaceChildren(
    ...tabs.map(([id, label]) =>
      h(
        "button",
        {
          role: "tab",
          type: "button",
          "aria-selected": String(state.tab === id),
          class: state.tab === id ? "active" : undefined,
          onclick: () => {
            state.tab = id;
            renderTabs();
            renderContent();
          },
        },
        label,
      ),
    ),
  );
}

function renderContent(): void {
  const content = $("content");
  const status = state.status;
  const banner: HTMLElement[] = [];
  if (status && !status.connected) {
    banner.push(h("p", { class: "banner down" }, t("service.disconnectedHelp", { endpoint: status.endpoint })));
  } else if (status?.api != null && status.api !== status.apiSupported) {
    banner.push(
      h("p", { class: "banner warn" }, t("service.apiMismatch", { expected: status.apiSupported, actual: status.api })),
    );
  }
  const disabled = !status?.connected || !state.snapshot;
  const body =
    state.tab === "general" ? generalTab() : playerTab(state.tab === "p1" ? 1 : 2);
  content.replaceChildren(...banner, h("fieldset", { class: "form", disabled }, ...body));
}

function playerTab(player: Player): HTMLElement[] {
  const nodes: HTMLElement[] = [];
  if (player === 2) nodes.push(h("p", { class: "muted" }, t("player2.inherit")));
  const [mode, ...rest] = PLAYER_SECTIONS;
  nodes.push(section(mode, player));
  const joystick = currentValue("Joystick", player, "0") === "1";
  nodes.push(buttonsSection("mouse", player, !joystick), buttonsSection("joystick", player, joystick));
  nodes.push(...rest.map((s) => section(s, player)));
  return nodes;
}

function generalTab(): HTMLElement[] {
  const autostart = h("input", {
    type: "checkbox",
    id: "autostart",
    checked: state.autostart === true,
    onchange: async (e: Event) => {
      const input = e.target as HTMLInputElement;
      try {
        await backend.setAutostart(input.checked);
        state.autostart = input.checked;
      } catch (err) {
        input.checked = !input.checked;
        state.message = { kind: "error", text: String(err) };
        renderFooter();
      }
    },
  });
  // The autostart toggle does not need the driver.
  const app = h(
    "div",
    { class: "card" },
    h("div", { class: "row" }, h("label", { for: "autostart" }, t("general.autostart")), autostart),
    h("div", { class: "row" }, h("span", {}, t("general.configFile")), h("code", {}, state.snapshot?.path ?? "—")),
    h("div", { class: "row" }, h("span", {}, t("general.endpoint")), h("code", {}, state.status?.endpoint ?? "—")),
    h("p", { class: "muted" }, t("general.about")),
  );
  return [...GLOBAL_SECTIONS.map((s) => section(s, null)), app];
}

// ---- Fields --------------------------------------------------------------

function currentValue(key: string, player: Player | null, def: string): string {
  const stored = player === null ? key : playerKey(key, player);
  return state.draft.get(stored) ?? effectiveValue(state.snapshot?.values ?? {}, key, player, def);
}

function section(def: Section, player: Player | null): HTMLElement {
  return h(
    "section",
    { class: "card" },
    h("h2", {}, t(def.title)),
    ...def.fields.map((field) => fieldRow(field, player)),
  );
}

/** Wires a control to the draft: records edits, flags invalid values. */
function bind(row: HTMLElement, key: string, shown: string, valid: (v: string) => boolean) {
  return (value: string) => {
    state.draft.set(key, shown, value);
    row.classList.toggle("dirty", state.draft.get(key) !== undefined);
    const ok = valid(value);
    row.classList.toggle("invalid", !ok);
    if (ok) state.invalid.delete(key);
    else state.invalid.add(key);
    state.message = null;
    renderFooter();
  };
}

function fieldRow(field: Field, player: Player | null): HTMLElement {
  const key = player === null ? field.key : playerKey(field.key, player);
  const shown = effectiveValue(state.snapshot?.values ?? {}, field.key, player, field.def);
  const value = state.draft.get(key) ?? shown;
  const id = `f-${key}`;
  const row = h("div", { class: "row" });
  row.classList.toggle("dirty", state.draft.get(key) !== undefined);
  row.classList.toggle("invalid", state.invalid.has(key));
  const record = bind(row, key, shown, (v) => isValid(field, v));
  // The mode decides which button map is the active one.
  const change =
    field.key === "Joystick"
      ? (v: string) => {
          record(v);
          renderContent();
        }
      : record;
  const hint = field.def === "" ? t("value.empty") : field.def;
  row.append(
    h(
      "label",
      { for: id, title: `${key} — ${t("value.default", { value: hint })}` },
      t(field.label),
      field.help ? h("small", {}, t(field.help)) : null,
    ),
    control(field, id, value, change),
  );
  return row;
}

function control(field: Field, id: string, value: string, change: (v: string) => void): HTMLElement {
  const onInput = (e: Event) => change((e.target as HTMLInputElement).value);
  switch (field.kind) {
    case "bool":
      return h("input", {
        id,
        type: "checkbox",
        checked: value === "1",
        onchange: (e: Event) => change((e.target as HTMLInputElement).checked ? "1" : "0"),
      });
    case "select": {
      const options = field.options.map((o) => h("option", { value: o.value, selected: o.value === value }, t(o.label)));
      if (!field.options.some((o) => o.value === value)) options.push(h("option", { value, selected: true }, value));
      return h("select", { id, onchange: onInput }, ...options);
    }
    case "number":
      return h("input", {
        id,
        type: "number",
        value,
        min: field.min,
        max: field.max,
        step: field.step ?? 1,
        placeholder: field.optional ? t("value.empty") : undefined,
        oninput: onInput,
      });
    case "text":
      return h("input", { id, type: "text", value, placeholder: field.placeholder, oninput: onInput, spellcheck: "false" });
    case "colour": {
      const text = h("input", { id, type: "text", value, class: "short", oninput: onInput, spellcheck: "false" });
      const picker = h("input", {
        type: "color",
        value: bgrToHex(value) ?? "#ffffff",
        oninput: (e: Event) => {
          const bgr = hexToBgr((e.target as HTMLInputElement).value);
          text.value = bgr;
          change(bgr);
        },
      });
      return h("span", { class: "pair" }, picker, text);
    }
  }
}

// ---- Buttons -------------------------------------------------------------

function buttonsSection(mode: ButtonMode, player: Player, active: boolean): HTMLElement {
  const title = t(mode === "mouse" ? "section.buttonsMouse" : "section.buttonsJoystick");
  const head = h(
    "tr",
    {},
    h("th", {}, t("buttons.button")),
    h("th", {}, t("buttons.onScreen")),
    h("th", {}, t("buttons.modifier")),
    h("th", {}, t("buttons.offScreen")),
    h("th", {}, t("buttons.modifier")),
  );
  const rows = BUTTONS.map((button) =>
    h(
      "tr",
      {},
      h("th", { scope: "row" }, t(`button.${button}` as MsgKey)),
      ...(["action", "mod", "offscreen", "offscreenMod"] as ButtonSlot[]).map((slot) =>
        h("td", {}, buttonCell(mode, button, slot, player)),
      ),
    ),
  );
  return h(
    "details",
    { class: "card", open: active },
    h("summary", {}, h("h2", {}, title, active ? h("span", { class: "badge" }, t("buttons.active")) : null)),
    h("p", { class: "muted" }, t("buttons.handedNote")),
    h("div", { class: "scroll" }, h("table", { class: "buttons" }, h("thead", {}, head), h("tbody", {}, ...rows))),
  );
}

function buttonCell(mode: ButtonMode, button: (typeof BUTTONS)[number], slot: ButtonSlot, player: Player): HTMLElement {
  const base = buttonKey(mode, button, slot);
  const key = playerKey(base, player);
  const def = buttonDefault(mode, button, slot);
  const shown = effectiveValue(state.snapshot?.values ?? {}, base, player, def);
  const value = state.draft.get(key) ?? shown;
  const cell = h("span", { class: "cell", title: `${key} — ${t("value.default", { value: def })}` });
  cell.classList.toggle("dirty", state.draft.get(key) !== undefined);
  const validCode = (v: string) => /^\d{1,3}$/.test(v) && Number(v) <= 255;
  const change = bind(cell, key, shown, validCode);
  const number = h("input", { type: "number", min: 0, max: 255, value, class: "code", "aria-label": key });
  if (slot === "mod" || slot === "offscreenMod") {
    number.addEventListener("input", () => change(number.value));
    cell.append(number);
    return cell;
  }
  const select = h("select", { "aria-label": key });
  const fill = (code: string) => {
    const options = KNOWN_ACTIONS.map((a) =>
      h("option", { value: String(a.code), selected: String(a.code) === code }, t(a.label, { arg: a.arg ?? "" })),
    );
    if (!KNOWN_ACTIONS.some((a) => String(a.code) === code)) {
      options.unshift(h("option", { value: code, selected: true }, t("action.code", { code })));
    }
    select.replaceChildren(...options);
  };
  fill(value);
  select.addEventListener("change", () => {
    number.value = select.value;
    change(select.value);
  });
  number.addEventListener("input", () => {
    fill(number.value);
    change(number.value);
  });
  cell.append(select, number);
  return cell;
}

// ---- Save / revert -------------------------------------------------------

function renderFooter(): void {
  const footer = $("footer");
  const pending = state.draft.size;
  const message = state.message
    ? h("span", { class: `message ${state.message.kind}`, role: "status" }, state.message.text)
    : state.invalid.size
      ? h("span", { class: "message error" }, t("actions.invalid"))
      : pending
        ? h("span", { class: "message" }, t("actions.pending", { count: pending }))
        : h("span", {});
  const canSave = pending > 0 && state.invalid.size === 0 && !state.saving && state.status?.connected === true;
  footer.replaceChildren(
    message,
    h("span", { class: "spacer" }),
    h("button", { type: "button", disabled: pending === 0 || state.saving, onclick: revert }, t("actions.revert")),
    h("button", { type: "button", class: "primary", disabled: !canSave, onclick: save }, t("actions.save")),
  );
}

function revert(): void {
  state.draft.clear();
  state.invalid.clear();
  state.message = null;
  renderContent();
  renderFooter();
}

async function save(): Promise<void> {
  state.saving = true;
  renderFooter();
  try {
    state.snapshot = await backend.setConfig(state.draft.changes());
    state.draft.clear();
    state.invalid.clear();
    state.message = { kind: "ok", text: t("actions.saved") };
    renderContent();
  } catch (err) {
    state.message = { kind: "error", text: t("actions.error", { error: String(err) }) };
  } finally {
    state.saving = false;
    renderFooter();
  }
}

// ---- Start ---------------------------------------------------------------

async function start(): Promise<void> {
  applyLanguage();
  renderShell();
  await backend.onStatus((status) => void onStatus(status));
  backend
    .getAutostart()
    .then((enabled) => {
      state.autostart = enabled;
      if (state.tab === "general") renderContent();
    })
    .catch(() => {});
  await onStatus(await backend.getStatus());
}

start().catch((err) => {
  document.body.textContent = String(err);
});
