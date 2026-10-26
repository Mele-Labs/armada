// The owner's own flight controls, drawn and pressed: each kind of action goes to the route its built-in cousin uses, a shell line goes
// through Fleet and its answer stands where the tiles were, a shell tile is off with no bay. Browser: a press and a rendering are what is read. The file comes from a fake `window.armada`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { FlightControls, FlightControlsRead, SessionShellRan } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { NOTHING_YET } from "../../../../shared/bridge";
import { FlightEdit } from "./FlightEdit";
import { chordTileFor, flightTileFor, pauseForP } from "./flight-keys";
import { SessionTiles } from "./SessionTiles";
import { plain } from "./session-tiles";
import { ShellBox, type ShellRun } from "./ShellBox";
import "../../styles/index.css";
import "./deck.css";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const FILE: FlightControls = {
  version: 1,
  job: [
    { id: "my-words", kind: "words", name: "Hand off", text: "Write down what is left, then stop." },
    { id: "my-command", kind: "command", name: "Review", text: "/review" },
    { id: "my-shell", kind: "shell", name: "Typecheck", text: "pnpm typecheck" },
    { id: "my-page", kind: "page", name: "Docs", text: "https://git.example/{repository}/pull/{pr}?b={branch}" },
  ],
  deck: [],
};

const ran = vi.fn(async (_session: string, _command: string): Promise<{ ok: true; value: SessionShellRan }> => ({ ok: true, value: { exit: 0, timed_out: false, output: "Found 0 errors.\n", truncated: false } }));

let base: unknown;
beforeAll(() => {
  const state = { ...NOTHING_YET, connection: { state: "connected" } } as unknown as BridgeState;
  base = (window as unknown as { armada: unknown }).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: state, seq: 1 }),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: FILE } }),
    saveFlightControls: async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }),
    getRepositoryFlightControls: async (repository: string) => ({ ok: true, value: { repository, place: "this_mac", read: { what: "read", controls: { version: 1, job: [], deck: [] } } } }),
    runSessionShell: ran,
  };
});

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  ran.mockClear();
  (window as unknown as { armada: unknown }).armada = base;
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount(node: React.ReactNode) {
  const host = document.createElement("div");
  // The foot gives the panel its height and `--tile` (the floor under its block); a bare host has neither, and the strip, as tall as the block's content, would stand no height.
  host.style.cssText = "--tile: 64px; display: flex; flex-direction: column; height: 160px";
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(node));
  roots.push({ root, host });
  await flush();
  await flush();
  return host;
}

const BRANCH = { kind: "branch", name: "mock/flight", slot: 7 };
const PULL = { kind: "pull_request", number: 1901, title: "Flight", branch: "mock/flight", address: "https://git.example/pull/1901", checks: { state: "pending" }, state: "open", auto: false };
const jobOf = (more: object = {}): Session => ({ id: "s1", rows: [], attachments: [BRANCH, PULL], turn: { state: "idle" }, ...more }) as unknown as Session;
const BAYS = { bays: [{ slot: { slot: 7 }, branch: "mock/flight", wrong: undefined, said: undefined }], clashes: [] } as never;
const REPOSITORY = { root: "/code/armada", name: "armada" };
const COMMANDS = [{ name: "review" }, { name: "compact" }];
const draftOf = (more: object = {}) => ({ commands: COMMANDS, get: () => [], send: vi.fn(async () => true), ...more }) as unknown as SessionsDraft & { send: ReturnType<typeof vi.fn> };

const tile = (host: HTMLElement, name: string) => host.querySelector<HTMLElement>(`button[aria-label="${name}"]`)!;
const press = (host: HTMLElement, name: string) => act(async () => tile(host, name).click());
const why = (button: HTMLElement) => document.getElementById(button.getAttribute("aria-describedby") ?? "")?.querySelector(".armada-tooltip__label")?.textContent;

