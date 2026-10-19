// A pull request pressed opens its panel at once, before the forge's read answers; a read that answers nothing leaves the panel
// open and saying so; and its own Open in browser goes to the browser, never back into this panel. Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { PullRequestPage } from "@armada/protocol";

import { SharedPanelHost } from "./host";
import { SharedPanelFrom } from "./reads";
import type { PanelReads, PatchesRead, PullRequestRead } from "./reads";
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

test("reviewers the page names are a plate beside the panel, a row each with the team marked and a verdict glyph, and none draws no plate", async () => {
  const reviewers = [
    { login: "ines", kind: "person", verdict: "approved" },
    { login: "dockside", kind: "team" },
  ] as const;
  await mount(readsOf(async () => ({ ...READ, page: { ...PAGE, reviewers: [...reviewers] } })));
  const plate = document.querySelector('section[aria-label="Reviewers"]');
  expect(plate?.querySelectorAll("li")).toHaveLength(2);
  expect(plate?.textContent).toContain("ines");
  expect(plate?.textContent).toContain("Team");
  expect(plate?.querySelector('[role="img"][aria-label="Approved"]')).not.toBeNull();
  expect(plate?.querySelector('[role="img"][aria-label="Waiting"]')).not.toBeNull();
  act(() => root?.unmount());
  host?.remove();
  forgetPanel();
  await mount(readsOf(async () => ({ ...READ, page: { ...PAGE, reviewers: [] } })));
  expect(document.querySelector('section[aria-label="Reviewers"]')).toBeNull();
});

// ---- The Files tab: every file's diff, one under the other, and the list beside the panel ----------------------

const NO_PATCH = "The forge sent no patch for this file. It is binary, or the change is too large for the forge to show.";
const FILES = [
  { path: "src/tide/tables.rs", status: "modified", additions: 1, deletions: 1 },
  { path: "src/tide/cache.rs", status: "added", additions: 1, deletions: 0 },
  { path: "assets/chart.png", status: "added", additions: 0, deletions: 0 },
] as const;
const WITH_FILES: PullRequestRead = { ...READ, page: { ...PAGE, files: [...FILES] } };
const patch = (path: string, over: { patch?: string; said?: string }) => ({ manifest_id: "tidewater", number: 418, path, ...over });
const PATCHES: PatchesRead = {
  ok: true,
  files: [
    patch("src/tide/tables.rs", { patch: "@@ -1,2 +1,2 @@\n-let days = 1;\n+let days = 2;\n fn tide() {}" }),
    patch("src/tide/cache.rs", { patch: "@@ -0,0 +1 @@\n+pub struct Cache;" }),
    patch("assets/chart.png", { said: NO_PATCH }),
  ],
};

const tabNamed = (name: string) => [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((one) => one.textContent === name);
const press = (name: string) => act(async () => tabNamed(name)!.click());
const filesPanel = () => document.querySelector('[role="tabpanel"][aria-label="Files"]');
/** Each file's section in the stack, by its path: the scroll target a press in the list lands on. */
const sections = () => [...(filesPanel()?.querySelectorAll<HTMLElement>("section[aria-label]:not([role])") ?? [])];
const list = () => named("Files");

test("the Files tab reads every file's patch once, the first time it is shown, and keeps it", async () => {
  const pullRequestPatches = vi.fn(async () => PATCHES);
  await mount({ ...readsOf(async () => WITH_FILES), pullRequestPatches });
  expect(pullRequestPatches).not.toHaveBeenCalled();
  await press("Files");
  expect(pullRequestPatches).toHaveBeenCalledTimes(1);
  expect(pullRequestPatches).toHaveBeenCalledWith(ITEM);
  await press("Conversation");
  await press("Files");
  expect(pullRequestPatches).toHaveBeenCalledTimes(1);
});

test("every file's diff is drawn one under the other in the forge's order, and a file with no patch says the forge's sentence", async () => {
  await mount({ ...readsOf(async () => WITH_FILES), pullRequestPatches: async () => PATCHES });
  await press("Files");
  expect(sections().map((one) => one.getAttribute("aria-label"))).toEqual(["src/tide/tables.rs", "src/tide/cache.rs", "assets/chart.png"]);
  expect(sections()[0]!.textContent).toContain("+let days = 2;");
  expect(sections()[2]!.querySelector('[role="note"]')?.textContent).toBe(NO_PATCH);
});

test("a refused read of the patches says Fleet's words in the Files tab", async () => {
  await mount({ ...readsOf(async () => WITH_FILES), pullRequestPatches: async () => ({ ok: false, said: "The forge refused the read." }) });
  await press("Files");
  expect(filesPanel()?.querySelector('[role="alert"]')?.textContent).toBe("The forge refused the read.");
  expect(sections()).toEqual([]);
});

test("while the patches are read, each file's header is drawn with a stand-in under it", async () => {
  await mount({ ...readsOf(async () => WITH_FILES), pullRequestPatches: () => new Promise<PatchesRead>(() => {}) });
  await press("Files");
  expect(sections().map((one) => one.getAttribute("aria-label"))).toEqual(FILES.map((one) => one.path));
  expect(filesPanel()?.querySelector('[role="status"][aria-label="Reading src/tide/cache.rs"]')).not.toBeNull();
});

test("the file list floats beside the panel on the Files tab, a press scrolls to that file's diff and marks it, and leaving Files hides it", async () => {
  const scrolled = vi.spyOn(HTMLElement.prototype, "scrollIntoView").mockImplementation(() => {});
  await mount({ ...readsOf(async () => WITH_FILES), pullRequestPatches: async () => PATCHES });
  expect(list()).toBeNull();
  await press("Files");
  const row = () => [...list()!.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.textContent?.includes("src/tide/cache.rs"))!;
  expect(row().getAttribute("aria-current")).toBeNull();
  await act(async () => row().click());
  expect(scrolled.mock.contexts).toContain(sections()[1]);
  expect(row().getAttribute("aria-current")).toBe("true");
  await press("Commits");
  expect(list()).toBeNull();
  scrolled.mockRestore();
});
