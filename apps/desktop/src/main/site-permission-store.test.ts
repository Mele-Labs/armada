// The choices made for each site outlast the app, one repository's apart from another's.

import { describe, expect, it } from "vitest";

import { NO_SETTINGS } from "../shared/site-permissions";
import { SiteSettingsStore } from "./site-permission-store";

function disk(first: string | null = null) {
  const held = { text: first };
  return { held, io: { read: () => held.text, write: (text: string) => void (held.text = text) } };
}

describe("kept site settings", () => {
  it("start from the defaults, downloads blocked", () => {
    const one = new SiteSettingsStore(disk().io).settingsOf("repo");
    expect(one.defaults).toEqual(NO_SETTINGS.defaults);
    expect(one.sites).toEqual({});
  });

  it("outlast the store: a choice and a default come back from the file", () => {
    const file = disk();
    const first = new SiteSettingsStore(file.io);
    first.choose("repo", "login.example.com", "local_network", "allow");
    first.setDefault("repo", "camera", "block");
    const again = new SiteSettingsStore(disk(file.held.text).io).settingsOf("repo");
    expect(again.sites["login.example.com"]).toEqual({ local_network: "allow" });
    expect(again.defaults.camera).toBe("block");
    expect(again.defaults.microphone).toBe("ask");
  });

  it("keep one repository's choices from another's", () => {
    const one = new SiteSettingsStore(disk().io);
    one.choose("repo-a", "login.example.com", "camera", "allow");
    expect(one.settingsOf("repo-b").sites).toEqual({});
  });

  it("start again from the defaults when the file is corrupt", () => {
    expect(new SiteSettingsStore(disk("{not json").io).settingsOf("repo")).toEqual({ sites: {}, defaults: NO_SETTINGS.defaults });
  });
});
