// The defaults for every Job's browser, where main keeps them: read when the document wires, followed as main says they change,
// and set with no Job and no tab. The mock plays the same set itself.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { JobBrowserState } from "../../../shared/api/job-browser";
import { NO_SETTINGS, type SiteDefaults } from "../../../shared/site-permissions";
import { peekJobBrowser, setBrowserDefault, wireJobBrowsers } from "./store";

vi.mock("../job-panes/store", () => ({
  jobOpens: vi.fn(),
  look: vi.fn(),
  peek: vi.fn(),
}));
vi.mock("../cockpit/viewed", () => ({
  viewingNow: vi.fn(() => false),
}));

const state: JobBrowserState = {
  sessionId: "job",
  tabId: "t1",
  url: "http://localhost:5173",
  title: "",
  back: false,
  forward: false,
  windowed: false,
  site: NO_SETTINGS,
};

let changed: (state: JobBrowserState) => void;
let defaultsChanged: (defaults: SiteDefaults) => void;
const BLOCKED: SiteDefaults = {
  ...NO_SETTINGS.defaults,
  popups: "block",
};
const api = {
  list: async () => [],
  onChanged: (listener: (state: JobBrowserState) => void) => {
    changed = listener;
    return () => undefined;
  },
  onShow: () => () => undefined,
  tabs: vi.fn(async () => undefined),
  defaults: vi.fn(async () => BLOCKED),
  onDefaultsChanged: (listener: (defaults: SiteDefaults) => void) => {
    defaultsChanged = listener;
    return () => undefined;
  },
  setDefault: vi.fn(async () => undefined),
};

let unwire: () => void;
beforeEach(() => {
  api.setDefault.mockClear();
  unwire = wireJobBrowsers(api as unknown as Parameters<typeof wireJobBrowsers>[0]);
});
afterEach(() => unwire());

test("the one set main keeps is read at wiring and is what every page's site settings carry", async () => {
  changed(state);
  await Promise.resolve();
  await Promise.resolve();
  expect(peekJobBrowser("job", "t1")?.site.defaults.popups).toBe("block");
});

test("a change main announces reaches the pages already drawn", () => {
  changed(state);
  defaultsChanged({
    ...NO_SETTINGS.defaults,
    camera: "block",
  });
  expect(peekJobBrowser("job", "t1")?.site.defaults.camera).toBe("block");
  expect(peekJobBrowser("job", "t1")?.site.defaults.popups).toBe("allow");
});

test("setting a default names the permission and the choice and nothing else, and waits for main to say it changed", () => {
  changed(state);
  setBrowserDefault("fullscreen", "block");
  expect(api.setDefault).toHaveBeenCalledWith("fullscreen", "block");
  expect(peekJobBrowser("job", "t1")?.site.defaults.fullscreen).toBe("allow");
});
