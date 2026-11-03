// The choices made for each site outlast the app, one repository's apart from another's, under one set of defaults for the whole Mac.

import { describe, expect, it } from "vitest";

import { NO_SETTINGS } from "../shared/site-permissions";
import { SiteSettingsStore } from "./site-permission-store";

function disk(first: string | null = null) {
  const held = {
    text: first,
  };
  return {
    held,
    io: {
      read: () => held.text,
      write: (text: string) => void (held.text = text),
    },
  };
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
    first.setDefault("camera", "block");
    const again = new SiteSettingsStore(disk(file.held.text).io).settingsOf("repo");
    expect(again.sites["login.example.com"]).toEqual({
      local_network: "allow",
    });
    expect(again.defaults.camera).toBe("block");
    expect(again.defaults.microphone).toBe("ask");
  });

  it("keep one repository's choices from another's", () => {
    const one = new SiteSettingsStore(disk().io);
    one.choose("repo-a", "login.example.com", "camera", "allow");
    expect(one.settingsOf("repo-b").sites).toEqual({});
  });

  it("keep one set of defaults for every repository", () => {
    const file = disk();
    const one = new SiteSettingsStore(file.io);
    one.setDefault("popups", "block");
    expect(one.settingsOf("repo-a").defaults.popups).toBe("block");
    expect(one.settingsOf("repo-b").defaults.popups).toBe("block");
    expect(one.defaults().popups).toBe("block");
    expect(new SiteSettingsStore(disk(file.held.text).io).defaults().popups).toBe("block");
  });

  it("ignore the defaults an older file kept for a repository, and keep its sites", () => {
    const old = JSON.stringify({
      repo: {
        sites: {
          "login.example.com": {
            camera: "allow",
          },
        },
        defaults: {
          camera: "block",
          local_network: "allow",
        },
      },
    });
    const one = new SiteSettingsStore(disk(old).io).settingsOf("repo");
    expect(one.defaults).toEqual(NO_SETTINGS.defaults);
    expect(one.defaults.popups).toBe("allow");
    expect(one.defaults.fullscreen).toBe("allow");
    expect(one.sites["login.example.com"]).toEqual({
      camera: "allow",
    });
  });

  it("read a saved default over the shipped ones, so a permission added since has its shipped value", () => {
    const file = JSON.stringify({
      defaults: {
        camera: "block",
        popups: "nonsense",
        invented: "allow",
      },
    });
    const kept = new SiteSettingsStore(disk(file).io).defaults();
    expect(kept.camera).toBe("block");
    expect(kept.popups).toBe("allow");
    expect(kept.fullscreen).toBe("allow");
    expect(kept).not.toHaveProperty("invented");
  });

  it("start again from the defaults when the file is corrupt", () => {
    expect(new SiteSettingsStore(disk("{not json").io).settingsOf("repo")).toEqual({
      sites: {},
      defaults: NO_SETTINGS.defaults,
    });
  });
});