test("words are sent to the Job as a message", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Hand off");
  expect(draft.send.mock.calls).toEqual([["s1", plain("Write down what is left, then stop.")]]);
});

test("a slash command is sent as a message to a hosted Job", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Review");
  expect(draft.send.mock.calls).toEqual([["s1", plain("/review")]]);
});

test("a slash command is typed in the terminal of a terminal Job", async () => {
  const command = vi.fn();
  const draft = draftOf({ command });
  const session = jobOf({ terminal: true, commands: COMMANDS });
  const host = await mount(<SessionTiles session={session} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Review");
  expect(command.mock.calls).toEqual([["s1", "/review"]]);
  expect(draft.send).not.toHaveBeenCalled();
});

test("a slash command the agent has not listed is off, and says so", async () => {
  const draft = draftOf({ commands: [] });
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  expect(tile(host, "Review").getAttribute("aria-disabled")).toBe("true");
  expect(why(tile(host, "Review"))).toBe("The agent has not listed review");
  await press(host, "Review");
  expect(draft.send).not.toHaveBeenCalled();
});

test("a page opens with the Job's branch, pull request and repository filled in", async () => {
  const open = vi.fn();
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} onOpenLink={open} />);
  await press(host, "Docs");
  expect(open).toHaveBeenCalledWith("https://git.example/armada/pull/1901?b=mock/flight");
});

test("a page that needs a fact the Job lacks is off, and says which", async () => {
  const open = vi.fn();
  const host = await mount(<SessionTiles session={jobOf({ attachments: [BRANCH] })} draft={draftOf()} repository={REPOSITORY} bays={BAYS} onOpenLink={open} />);
  expect(tile(host, "Docs").getAttribute("aria-disabled")).toBe("true");
  expect(why(tile(host, "Docs"))).toBe("The Job has no pull request");
  await press(host, "Docs");
  expect(open).not.toHaveBeenCalled();
});

test("a shell line runs through Fleet in the Job's slot, and what it printed stands where the tiles were until Close", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Typecheck");
  await flush();
  expect(ran).toHaveBeenCalledWith("s1", "pnpm typecheck");
  expect(draft.send).not.toHaveBeenCalled();
  const box = host.querySelector('[role="region"][aria-label="Shell output"]')!;
  expect(box.textContent).toContain("$ pnpm typecheck");
  expect(box.textContent).toContain("Found 0 errors.");
  expect(box.textContent).toContain("exit 0");
  expect(host.querySelector('button[aria-label="Hand off"]')).toBeNull();
  await act(async () => [...box.querySelectorAll("button")].find((one) => one.textContent === "Close")!.click());
  expect(tile(host, "Hand off")).not.toBeNull();
});

test("a shell line Fleet refuses is worded in the box", async () => {
  ran.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.session_shell_refused", message: "The Job holds no slot", run_id: "", fields: {}, chain: [] } } } as never);
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Typecheck");
  await flush();
  expect(host.querySelector('[aria-label="Shell output"]')!.textContent).toContain("The Job holds no slot");
});

test("a shell tile is off while the Job holds no bay, and a press runs nothing", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={{ bays: [], clashes: [] }} />);
  expect(tile(host, "Typecheck").getAttribute("aria-disabled")).toBe("true");
  expect(why(tile(host, "Typecheck"))).toBe("The Job holds no bay");
  await press(host, "Typecheck");
  expect(ran).not.toHaveBeenCalled();
});

// ---- A repository's own -----------------------------------------------------------------------------------------

const LH = { root: "/Users/user/widgets", name: "widgets" };
const SPECS = { id: "my-run-specs", kind: "shell", name: "Run specs", text: "bin/rspec" } as const;
const MINE: FlightControls = { version: 1, job: [], deck: [] };

/**
 * A fresh `window.armada`, so the window's store is made again over it: the person's file is `MINE` and the repository's `theirs`.
 * `saved` answers a save to the repository.
 */
