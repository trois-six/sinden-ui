import assert from "node:assert/strict";
import { test } from "node:test";

import { MESSAGES, resolveLang, setLang, t, type MsgKey } from "./i18n.ts";
import { GLOBAL_SECTIONS, KNOWN_ACTIONS, PLAYER_SECTIONS } from "./schema.ts";

test("French defines every English message, with the same placeholders", () => {
  const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  for (const key of Object.keys(MESSAGES.en) as MsgKey[]) {
    assert.ok(MESSAGES.fr[key], `missing fr: ${key}`);
    assert.deepEqual(placeholders(MESSAGES.fr[key]), placeholders(MESSAGES.en[key]), key);
  }
});

test("every label of the schema has a message", () => {
  const keys: MsgKey[] = [];
  for (const section of [...PLAYER_SECTIONS, ...GLOBAL_SECTIONS]) {
    keys.push(section.title);
    for (const field of section.fields) {
      keys.push(field.label);
      if (field.help) keys.push(field.help);
      if (field.kind === "select") keys.push(...field.options.map((o) => o.label));
    }
  }
  keys.push(...KNOWN_ACTIONS.map((a) => a.label));
  for (const key of keys) assert.ok(key in MESSAGES.en, key);
});

test("handedness in French is Droitier / Gaucher", () => {
  assert.equal(MESSAGES.fr["hand.right"], "Droitier");
  assert.equal(MESSAGES.fr["hand.left"], "Gaucher");
});

test("language resolution", () => {
  assert.equal(resolveLang("fr", ["en-US"]), "fr");
  assert.equal(resolveLang("auto", ["fr-CA", "en"]), "fr");
  assert.equal(resolveLang(null, ["de-DE", "en-GB"]), "en");
  assert.equal(resolveLang(null, ["de-DE"]), "en");
});

test("placeholders", () => {
  setLang("fr");
  assert.equal(t("gun.player", { player: 2 }), "Joueur 2");
  setLang("en");
  assert.equal(t("actions.pending", { count: 3 }), "3 unsaved change(s)");
  assert.equal(t("gun.player"), "Player {player}");
});
