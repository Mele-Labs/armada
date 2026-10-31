// What a Job browser's page is given when it asks: the saved choice, the default beneath it, and never while the agent drives.

import { describe, expect, it } from "vitest";

import { NO_SETTINGS, type Answer, type Choice, type SitePermission, type SiteSettings } from "../../shared/site-permissions";
import { check, decide, downloadOf, hostOf, permissionsOf, type Rules } from "./site-permissions";

const HOST = "login.example.com";

function rules(over: { driving?: boolean; site?: Partial<Record<SitePermission, Choice>>; defaults?: Partial<SiteSettings["defaults"]>; answer?: Answer } = {}) {
  const asked: SitePermission[] = [];
  const kept: [string, SitePermission, Choice][] = [];
  const made: Rules = {
    settings: () => ({ ...NO_SETTINGS, sites: { [HOST]: over.site ?? {} }, defaults: { ...NO_SETTINGS.defaults, ...over.defaults } }),
    driving: () => over.driving ?? true,
    ask: async (permission) => {
      asked.push(permission);
      return over.answer ?? "dismiss";
    },
    remember: (host, permission, choice) => void kept.push([host, permission, choice]),
  };
  return { made, asked, kept };
}

describe("which permissions a request names", () => {
  it("maps what Electron sends and refuses the rest", () => {
    expect(permissionsOf("local-network-access", {})).toEqual(["local_network"]);
    expect(permissionsOf("media", { mediaTypes: ["video"] })).toEqual(["camera"]);
    expect(permissionsOf("media", { mediaTypes: ["audio", "video"] })).toEqual(["camera", "microphone"]);
    expect(permissionsOf("media", {})).toEqual(["camera", "microphone"]);
    expect(permissionsOf("geolocation", {})).toEqual(["location"]);
    expect(permissionsOf("clipboard-read", {})).toEqual(["clipboard"]);
    expect(permissionsOf("notifications", {})).toEqual(["notifications"]);
    expect(permissionsOf("midi", {})).toBeNull();
    expect(permissionsOf("fullscreen", {})).toBeNull();
  });
});

describe("a request while the agent drives", () => {
  it("is refused whatever was saved, and never asks", async () => {
    const one = rules({ driving: false, site: { local_network: "allow" } });
    expect(await decide(["local_network"], HOST, one.made)).toBe(false);
    expect(check(["local_network"], HOST, one.made)).toBe(false);
    expect(downloadOf(HOST, rules({ driving: false, site: { downloads: "allow" } }).made)).toBe("block");
    expect(one.asked).toEqual([]);
  });
});

describe("a request while the owner drives", () => {
  it("is given where the site's choice is Allow, and refused where it is Block", async () => {
    expect(await decide(["camera"], HOST, rules({ site: { camera: "allow" } }).made)).toBe(true);
    expect(await decide(["camera"], HOST, rules({ site: { camera: "block" } }).made)).toBe(false);
  });

  it("falls to the default where the site has no choice, and a site's own choice beats it", async () => {
    expect(await decide(["location"], HOST, rules({ defaults: { location: "allow" } }).made)).toBe(true);
    expect(await decide(["location"], HOST, rules({ defaults: { location: "allow" }, site: { location: "block" } }).made)).toBe(false);
  });

  it("asks where the choice is Ask, and keeps the answer", async () => {
    const yes = rules({ answer: "allow" });
    expect(await decide(["local_network"], HOST, yes.made)).toBe(true);
    expect(yes.asked).toEqual(["local_network"]);
    expect(yes.kept).toEqual([[HOST, "local_network", "allow"]]);
    const no = rules({ answer: "block" });
    expect(await decide(["local_network"], HOST, no.made)).toBe(false);
    expect(no.kept).toEqual([[HOST, "local_network", "block"]]);
  });

  it("refuses a dismissed question and keeps nothing", async () => {
    const one = rules({ answer: "dismiss" });
    expect(await decide(["local_network"], HOST, one.made)).toBe(false);
    expect(one.kept).toEqual([]);
  });

  it("refuses an answer that arrives after the page was handed back, and keeps nothing", async () => {
    let driving = true;
    const kept: unknown[] = [];
    const one: Rules = {
      settings: () => NO_SETTINGS,
      driving: () => driving,
      ask: async () => {
        driving = false;
        return "allow";
      },
      remember: (...args) => void kept.push(args),
    };
    expect(await decide(["local_network"], HOST, one)).toBe(false);
    expect(kept).toEqual([]);
  });

  it("gives camera and microphone together only when both are", async () => {
    expect(await decide(["camera", "microphone"], HOST, rules({ site: { camera: "allow", microphone: "allow" } }).made)).toBe(true);
    expect(await decide(["camera", "microphone"], HOST, rules({ site: { camera: "allow", microphone: "block" } }).made)).toBe(false);
  });

  it("answers the synchronous check from what is already allowed, and never asks", () => {
    const one = rules({ site: { local_network: "allow" } });
    expect(check(["local_network"], HOST, one.made)).toBe(true);
    expect(check(["camera"], HOST, one.made)).toBe(false);
    expect(one.asked).toEqual([]);
  });

  it("reads a download from its own choice, blocked until chosen", () => {
    expect(downloadOf(HOST, rules().made)).toBe("block");
    expect(downloadOf(HOST, rules({ site: { downloads: "allow" } }).made)).toBe("allow");
    expect(downloadOf(HOST, rules({ site: { downloads: "ask" } }).made)).toBe("ask");
  });

  it("keeps a choice for the host it was made on", async () => {
    const one: Rules = { ...rules().made, settings: () => ({ ...NO_SETTINGS, sites: { [HOST]: { camera: "allow" } } }) };
    expect(await decide(["camera"], "other.example.com", one)).toBe(false);
  });
});

describe("the host of an address", () => {
  it("is the web's, with its port, and null for anything else", () => {
    expect(hostOf("https://login.example.com/signin")).toBe("login.example.com");
    expect(hostOf("http://localhost:5173/checkout")).toBe("localhost:5173");
    expect(hostOf("about:blank")).toBeNull();
    expect(hostOf("file:///etc/hosts")).toBeNull();
    expect(hostOf("not an address")).toBeNull();
  });
});