function fleetWith(theirs: FlightControlsRead, saved?: () => Promise<unknown>) {
  const own = vi.fn(async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }));
  const repository = vi.fn(saved ?? (async (root: string, controls: FlightControls) => ({ ok: true, value: { repository: root, place: "this_mac", read: { what: "read", controls } } })));
  (window as unknown as { armada: unknown }).armada = {
    ...(base as object),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: MINE } }),
    saveFlightControls: own,
    getRepositoryFlightControls: async (root: string) => ({ ok: true, value: { repository: root, place: "this_mac", read: theirs } }),
    saveRepositoryFlightControls: repository,
  };
  return { own, repository };
}

async function editing(close = vi.fn()) {
  const band = document.createElement("span");
  document.body.append(band);
  roots.push({ root: createRoot(document.createElement("div")), host: band });
  const host = await mount(<FlightEdit set="job" host={band} repository={LH} onClose={close} />);
  return { host, band, close };
}

const done = (band: HTMLElement) => act(async () => [...band.querySelectorAll("button")].find((one) => one.textContent === "Done")!.click());
const only = () => page.getByRole("checkbox", { name: "Only in widgets" });

async function addShell(name: string, line: string, onlyThere: boolean) {
  await userEvent.click(page.getByRole("option", { name: "Add action" }));
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Kind" }), "shell");
  await userEvent.fill(page.getByRole("textbox", { name: "Name" }), name);
  await userEvent.fill(page.getByRole("textbox", { name: "Shell line" }), line);
  if (onlyThere) await userEvent.click(only());
  await userEvent.click(page.getByRole("button", { name: "Save" }));
}

test("an action marked Only in the repository is saved to the repository's file, and yours is not written", async () => {
  const saves = fleetWith({ what: "read", controls: MINE });
  const { band, close } = await editing();
  await addShell("Run specs", "bin/rspec", true);
  await done(band);
  await flush();
  expect(saves.repository).toHaveBeenCalledWith(LH.root, { version: 1, job: [SPECS], deck: [] });
  expect(saves.own).not.toHaveBeenCalled();
  expect(close).toHaveBeenCalled();
});

test("an action left unmarked is saved to your file, and the repository's is not written", async () => {
  const saves = fleetWith({ what: "read", controls: MINE });
  const { band } = await editing();
  await addShell("Typecheck", "pnpm typecheck", false);
  await done(band);
  await flush();
  expect(saves.own).toHaveBeenCalledTimes(1);
  expect(saves.own.mock.calls[0]![0].job).toContainEqual({ id: "my-typecheck", kind: "shell", name: "Typecheck", text: "pnpm typecheck" });
  expect(saves.repository).not.toHaveBeenCalled();
});

test("a repository's action unmarked moves to your file, and both files are saved", async () => {
  const saves = fleetWith({ what: "read", controls: { version: 1, job: [SPECS], deck: [] } });
  const { band } = await editing();
  await userEvent.click(page.getByRole("option", { name: "Run specs" }));
  await expect.element(only()).toBeChecked();
  await userEvent.click(only());
  await userEvent.click(page.getByRole("button", { name: "Save" }));
  await done(band);
  await flush();
  expect(saves.repository).toHaveBeenCalledWith(LH.root, { version: 1, job: [], deck: [] });
  expect(saves.own.mock.calls[0]![0].job).toContainEqual(SPECS);
});

