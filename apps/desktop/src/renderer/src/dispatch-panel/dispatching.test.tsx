// The words in the Dispatch sheet and Fleet's reading of them belong to the sheet. Whatever holds the
// dispatching outside it (in the app, the whole window) is not drawn again for a key typed there.

import { forgetAllKept } from "@armada/components";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { connected } from "../mock/moment";
import { SessionsFrom } from "../sessions-draft";
import { DispatchSheet } from "./entries";
import { DispatchingProvider, useDispatch, useDispatching, type Dispatching } from "./dispatching";

const counts = { window: 0, outside: 0 };
let held: Dispatching | undefined;
let mounted: { root: Root; host: HTMLElement } | undefined;
const state = connected([], []);
const readRequestRepository = vi.fn(async (_words: string) => ({ manifest_id: null, likely: [] }));

/** A part of the window outside the sheet that reads the dispatching, as the Deck and the Merge line do. */
function Outside() {
  held = useDispatching();
  counts.outside += 1;
  return null;
}

/** What the window is given beyond the sheet: Sessions the draft serves, and the repositories set up. */
let scene: { draft?: SessionsDraft; repositories: ReturnType<typeof repository>[]; opened: string[] } = { repositories: [], opened: [] };

/** What the app is to the dispatching: the one that holds it. */
function Window() {
  counts.window += 1;
  const dispatching = useDispatch({
    state,
    repositories: scene.repositories,
    openSession: (id) => void scene.opened.push(id),
    openAutopilot: () => undefined,
  });
  return (
    <DispatchingProvider value={dispatching}>
      <Outside />
      <DispatchSheet />
    </DispatchingProvider>
  );
}

/** The window under what serves its Sessions, as the app mounts it. */
function Scene() {
  return (
    <SessionsFrom held={scene.draft}>
      <Window />
    </SessionsFrom>
  );
}

beforeEach(() => {
  forgetAllKept();
  scene = { repositories: [], opened: [] };
  counts.window = 0;
  counts.outside = 0;
  readRequestRepository.mockClear();
  (window as unknown as { armada: unknown }).armada = { readRequestRepository };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(<Scene />);
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  held = undefined;
  (window as unknown as { armada?: unknown }).armada = undefined;
});

test("typing in the sheet draws the sheet, and not the window that holds the dispatching", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await expect.element(request).toHaveFocus();
  // The first key lands on a window that has just opened its sheet, and React runs the window's body once more
  // to find that the acts typing makes change nothing. That draws no part of it, and it is the one pass allowed.
  await userEvent.keyboard("F");
  await expect.element(request).toHaveValue("F");
  const before = { ...counts };

  await userEvent.keyboard("ix the login page");
  await expect.element(request).toHaveValue("Fix the login page");
  // The reading is made and answered, and that stays inside the sheet too.
  await expect.poll(() => readRequestRepository.mock.calls.at(-1)?.[0]).toBe("Fix the login page");
  await new Promise((done) => setTimeout(done, 100));

  expect(counts).toEqual(before);
});

test("the draft is still there when the sheet is put away and comes back", async () => {
  await expect.poll(() => held).toBeDefined();
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.keyboard("Fix the login page");
  await expect.element(request).toHaveValue("Fix the login page");

  held?.closeSheet();
  await expect.element(request).not.toBeInTheDocument();
  held?.openNew();
  await expect.element(request).toHaveValue("Fix the login page");
});

// ---- A Session's Model and Effort, and Queue for later ----------------------------------------------------

const ARMADA = repository();
const ROOT = ARMADA.root;

