// What a Job browser's page is given when it asks: the saved choice, the default beneath it, and the person's answer where it is Ask.

import { describe, expect, it } from "vitest";

import {
  NO_SETTINGS,
  type Answer,
  type Choice,
  type SitePermission,
  type SiteSettings,
} from "../../shared/site-permissions";
import {
  check,
  decide,
  downloadOf,
  hostOf,
  permissionsOf,
  popupOf,
  type Rules,
} from "./site-permissions";

const HOST = "login.example.com";

function rules(
  over: {
    site?: Partial<Record<SitePermission, Choice>>;
    defaults?: Partial<SiteSettings["defaults"]>;
    answer?: Answer;
  } = {},
) {
  const asked: SitePermission[] = [];
  const kept: [
    string,
    SitePermission,
    Choice,
  ][] = [];
  const made: Rules = {
    settings: () => ({
      ...NO_SETTINGS,
      sites: {
        [HOST]: over.site ?? {},
      },
      defaults: {
        ...NO_SETTINGS.defaults,
        ...over.defaults,
      },
    }),
    ask: async (permission) => {
      asked.push(permission);
      return over.answer ?? "dismiss";
    },
    remember: (host, permission, choice) =>
      void kept.push([
        host,
        permission,
        choice,
      ]),
  };
  return {
    made,
    asked,
    kept,
  };
}

describe("which permissions a request names", () => {
  it("maps what Electron sends and refuses the rest", () => {
    expect(permissionsOf("local-network-access", {})).toEqual([
      "local_network",
    ]);
    expect(
      permissionsOf("media", {
        mediaTypes: [
          "video",
        ],
      }),
    ).toEqual([
      "camera",
    ]);
    expect(
      permissionsOf("media", {
        mediaTypes: [
          "audio",
          "video",
        ],
      }),
    ).toEqual([
      "camera",
      "microphone",
    ]);
    expect(permissionsOf("media", {})).toEqual([
      "camera",
      "microphone",
    ]);
    expect(permissionsOf("geolocation", {})).toEqual([
      "location",
    ]);
    expect(permissionsOf("clipboard-read", {})).toEqual([
      "clipboard",
    ]);
    expect(permissionsOf("notifications", {})).toEqual([
      "notifications",
    ]);
    expect(permissionsOf("midi", {})).toBeNull();
  });

  it("reads Electron's fullscreen and pointer lock as the one Fullscreen row", () => {
    expect(permissionsOf("fullscreen", {})).toEqual([
      "fullscreen",
    ]);
    expect(permissionsOf("pointerLock", {})).toEqual([
      "fullscreen",
    ]);
  });
});

describe("fullscreen and pointer lock", () => {
  it("are given by default, and never asked", async () => {
    const one = rules();
    expect(
      await decide(
        [
          "fullscreen",
        ],
        HOST,
        one.made,
      ),
    ).toBe(true);
    expect(
      check(
        [
          "fullscreen",
        ],
        HOST,
        one.made,
      ),
    ).toBe(true);
    expect(one.asked).toEqual([]);
  });

  it("are refused where the site's choice is Block, and asked where it is Ask", async () => {
    expect(
      await decide(
        [
          "fullscreen",
        ],
        HOST,
        rules({
          site: {
            fullscreen: "block",
          },
        }).made,
      ),
    ).toBe(false);
    const ask = rules({
      defaults: {
        fullscreen: "ask",
      },
      answer: "allow",
    });
    expect(
      await decide(
        [
          "fullscreen",
        ],
        HOST,
        ask.made,
      ),
    ).toBe(true);
    expect(ask.asked).toEqual([
      "fullscreen",
    ]);
    expect(
      check(
        [
          "fullscreen",
        ],
        HOST,
        ask.made,
      ),
    ).toBe(false);
  });
});

describe("a page opening a popup", () => {
  it("is allowed by default, blocked or asked as the host's own choice or the default says", () => {
    expect(popupOf(HOST, rules().made)).toBe("allow");
    expect(
      popupOf(
        HOST,
        rules({
          site: {
            popups: "block",
          },
        }).made,
      ),
    ).toBe("block");
    expect(
      popupOf(
        HOST,
        rules({
          defaults: {
            popups: "ask",
          },
        }).made,
      ),
    ).toBe("ask");
    expect(
      popupOf(
        HOST,
        rules({
          defaults: {
            popups: "block",
          },
          site: {
            popups: "allow",
          },
        }).made,
      ),
    ).toBe("allow");
  });
});

