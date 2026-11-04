// A browser tab's page is a native view, painted over everything of Bridge's own. Whatever lies over its frame holds it, or the page stays painted over that surface.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { NO_SETTINGS } from "../../../shared/site-permissions";
import { JobBrowserPane } from "./JobBrowserPane";
import { wireJobBrowsers, type JobBrowser } from "./store";

const freeze = vi.fn(async () => null);
const thaw = vi.fn(async () => undefined);
const api = {
  list: async () => [],
  defaults: async () => NO_SETTINGS.defaults,
  onDefaultsChanged: () => () => undefined,
  onChanged: () => () => undefined,
  onShow: () => () => undefined,
  onCapture: () => () => undefined,
  place: async () => undefined,
  unplace: async () => undefined,
  freeze,
  thaw,
} as unknown as Parameters<typeof wireJobBrowsers>[0];

const browser: JobBrowser = {
  url: "http://localhost:5481/",
  title: "A page",
  back: false,
  forward: false,
  loads: 0,
  windowed: false,
  pins: [],
  site: NO_SETTINGS,
};

let root: Root;
let host: HTMLElement;
let unwire: () => void;

const frames = async () => {
  await act(async () => {
    await new Promise((frame) => requestAnimationFrame(frame));
    await new Promise((frame) => requestAnimationFrame(frame));
  });
};

beforeEach(async () => {
  freeze.mockClear();
  thaw.mockClear();
  unwire = wireJobBrowsers(api);
  host = document.createElement("div");
  host.style.cssText = "width: 800px; height: 600px; display: flex;";
  document.body.append(host);
  root = createRoot(host);
  act(() =>
    root.render(<JobBrowserPane sessionId="job" tabId="tab" browser={browser} placed="pane" />),
  );
  await frames();
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
  unwire();
});

test("a pane zoomed over the page holds it, and restored lets it go", async () => {
  expect(freeze).not.toHaveBeenCalled();
  // What `JobPanes` does to the pane zoomed: fixed, over the Job's area, and the page's pane stays where it was.
  const thread = document.createElement("div");
  thread.style.cssText = "position:fixed;left:0;top:0;width:100vw;height:100vh;z-index:20";
  document.body.append(thread);
  await frames();
  expect(freeze).toHaveBeenCalledWith("job", "tab");

  thread.remove();
  await frames();
  expect(thaw).toHaveBeenCalledWith("job", "tab");
});