test("a repository's tiles sit at the end of Edit, and move only among themselves", async () => {
  fleetWith({ what: "read", controls: { version: 1, job: [SPECS, { id: "my-lint", kind: "shell", name: "Lint", text: "bin/rubocop" }], deck: [] } });
  const { host } = await editing();
  const names = () => [...host.querySelectorAll('[role="option"]')].map((one) => one.getAttribute("aria-label"));
  expect(names().slice(-4)).toEqual(["Pause", "Run specs", "Lint", "Add action"]);
  await userEvent.keyboard("{Alt>}{ArrowLeft}{/Alt}");
  (host.querySelector('[data-edit-tile="my-run-specs"]') as HTMLElement).focus();
  await userEvent.keyboard("{Alt>}{ArrowLeft}{/Alt}");
  expect(names().slice(-4)).toEqual(["Pause", "Run specs", "Lint", "Add action"]);
  await userEvent.keyboard("{Alt>}{ArrowRight}{/Alt}");
  expect(names().slice(-4)).toEqual(["Pause", "Lint", "Run specs", "Add action"]);
});

test("a repository's action is drawn in its own Job's tiles, after the built-ins, and in no other repository's", async () => {
  fleetWith({ what: "read", controls: MINE });
  const armada = vi.fn(async (root: string) => ({ ok: true, value: { repository: root, place: "this_mac", read: { what: "read", controls: root === LH.root ? { version: 1, job: [SPECS], deck: [] } : MINE } } }));
  Object.assign((window as unknown as { armada: object }).armada, { getRepositoryFlightControls: armada });
  const there = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={LH} bays={BAYS} />);
  const names = (host: HTMLElement) => [...host.querySelectorAll(".deck-flight > button")].map((one) => one.getAttribute("aria-label"));
  expect(names(there).slice(-2)).toEqual(["Requests", "Run specs"]);
  const elsewhere = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  expect(names(elsewhere)).not.toContain("Run specs");
  expect(armada.mock.calls.map(([root]) => root)).toEqual([LH.root, REPOSITORY.root]);
});

test("a built-in's sheet never offers Only in the repository", async () => {
  fleetWith({ what: "read", controls: MINE });
  await editing();
  await userEvent.click(page.getByRole("option", { name: "Compact job context" }));
  await expect.element(page.getByRole("textbox", { name: "Command" })).toBeInTheDocument();
  await expect.element(only()).not.toBeInTheDocument();
});

test("a refused repository save says Fleet's words and keeps the edit on screen", async () => {
  const refused = { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.flight_controls_unplaced", message: "The synced folder is not mounted", run_id: "", fields: {}, chain: [] } } };
  fleetWith({ what: "read", controls: MINE }, async () => refused);
  const { host, band, close } = await editing();
  await addShell("Run specs", "bin/rspec", true);
  await done(band);
  await flush();
  expect(host.querySelector('[role="alert"]')?.textContent).toBe("Actions in widgets not saved: The synced folder is not mounted");
  expect(host.querySelector('[role="listbox"]')).not.toBeNull();
  expect(close).not.toHaveBeenCalled();
});

test("a repository file that will not read draws your own tiles only, and Edit says why", async () => {
  fleetWith({ what: "not_read", why: "line 3: unknown key" });
  const tiles = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={LH} bays={BAYS} />);
  expect(tiles.querySelectorAll(".deck-flight > button").length).toBe(7);
  const { host } = await editing();
  expect(host.querySelector('[role="status"]:not(.deck-sr)')?.textContent).toBe("Actions in widgets not read: line 3: unknown key");
  await userEvent.click(page.getByRole("option", { name: "Add action" }));
  await expect.element(page.getByRole("textbox", { name: "Name" })).toBeInTheDocument();
  await expect.element(only()).not.toBeInTheDocument();
});

// ---- A tile's key ----------------------------------------------------------------------------------------------------

/** The file with these entries laid over its tiles' ids. */
const keyed = (...keys: { id: string; key?: string }[]) => {
  (window as unknown as { armada: unknown }).armada = {
    ...(base as object),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: { ...FILE, job: [...FILE.job.map((one) => ({ ...one, ...keys.find((k) => k.id === one.id) })), ...keys.filter((k) => !FILE.job.some((one) => one.id === k.id))] } } }),
    saveFlightControls: async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }),
  };
};