describe("a request from a page", () => {
  it("is given where the site's choice is Allow, and refused where it is Block", async () => {
    expect(
      await decide(
        [
          "camera",
        ],
        HOST,
        rules({
          site: {
            camera: "allow",
          },
        }).made,
      ),
    ).toBe(true);
    expect(
      await decide(
        [
          "camera",
        ],
        HOST,
        rules({
          site: {
            camera: "block",
          },
        }).made,
      ),
    ).toBe(false);
  });

  it("falls to the default where the site has no choice, and a site's own choice beats it", async () => {
    expect(
      await decide(
        [
          "location",
        ],
        HOST,
        rules({
          defaults: {
            location: "allow",
          },
        }).made,
      ),
    ).toBe(true);
    expect(
      await decide(
        [
          "location",
        ],
        HOST,
        rules({
          defaults: {
            location: "allow",
          },
          site: {
            location: "block",
          },
        }).made,
      ),
    ).toBe(false);
  });

  it("asks where the choice is Ask, and keeps the answer", async () => {
    const yes = rules({
      answer: "allow",
    });
    expect(
      await decide(
        [
          "local_network",
        ],
        HOST,
        yes.made,
      ),
    ).toBe(true);
    expect(yes.asked).toEqual([
      "local_network",
    ]);
    expect(yes.kept).toEqual([
      [
        HOST,
        "local_network",
        "allow",
      ],
    ]);
    const no = rules({
      answer: "block",
    });
    expect(
      await decide(
        [
          "local_network",
        ],
        HOST,
        no.made,
      ),
    ).toBe(false);
    expect(no.kept).toEqual([
      [
        HOST,
        "local_network",
        "block",
      ],
    ]);
  });

  it("refuses a dismissed question and keeps nothing", async () => {
    const one = rules({
      answer: "dismiss",
    });
    expect(
      await decide(
        [
          "local_network",
        ],
        HOST,
        one.made,
      ),
    ).toBe(false);
    expect(one.kept).toEqual([]);
  });

  it("gives camera and microphone together only when both are", async () => {
    expect(
      await decide(
        [
          "camera",
          "microphone",
        ],
        HOST,
        rules({
          site: {
            camera: "allow",
            microphone: "allow",
          },
        }).made,
      ),
    ).toBe(true);
    expect(
      await decide(
        [
          "camera",
          "microphone",
        ],
        HOST,
        rules({
          site: {
            camera: "allow",
            microphone: "block",
          },
        }).made,
      ),
    ).toBe(false);
  });

  it("answers the synchronous check from what is already allowed, and never asks", () => {
    const one = rules({
      site: {
        local_network: "allow",
      },
    });
    expect(
      check(
        [
          "local_network",
        ],
        HOST,
        one.made,
      ),
    ).toBe(true);
    expect(
      check(
        [
          "camera",
        ],
        HOST,
        one.made,
      ),
    ).toBe(false);
    expect(one.asked).toEqual([]);
  });

  it("reads a download from its own choice, asked until chosen", () => {
    expect(downloadOf(HOST, rules().made)).toBe("ask");
    expect(
      downloadOf(
        HOST,
        rules({
          site: {
            downloads: "allow",
          },
        }).made,
      ),
    ).toBe("allow");
    expect(
      downloadOf(
        HOST,
        rules({
          site: {
            downloads: "ask",
          },
        }).made,
      ),
    ).toBe("ask");
  });

  it("keeps a choice for the host it was made on", async () => {
    const one: Rules = {
      ...rules().made,
      settings: () => ({
        ...NO_SETTINGS,
        sites: {
          [HOST]: {
            camera: "allow",
          },
        },
      }),
    };
    expect(
      await decide(
        [
          "camera",
        ],
        "other.example.com",
        one,
      ),
    ).toBe(false);
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
