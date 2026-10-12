// The settings main acts on itself follow every published list. `settings.ts`.

import { describe, expect, it } from "vitest";

import type { Setting, SettingsList } from "@armada/protocol";
import * as request from "./request";
import { publishSettings, textSetting } from "./settings";

function setting(key: string, value: Setting["value"]): Setting {
  return {
    key,
    group: "Fleet",
    section: "Timeouts",
    title: key,
    description: "",
    kind: { kind: "integer", min: 0, max: 100000 },
    default: value,
    value,
    saved: null,
    applies: "live",
    pending_restart: false,
    overridden_by_env: null,
  };
}

const list = (...settings: Setting[]): SettingsList => ({ path: "/m/settings.json", refused: null, settings });

describe("publishSettings", () => {
  it("publishes the list, and the command wait follows timeouts.commandSeconds plus five seconds", () => {
    expect(request.COMMAND_MS).toBe(20_000);
    const published: SettingsList[] = [];
    publishSettings(list(setting("timeouts.commandSeconds", 30)), (change) => published.push(change.settings));
    expect(published).toHaveLength(1);
    // A live binding: the importer reads the value now in force.
    expect(request.COMMAND_MS).toBe(35_000);
    publishSettings(list(setting("timeouts.commandSeconds", 15)), () => {});
    expect(request.COMMAND_MS).toBe(20_000);
  });

  it("leaves the wait as it was on a list without the key", () => {
    publishSettings(list(), () => {});
    expect(request.COMMAND_MS).toBe(20_000);
  });
});

describe("textSetting", () => {
  it("reads a text value, and empty for anything else", () => {
    const held = list(setting("editor.command", "code -g {file}:{line}"), setting("timeouts.checkSeconds", 900));
    expect(textSetting(held, "editor.command")).toBe("code -g {file}:{line}");
    expect(textSetting(held, "timeouts.checkSeconds")).toBe("");
    expect(textSetting(null, "editor.command")).toBe("");
  });
});