/** The tile a keydown on `at` reaches, read the way the deck's handler reads it. */
function reach(at: Element, init: KeyboardEventInit, find: (event: KeyboardEvent) => HTMLElement | null = chordTileFor): HTMLElement | null {
  let found: HTMLElement | null = null;
  const listen = (event: KeyboardEvent) => void (found = find(event));
  window.addEventListener("keydown", listen);
  at.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
  window.removeEventListener("keydown", listen);
  return found;
}
const OPTION_R = { key: "®", code: "KeyR", altKey: true };

/** A store made over a fresh `window.armada` reads its file a moment after the first draw. */
const read = () => act(async () => void (await new Promise((done) => setTimeout(done, 150))));

/** The tiles drawn the way the deck holds them, inside its controls. */
async function drawn(props: Partial<React.ComponentProps<typeof SessionTiles>> = {}) {
  const host = await mount(
    <section className="deck-controls" style={{ display: "contents" }}>
      <SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} {...props} />
    </section>,
  );
  await read();
  return host;
}

test("a chord given to a tile presses it from anywhere on the Cockpit while the Job is open", async () => {
  keyed({ id: "my-words", key: "⌥R" });
  const host = await drawn();
  expect(reach(document.body, OPTION_R)).toBe(tile(host, "Hand off"));
  expect(reach(host, { key: "r", code: "KeyR", altKey: true, shiftKey: true })).toBeNull();
});

test("a tile's own key is Option and Shift with its number, so a bare number, which answers a question, presses nothing", async () => {
  const host = await drawn();
  expect(tile(host, "Judge").dataset.key).toBe("⌥⇧6");
  expect(tile(host, "Judge").querySelector(".deck-tile__key")?.textContent).toBe("6");
  expect(tile(host, "Hand off").dataset.key).toBeUndefined();
  expect(reach(document.body, { key: "6", code: "Digit6" }, flightTileFor)).toBeNull();
  expect(reach(document.body, { key: "‡", code: "Digit6", altKey: true, shiftKey: true })).toBe(tile(host, "Judge"));
});

test("⌥ and ⇧ chords do not fire in a text box, and ⌘ and ⌃ chords do", async () => {
  keyed({ id: "my-words", key: "⌥R" }, { id: "my-command", key: "⌘R" });
  const host = await drawn();
  const box = document.createElement("textarea");
  host.append(box);
  expect(reach(box, OPTION_R)).toBeNull();
  expect(reach(box, { key: "r", code: "KeyR", metaKey: true })).toBe(tile(host, "Review"));
  // ⌘ answers to Control as it does throughout the registry.
  expect(reach(box, { key: "r", code: "KeyR", ctrlKey: true })).toBe(tile(host, "Review"));
});

test("nothing fires while the tiles are edited, or a menu is open", async () => {
  keyed({ id: "my-words", key: "⌥R" });
  const host = await drawn();
  const editing = document.createElement("div");
  editing.setAttribute("data-flight-editing", "");
  const menu = document.createElement("div");
  menu.setAttribute("role", "menu");
  host.append(editing, menu);
  expect(reach(editing, OPTION_R)).toBeNull();
  expect(reach(menu, OPTION_R)).toBeNull();
  const edit = await mount(<FlightEdit set="job" host={null} onClose={() => {}} />);
  await read();
  expect(reach(edit.querySelector('[role="listbox"]')!, OPTION_R)).toBeNull();
});

test("a letter given to a tile presses it, and the same letter shifted does not", async () => {
  keyed({ id: "my-words", key: "r" });
  const host = await drawn();
  expect(tile(host, "Hand off").dataset.key).toBe("r");
  expect(reach(document.body, { key: "r", code: "KeyR" }, flightTileFor)).toBe(tile(host, "Hand off"));
  expect(reach(document.body, { key: "R", code: "KeyR", shiftKey: true }, flightTileFor)).toBeNull();
});