/** The draft a real Fleet serves, as far as dispatching reads it: what it starts, what it queues, and the lists the selects offer. */
function served(serves: { queue: boolean } = { queue: true }) {
  // One list, since the sheet reads the Sessions through `useSyncExternalStore` and a new array each read never settles.
  const none: never[] = [];
  const start = vi.fn(async (..._args: Parameters<SessionsDraft["start"]>) => "s-new");
  const send = vi.fn();
  const queue = vi.fn();
  const draft = {
    get: () => none,
    subscribe: () => () => undefined,
    start,
    send,
    ...(serves.queue ? { queue } : {}),
    models: ["haiku", "sonnet", "opus"],
    defaultModel: "sonnet",
    efforts: ["low", "medium", "high"],
  } as unknown as SessionsDraft;
  return { draft, start, send, queue };
}

/** The window again, over `scene`, as if it had opened with it. */
async function remount(next: Partial<typeof scene>) {
  scene = { ...scene, ...next };
  mounted?.root.unmount();
  mounted?.host.remove();
  held = undefined;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(<Scene />);
  await expect.poll(() => held).toBeDefined();
}

/** The sheet open on a Session for the armada repository, with `words` typed. */
async function sessionSheet(words: string) {
  held?.choose("session");
  held?.openNew(ROOT);
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.click(request);
  await userEvent.keyboard(words);
  await expect.element(request).toHaveValue(words);
  return request;
}

const queueButton = () => page.getByRole("button", { name: "Queue for later" });

test("Queue for later saves the idea with its repository and picks, starts nothing, and lets the sheet go", async () => {
  const fleet = served();
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Make the Cleanup grid sortable by size");
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Model" }), "opus");
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Effort" }), "high");

  await userEvent.click(queueButton());

  expect(fleet.queue).toHaveBeenCalledTimes(1);
  expect(fleet.queue).toHaveBeenCalledWith({ text: "Make the Cleanup grid sortable by size", root: ROOT, repository: expect.any(String), model: "opus", effort: "high" });
  expect(fleet.start).not.toHaveBeenCalled();
  expect(scene.opened).toEqual([]);
  await expect.element(page.getByRole("textbox", { name: "Request" })).not.toBeInTheDocument();
  // The draft and the picks are let go as a start lets them go.
  held?.openNew(ROOT);
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveValue("");
  await expect.element(page.getByRole("combobox", { name: "Model" })).toHaveValue("");
});

test("⇧⌘Enter in the request queues the idea and does not also start it", async () => {
  const fleet = served();
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Draft the billing export notes");

  await userEvent.keyboard("{Control>}{Shift>}{Enter}{/Shift}{/Control}");

  await expect.poll(() => fleet.queue.mock.calls.length).toBe(1);
  expect(fleet.queue.mock.calls[0]?.[0]).toMatchObject({ text: "Draft the billing export notes", root: ROOT });
  expect(fleet.start).not.toHaveBeenCalled();
  expect(scene.opened).toEqual([]);
});

test("plain ↵ still starts a Session, and queues nothing", async () => {
  const fleet = served();
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Fix the login page");

  await userEvent.keyboard("{Enter}");

  await expect.poll(() => fleet.start.mock.calls.length).toBe(1);
  expect(fleet.start).toHaveBeenCalledWith(undefined, ROOT, "Fix the login page", {});
  await expect.poll(() => scene.opened).toEqual(["s-new"]);
  expect(fleet.queue).not.toHaveBeenCalled();
});

test("a start leaves the model and the effort out when the selects were not touched, and sends them when they were", async () => {
  const fleet = served();
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Fix the login page");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => fleet.start.mock.calls.length).toBe(1);
  expect(fleet.start.mock.calls[0]?.[3]).toEqual({});

  await sessionSheet("Fix the logout page");
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Model" }), "haiku");
  await userEvent.click(page.getByRole("button", { name: "Start", exact: true }));
  await expect.poll(() => fleet.start.mock.calls.length).toBe(2);
  expect(fleet.start.mock.calls[1]?.[3]).toEqual({ model: "haiku" });
});

