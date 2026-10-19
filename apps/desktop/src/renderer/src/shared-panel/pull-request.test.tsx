// A pull request pressed opens its panel at once, before the forge's read answers; a read that answers nothing leaves the panel
// open and saying so; and its own Open in browser goes to the browser, never back into this panel. Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { PullRequestPage } from "@armada/protocol";

import { SharedPanelHost } from "./host";
import { SharedPanelFrom } from "./reads";
import type { PanelReads, PullRequestRead } from "./reads";
import { forgetPanel, openInPanel, usePanel } from "./store";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ADDRESS = "https://forge.example/harbor-labs/tidewater/pull/418";
const ITEM = { kind: "pr", number: 418, address: ADDRESS, title: "Keep the tide tables a day", branch: "ines/tide-cache" } as const;

const PAGE: PullRequestPage = { manifest_id: "tidewater", number: 418, base: "main", additions: 12, deletions: 3, mergeable: "mergeable", review: "none", approvals: 0, conversation: [], checks: [], files: [], commits: [] };
const READ: PullRequestRead = {
  repository: "harbor-labs/tidewater",
  facts: { kind: "pull_request", number: 418, title: "Keep the tide tables a day", branch: "ines/tide-cache", address: ADDRESS, checks: { state: "passed" }, state: "open", auto: false },
  page: PAGE,
};

const openLink = vi.fn(async (_address: string) => ({ ok: true }) as const);

const readsOf = (pullRequest: PanelReads["pullRequest"]): PanelReads => ({
  pullRequest,
  issue: async () => undefined,
  bay: async () => undefined,
  check: async () => undefined,
  lines: null,
  numbered: [],
});

/** A read held open until the test answers it. */
function held() {
  let answer: (read: PullRequestRead | undefined) => void = () => {};
  const read = new Promise<PullRequestRead | undefined>((resolve) => (answer = resolve));
  return { read: () => read, answer: (value: PullRequestRead | undefined) => act(async () => answer(value)) };
}

let opened: ReturnType<typeof usePanel>["open"];
function Watch() {
  opened = usePanel().open;
  return null;
}

let root: Root | undefined;
let host: HTMLElement | undefined;
async function mount(reads: PanelReads) {
  host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
  await act(async () =>
    root!.render(
      <SharedPanelFrom reads={reads}>
        <Watch />
        <SharedPanelHost />
      </SharedPanelFrom>,
    ),
  );
  await act(async () => void openInPanel(ITEM));
}

const named = (name: string) => document.querySelector(`[role="dialog"][aria-label="${name}"]`);
const status = () => document.querySelector('[role="status"][aria-label="Reading pull request #418"]');
const browserButton = () => [...document.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.getAttribute("aria-label") === "Open in browser");

beforeEach(() => {
  openLink.mockClear();
  (window as unknown as { armada: unknown }).armada = { openLink };
});

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  root = undefined;
  forgetPanel();
});

test("the panel is drawn the moment it is pressed, with stand-ins, and the filled panel replaces it when the read answers", async () => {
  const read = held();
  await mount(readsOf(read.read));
  const waiting = named("Pull request #418");
  expect(waiting).not.toBeNull();
  expect(status()).not.toBeNull();
  expect(document.body.textContent).toContain("Keep the tide tables a day");
  expect(document.querySelector('[role="tab"]')).toBeNull();

  await read.answer(READ);
  // The same dialog, filled: a new one would travel in again and take focus back to Close.
  expect(named("Pull request #418")).toBe(waiting);
  expect(status()).toBeNull();
  expect(document.querySelector('[role="tab"]')).not.toBeNull();
});

test("a read that answers nothing leaves the panel open and says so", async () => {
  const read = held();
  await mount(readsOf(read.read));
  await read.answer(undefined);
  expect(named("Pull request #418")).not.toBeNull();
  expect(status()).toBeNull();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe("The forge did not answer for this pull request.");
  expect(browserButton()).not.toBeUndefined();
});

test("a read that throws is a read that answered nothing", async () => {
  await mount(readsOf(() => Promise.reject(new Error("forge down"))));
  await act(async () => {});
  expect(document.querySelector('[role="alert"]')?.textContent).toBe("The forge did not answer for this pull request.");
});

test("Open in browser opens the pull request's address in the browser and leaves the panel where it is", async () => {
  await mount(readsOf(async () => READ));
  expect(status()).toBeNull();
  const was = opened;
  await act(async () => browserButton()!.click());
  expect(openLink).toHaveBeenCalledWith(ADDRESS);
  expect(opened).toBe(was);
});

test("Open in browser works while the read is still out", async () => {
  await mount(readsOf(held().read));
  await act(async () => browserButton()!.click());
  expect(openLink).toHaveBeenCalledWith(ADDRESS);
});

test("a browser that refuses is said in the panel", async () => {
  openLink.mockImplementationOnce(async (address: string) => ({ ok: false, why: "refused", address, detail: "no browser is set" }) as never);
  await mount(readsOf(async () => READ));
  await act(async () => browserButton()!.click());
  expect(document.querySelector('[role="alert"]')?.textContent).toBe(`This machine did not open ${ADDRESS}: no browser is set`);
});