test("by default P presses Pause", async () => {
  const host = await drawn();
  expect(tile(host, "Pause").dataset.key).toBe("p");
  expect(pauseForP(null)).toBe(tile(host, "Pause"));
});

test("once Pause has another key, P presses the tile given P and leaves Pause alone", async () => {
  keyed({ id: "pause", key: "⌥P" }, { id: "my-words", key: "p" });
  const host = await drawn();
  expect(pauseForP(null)).toBeNull();
  expect(reach(document.body, { key: "p", code: "KeyP" }, flightTileFor)).toBe(tile(host, "Hand off"));
  expect(reach(document.body, { key: "π", code: "KeyP", altKey: true })).toBe(tile(host, "Pause"));
});

test("in the list, P presses the row's pause whatever Pause wears", async () => {
  keyed({ id: "pause", key: "⌥P" });
  const host = await drawn();
  const row = document.createElement("button");
  host.append(row);
  expect(pauseForP(row)).toBe(row);
});

// ---- Recording a key in the sheet ------------------------------------------------------------------------------------

const keyField = () => page.getByRole("button", { name: /^Key:/ });
const save = () => page.getByRole("button", { name: "Save" });
async function recording(name: string, ...keys: { id: string; key: string }[]) {
  keyed(...keys);
  const band = document.createElement("span");
  document.body.append(band);
  roots.push({ root: createRoot(document.createElement("div")), host: band });
  // Edit takes the tiles as they stand when it opens, so the file is read first, by tiles drawn beside it.
  await drawn();
  await mount(<FlightEdit set="job" host={band} onClose={() => {}} />);
  await userEvent.click(page.getByRole("option", { name }));
  await userEvent.click(keyField());
}

test("the Key control records ⌥R, ⇧⌘J or a bare letter in the registry's spelling", async () => {
  await recording("Hand off");
  await userEvent.keyboard("{Alt>}r{/Alt}");
  await expect.element(keyField()).toHaveAccessibleName("Key: ⌥R");
  await userEvent.click(keyField());
  await userEvent.keyboard("{Shift>}{Meta>}j{/Meta}{/Shift}");
  await expect.element(keyField()).toHaveAccessibleName("Key: ⇧⌘J");
  await userEvent.click(keyField());
  await userEvent.keyboard("r");
  await expect.element(keyField()).toHaveAccessibleName("Key: r");
  await expect.element(save()).toBeEnabled();
});

test("Esc lets go of the control without a change, and Delete clears the key", async () => {
  await recording("Hand off");
  await userEvent.keyboard("{Alt>}r{/Alt}");
  await userEvent.click(keyField());
  await userEvent.keyboard("{Escape}");
  await expect.element(keyField()).toHaveAccessibleName("Key: ⌥R");
  await expect.element(page.getByRole("dialog", { name: "Change Hand off" })).toBeInTheDocument();
  await userEvent.click(keyField());
  await userEvent.keyboard("{Delete}");
  await expect.element(keyField()).toHaveAccessibleName("Key: unbound");
});

test("a key the Cockpit answers is named and Save is held", async () => {
  await recording("Hand off");
  await userEvent.keyboard("m");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Used by Show the map");
  await expect.element(save()).toBeDisabled();
  await userEvent.click(keyField());
  await userEvent.keyboard("{Alt>}1{/Alt}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Used by Choose a repository to dispatch to");
  await expect.element(save()).toBeDisabled();
  await userEvent.click(keyField());
  await userEvent.keyboard("{Meta>}c{/Meta}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Used by Copy");
  await expect.element(save()).toBeDisabled();
});

