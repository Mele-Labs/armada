// A page's ask sits above the page and leaves it live: nothing draws over a native view, so an ask drawn over one would freeze it.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { NO_SETTINGS } from "../../../shared/site-permissions";
import { JobBrowserPane } from "./JobBrowserPane";
import { wireJobBrowsers, type JobBrowser } from "./store";

const freeze = vi.fn(async () => null);
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
  thaw: async () => undefined,
} as unknown as Parameters<typeof wireJobBrowsers>[0];

const asking: JobBrowser = {
  url: "http://192.168.68.98:5481/",
  title: "A page",
  back: false,
  forward: false,
  loads: 0,
  windowed: false,
  pins: [],
  site: {
    ...NO_SETTINGS,
    pending: {
      permission: "local_network",
      host: "192.168.68.98:5481",
    },
  },
};

let root: Root;
let host: HTMLElement;
let unwire: () => void;

beforeEach(async () => {
  freeze.mockClear();
  unwire = wireJobBrowsers(api);
  host = document.createElement("div");
  host.style.cssText = "width: 800px; height: 600px; display: flex;";
  document.body.append(host);
  root = createRoot(host);
  act(() =>
    root.render(<JobBrowserPane sessionId="job" tabId="tab" browser={asking} placed="pane" />),
  );
  // The pane reads what is drawn over it once a frame, so a hold would start after one.
  await act(async () => {
    await new Promise((frame) => requestAnimationFrame(frame));
    await new Promise((frame) => requestAnimationFrame(frame));
  });
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
  unwire();
});

const strip = () => document.querySelector('[role="alertdialog"][aria-label="Site request"]')!;

test("an open ask does not freeze the page", () => {
  expect(strip()).not.toBeNull();
  expect(freeze).not.toHaveBeenCalled();
});

test("the ask sits between the toolbar and the page, covering neither", () => {
  const head = document.querySelector(".job-browser__head")!.getBoundingClientRect();
  const ask = strip().getBoundingClientRect();
  const page = document.querySelector(".job-browser__page")!.getBoundingClientRect();
  expect(ask.top).toBeGreaterThanOrEqual(head.bottom);
  expect(page.top).toBeGreaterThanOrEqual(ask.bottom);
});

test("the ask still names the site, its risk and both answers", () => {
  expect(strip().textContent).toContain("192.168.68.98:5481 wants to use Local network");
  expect(strip().textContent).toContain(
    "It can reach apps on this Mac and devices on your network.",
  );
  const named = [
    ...strip().querySelectorAll("button"),
  ].map((one) => one.textContent);
  expect(named).toEqual([
    "Block",
    "Allow",
  ]);
});
