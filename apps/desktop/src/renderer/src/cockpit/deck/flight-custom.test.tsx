// The owner's own flight controls, drawn and pressed: each kind of action goes to the route its built-in cousin uses, a shell line goes
// through Fleet and its answer stands where the tiles were, a shell tile is off with no bay, and a Job's tiles past twelve sit on pages of twelve, a dot for each,
// in the tiles and in edit mode. Browser: a press and a rendering are what is read. The file comes from a fake `window.armada`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { FlightControls, FlightControlsRead, SessionShellRan } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { NOTHING_YET } from "../../../../shared/bridge";
import { ASK_FOR_UPDATE } from "./acts";
import { FlightEdit } from "./FlightEdit";
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

test("words are sent to the Job the way Ask for update's are", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Ask for update");
  await press(host, "Hand off");
  expect(draft.send.mock.calls).toEqual([
    ["s1", plain(ASK_FOR_UPDATE)],
    ["s1", plain("Write down what is left, then stop.")],
  ]);
});

test("a slash command is sent as a message to a hosted Job, as Compact's /compact is", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Compact job context");
  await press(host, "Review");
  expect(draft.send.mock.calls).toEqual([
    ["s1", plain("/compact")],
    ["s1", plain("/review")],
  ]);
});

test("a slash command is typed in the terminal of a terminal Job, as Compact's is", async () => {
  const command = vi.fn();
  const draft = draftOf({ command });
  const session = jobOf({ terminal: true, commands: COMMANDS });
  const host = await mount(<SessionTiles session={session} draft={draft} repository={REPOSITORY} bays={BAYS} />);
  await press(host, "Compact job context");
  await press(host, "Review");
  expect(command.mock.calls).toEqual([
    ["s1", "/compact"],
    ["s1", "/review"],
  ]);
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

const strip = (host: HTMLElement) => host.querySelector<HTMLElement>(".deck-controls__pages")!;
const dots = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".deck-controls__dots > button")];
const current = (host: HTMLElement) => dots(host).findIndex((one) => one.hasAttribute("aria-current"));
const onPage = (host: HTMLElement, page: number) => host.querySelectorAll(`.deck-controls__tiles > [data-page="${page}"]`).length;
/** A page takes a moment to scroll in, and the dot follows the scroll. */
const settle = () => act(async () => void (await new Promise((done) => setTimeout(done, 900))));

test("seventeen tiles sit on two pages, twelve and five, with a dot for each and the first current", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  expect(host.querySelectorAll(".deck-controls__tiles > button").length).toBe(17);
  expect([onPage(host, 0), onPage(host, 1)]).toEqual([12, 5]);
  expect(dots(host).map((one) => one.getAttribute("aria-label"))).toEqual(["Page 1", "Page 2"]);
  expect(current(host)).toBe(0);
  expect(strip(host).scrollWidth).toBeGreaterThan(strip(host).clientWidth);
});

test("twelve tiles sit on one page with no dots", async () => {
  const hidden = ["land", "report", "map", "judge", "release"].map((id) => ({ id, hidden: true }));
  (window as unknown as { armada: unknown }).armada = { ...(base as object), getFlightControls: async () => ({ ok: true, value: { what: "read", controls: { ...FILE, job: [...FILE.job, ...hidden] } } }) };
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  expect(host.querySelectorAll(".deck-controls__tiles > button").length).toBe(12);
  expect(onPage(host, 0)).toBe(12);
  expect(dots(host)).toHaveLength(0);
  expect(strip(host).scrollWidth).toBeLessThanOrEqual(strip(host).clientWidth);
});

test("a press on a page's dot brings that page in and makes it current", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  await act(async () => dots(host)[1]!.click());
  await settle();
  expect(current(host)).toBe(1);
  expect(strip(host).scrollLeft).toBeGreaterThan(0);
  await act(async () => dots(host)[0]!.click());
  await settle();
  expect(current(host)).toBe(0);
  expect(strip(host).scrollLeft).toBe(0);
});

test("a tile on the second page taking focus brings its page in", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} bays={BAYS} />);
  expect(tile(host, "Pause").dataset.page).toBe("1");
  await act(async () => tile(host, "Pause").focus());
  await settle();
  expect(current(host)).toBe(1);
  expect(strip(host).scrollLeft).toBeGreaterThan(0);
});

test("edit mode pages the same way, with + the last cell of the last page", async () => {
  const host = await mount(<FlightEdit set="job" host={null} onClose={() => {}} />);
  const add = host.querySelector<HTMLElement>("[data-add]")!;
  expect([onPage(host, 0), onPage(host, 1)]).toEqual([12, 6]);
  expect(add.dataset.page).toBe("1");
  expect(host.querySelector(".deck-controls__tiles")!.lastElementChild).toBe(add);
  expect(dots(host)).toHaveLength(2);
  expect(current(host)).toBe(0);
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

test("a repository's action is drawn in its own Job's tiles, before Pause, and in no other repository's", async () => {
  fleetWith({ what: "read", controls: MINE });
  const armada = vi.fn(async (root: string) => ({ ok: true, value: { repository: root, place: "this_mac", read: { what: "read", controls: root === LH.root ? { version: 1, job: [SPECS], deck: [] } : MINE } } }));
  Object.assign((window as unknown as { armada: object }).armada, { getRepositoryFlightControls: armada });
  const there = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={LH} bays={BAYS} />);
  const names = (host: HTMLElement) => [...host.querySelectorAll(".deck-controls__tiles > button")].map((one) => one.getAttribute("aria-label"));
  expect(names(there).slice(-2)).toEqual(["Run specs", "Pause"]);
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
  expect(tiles.querySelectorAll(".deck-controls__tiles > button").length).toBe(13);
  const { host } = await editing();
  expect(host.querySelector('[role="status"]:not(.deck-sr)')?.textContent).toBe("Actions in widgets not read: line 3: unknown key");
  await userEvent.click(page.getByRole("option", { name: "Add action" }));
  await expect.element(page.getByRole("textbox", { name: "Name" })).toBeInTheDocument();
  await expect.element(only()).not.toBeInTheDocument();
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