test("the selects offer the draft's models and efforts, with the default model named on Auto", async () => {
  const fleet = served();
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Fix the login page");
  const model = page.getByRole("combobox", { name: "Model" });
  await expect.element(model.getByRole("option", { name: "sonnet" }).first()).toBeInTheDocument();
  await expect.element(model.getByRole("option", { name: "opus" })).toBeInTheDocument();
  await expect.element(page.getByRole("combobox", { name: "Effort" }).getByRole("option", { name: "high" })).toBeInTheDocument();
});

test("an Autopilot has no Queue for later, and the key queues nothing there", async () => {
  const fleet = served();
  (window as unknown as { armada: Record<string, unknown> }).armada = { readRequestRepository, proposeAutopilot: vi.fn() };
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  for (const kind of ["autopilot"] as const) {
    held?.choose(kind);
    held?.openNew(ROOT);
    const request = page.getByRole("textbox", { name: "Request" });
    await expect.element(request).toBeVisible();
    await userEvent.click(request);
    await userEvent.keyboard("Fix the login page");
    await expect.element(queueButton()).not.toBeInTheDocument();
    await expect.element(page.getByRole("combobox", { name: "Model" })).not.toBeInTheDocument();
    await userEvent.keyboard("{Control>}{Shift>}{Enter}{/Shift}{/Control}");
    held?.closeSheet();
  }
  expect(fleet.queue).not.toHaveBeenCalled();
});

test("where the draft serves no queue there is no Queue for later and the selects stay", async () => {
  const fleet = served({ queue: false });
  await remount({ draft: fleet.draft, repositories: [ARMADA] });
  await sessionSheet("Fix the login page");
  await expect.element(queueButton()).not.toBeInTheDocument();
  await expect.element(page.getByRole("combobox", { name: "Model" })).toBeInTheDocument();
});

test("with no repository chosen Queue for later asks for one, and the pick queues the idea", async () => {
  const fleet = served();
  const elsewhere = { ...ARMADA, root: "/Users/user/billing", manifest: { ...ARMADA.manifest!, id: "billing", repository: "billing" } };
  await remount({ draft: fleet.draft, repositories: [ARMADA, elsewhere] });
  held?.choose("session");
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.click(request);
  await userEvent.keyboard("Draft the export notes");

  await userEvent.click(queueButton());
  expect(fleet.queue).not.toHaveBeenCalled();

  await userEvent.click(page.getByRole("radio", { name: /Repository billing/ }));
  await expect.poll(() => fleet.queue.mock.calls.length).toBe(1);
  expect(fleet.queue.mock.calls[0]?.[0]).toMatchObject({ text: "Draft the export notes", root: "/Users/user/billing", repository: "billing" });
  expect(fleet.start).not.toHaveBeenCalled();
});

test("Queue for later pressed while the reading is out is answered by the repository picked, without the reading", async () => {
  // A reading that never comes back, as a slow Fleet's does on a loaded machine.
  readRequestRepository.mockImplementation(() => new Promise(() => undefined));
  const fleet = served();
  const elsewhere = { ...ARMADA, root: "/Users/user/billing", manifest: { ...ARMADA.manifest!, id: "billing", repository: "billing" } };
  await remount({ draft: fleet.draft, repositories: [ARMADA, elsewhere] });
  held?.choose("session");
  held?.openNew();
  const request = page.getByRole("textbox", { name: "Request" });
  await expect.element(request).toBeVisible();
  await userEvent.click(request);
  await userEvent.keyboard("Draft the export notes");

  try {
    await userEvent.click(queueButton());
    await userEvent.click(page.getByRole("radio", { name: /Repository billing/ }));
    await expect.poll(() => fleet.queue.mock.calls.length).toBe(1);
    expect(fleet.queue.mock.calls[0]?.[0]).toMatchObject({ text: "Draft the export notes", root: "/Users/user/billing" });
  } finally {
    readRequestRepository.mockImplementation(async () => ({ manifest_id: null, likely: [] }));
  }
});
