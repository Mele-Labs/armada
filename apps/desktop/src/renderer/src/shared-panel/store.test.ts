import { afterEach, expect, test, vi } from "vitest";

import { browsePullRequestsWith, closePanel, forgetPanel, hostsPanel, openAddressInPanel, openInPanel, openSlotKeyInPanel, panelBack, usePanel } from "./store";

const PR = { kind: "pr", number: 7, address: "https://github.com/mele-labs/armada/pull/7" } as const;
// `usePanel` is a hook; outside a component it is read straight from the store.
vi.mock("react", async (actual) => ({ ...(await actual<typeof import("react")>()), useSyncExternalStore: (_subscribe: unknown, get: () => ReturnType<typeof usePanel>) => get() }));
const peek = () => usePanel();

afterEach(() => forgetPanel());

test("with no window hosting it, nothing opens and the surface keeps its own way", () => {
  expect(openInPanel(PR)).toBe(false);
  expect(peek().open).toBeUndefined();
});

test("a hosted panel opens each kind, and goes when its last host does", () => {
  const leave = hostsPanel();
  expect(openInPanel(PR)).toBe(true);
  expect(peek().open).toEqual(PR);
  expect(openInPanel({ kind: "issue", number: "2214" })).toBe(true);
  expect(peek()).toMatchObject({ open: { kind: "issue", number: "2214" }, back: PR });
  panelBack();
  expect(peek().open).toEqual(PR);
  expect(openSlotKeyInPanel("slot:m1/3")).toBe(true);
  expect(peek().open).toEqual({ kind: "bay", manifestId: "m1", slot: 3 });
  expect(openInPanel({ kind: "check", id: "land:/code/armada:main:test" })).toBe(true);
  closePanel();
  expect(peek().open).toBeUndefined();
  openInPanel(PR);
  leave();
  expect(peek().open).toBeUndefined();
});

test("only a pull request's own address opens one", () => {
  const leave = hostsPanel();
  expect(openAddressInPanel("https://github.com/mele-labs/armada/pull/12")).toBe(true);
  expect(peek().open).toMatchObject({ kind: "pr", number: 12 });
  closePanel();
  expect(openAddressInPanel("https://github.com/mele-labs/armada/pull/12/files")).toBe(false);
  expect(openAddressInPanel("https://github.com/mele-labs/armada/issues/12")).toBe(false);
  expect(peek().open).toBeUndefined();
  leave();
});

test("where pull requests open in the browser, a pull request goes there and the panel stays shut; the rest still open", () => {
  const leave = hostsPanel();
  const browse = vi.fn(() => true);
  const stop = browsePullRequestsWith(browse);
  expect(openInPanel(PR)).toBe(true);
  expect(browse).toHaveBeenCalledWith(PR.address);
  expect(peek().open).toBeUndefined();
  expect(openInPanel({ kind: "issue", number: "2214" })).toBe(true);
  expect(peek().open).toMatchObject({ kind: "issue" });
  // A pull request named by number alone has no address to send, so it opens here.
  expect(openInPanel({ kind: "pr", number: 9, address: "" })).toBe(true);
  expect(peek().open).toMatchObject({ kind: "pr", number: 9 });
  stop();
  expect(browse).toHaveBeenCalledTimes(1);
  openInPanel(PR);
  expect(browse).toHaveBeenCalledTimes(1);
  leave();
});