test("Shift with a letter alone is refused and Save is held", async () => {
  await recording("Hand off");
  await userEvent.keyboard("{Shift>}j{/Shift}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("⇧ needs ⌘, ⌃ or ⌥ with it");
  await expect.element(save()).toBeDisabled();
});

test("a key another tile holds is named for that tile and Save is held", async () => {
  await recording("Review", { id: "my-words", key: "⌥R" });
  await userEvent.keyboard("{Alt>}r{/Alt}");
  await expect.element(page.getByRole("alert")).toHaveTextContent("Used by Hand off");
  await expect.element(save()).toBeDisabled();
});

test("a recorded key is saved in the registry's spelling, Pause's too", async () => {
  const saved = vi.fn(async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }));
  (window as unknown as { armada: object }).armada = { ...(base as object), saveFlightControls: saved };
  const band = document.createElement("span");
  document.body.append(band);
  roots.push({ root: createRoot(document.createElement("div")), host: band });
  // Edit takes the tiles as they stand when it opens, so the file is read first, by tiles drawn beside it.
  await drawn();
  await mount(<FlightEdit set="job" host={band} onClose={() => {}} />);
  await userEvent.click(page.getByRole("option", { name: "Pause" }));
  await userEvent.click(keyField());
  await userEvent.keyboard("{Alt>}p{/Alt}");
  await userEvent.click(save());
  await done(band);
  await flush();
  expect(saved.mock.calls.at(-1)![0].job).toContainEqual({ id: "pause", key: "⌥P" });
});

// ---- The box -----------------------------------------------------------------------------------------------------

const ended = (over: Partial<SessionShellRan>): ShellRun => ({ line: "pnpm test", ran: { timed_out: false, output: "", truncated: false, ...over } });
async function box(run: ShellRun) {
  const host = await mount(<ShellBox run={run} onClose={() => {}} />);
  return host.querySelector<HTMLElement>('[role="region"]')!;
}
const code = (region: HTMLElement) => [...region.querySelectorAll<HTMLElement>(".deck-shell__code")];

test("the box prints the line, the output and the exit", async () => {
  const region = await box(ended({ exit: 0, output: "ok\n" }));
  expect(region.textContent).toContain("$ pnpm test");
  expect(region.textContent).toContain("ok");
  expect(code(region).map((one) => one.textContent)).toEqual(["exit 0"]);
  expect(code(region)[0]!.hasAttribute("data-failed")).toBe(false);
  expect(region.getAttribute("aria-busy")).toBeNull();
});

test("a non-zero exit is drawn as failed", async () => {
  const region = await box(ended({ exit: 2, output: "1 failed\n" }));
  expect(code(region).map((one) => one.textContent)).toEqual(["exit 2"]);
  expect(code(region)[0]!.hasAttribute("data-failed")).toBe(true);
});

test("a line stopped at the limit says timed out and has no exit", async () => {
  const region = await box(ended({ timed_out: true, output: "waiting\n" }));
  expect(code(region).map((one) => one.textContent)).toEqual(["timed out"]);
  expect(code(region)[0]!.hasAttribute("data-failed")).toBe(true);
});

test("output cut to fit says so before the part kept", async () => {
  const region = await box(ended({ exit: 0, truncated: true, output: "tail of it\n" }));
  const text = region.textContent ?? "";
  expect(code(region)[0]!.textContent).toBe("truncated to the last 64 KiB");
  expect(text.indexOf("truncated")).toBeLessThan(text.indexOf("tail of it"));
});

test("terminal colour codes are not drawn", async () => {
  const region = await box(ended({ exit: 0, output: "\u001b[31mred\u001b[0m\n" }));
  expect(region.textContent).toContain("red");
  expect(region.textContent).not.toContain("\u001b");
});

test("while the line runs the box is busy and says nothing of how it ended", async () => {
  const region = await box({ line: "pnpm test" });
  expect(region.getAttribute("aria-busy")).toBe("true");
  expect(code(region)).toEqual([]);
});

test("a line that was refused shows the words", async () => {
  const region = await box({ line: "pnpm test", said: "The Job holds no slot" });
  expect(code(region).map((one) => one.textContent)).toEqual(["The Job holds no slot"]);
});
