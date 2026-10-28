// A pull request pressed opens its panel at once, before the forge's read answers; a read that answers nothing leaves the panel
// open and saying so; and its own Open in browser goes to the browser, never back into this panel. Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { PullRequestPage, SessionRunMessage } from "@armada/protocol";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import "../styles/index.css";
import { SessionsFrom } from "../sessions-draft";
import { mockPullRequestNotesApi, resolvePullRequestNoteInMock } from "../mock/slices/pull-request-notes";
import { fleetPanelReads, pullRequestNotesReads } from "./fleet-reads";
import type { FleetPanelApi, FleetPanelState } from "./fleet-reads";
import { SharedPanelHost } from "./host";
import { forgetNotes, readNotes } from "./job-notes";
import type { NoteHandle } from "./job-notes";
import { SharedPanelFrom, forgetReadings } from "./reads";
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
const refreshing = () => document.querySelector('[role="progressbar"][aria-label="Refreshing pull request #418"]');
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
  forgetReadings();
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

test("a panel opened again draws what it read before while the read goes out once more", async () => {
  await mount(readsOf(async () => READ));
  await act(async () => root!.unmount());
  host?.remove();
  const again = held();
  await mount(readsOf(again.read));
  expect(status()).toBeNull();
  expect(document.querySelector('[role="tab"]')).not.toBeNull();
  await again.answer({ ...READ, facts: { ...READ.facts, title: "Keep the tide tables two days" } });
  expect(document.body.textContent).toContain("Keep the tide tables two days");
});

test("what was read before is marked as refreshing while the read goes out once more, and the mark goes when it lands", async () => {
  await mount(readsOf(async () => READ));
  expect(refreshing()).toBeNull();
  await act(async () => root!.unmount());
  host?.remove();
  const again = held();
  await mount(readsOf(again.read));
  expect(refreshing()).not.toBeNull();
  await again.answer(READ);
  expect(refreshing()).toBeNull();
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

test("a page the forge refused is drawn with Fleet's words for it, not the fixed sentence", async () => {
  const said = "GitHub refused access to harbor-labs/tidewater. Run gh auth refresh -s repo, then press this pull request again.";
  const refused = async () => ({ ok: false, outcome: { ok: false, why: "refused", error: { message: said } } });
  const api = { readReviewPage: refused, getPullRequest: refused } as unknown as FleetPanelApi;
  const state = { holds: { repositories: [{ root: "/code/tidewater", records_root: "/r", manifest: { id: "tidewater", repository: "harbor-labs/tidewater" } }] } } as unknown as FleetPanelState;
  await mount({ ...readsOf(async () => undefined), ...fleetPanelReads(api, () => state) });
  await act(async () => {});
  expect(document.querySelector('[role="alert"]')?.textContent).toBe(said);
});

const FIX = "gh auth refresh -s repo";
const FIX_SAID = `GitHub refused the token gh signed in with. Sign in again with \`${FIX}\`, authorize it for the organization's SSO, then open it again.`;

/** A Fleet that runs what it is given: its output arrives by run id, as main sends it. */
function runner() {
  let hear: (runId: string, message: SessionRunMessage) => void = () => {};
  const api = {
    startTerminalRun: vi.fn(async (_command: string, _dir: string) => ({ ok: true, value: { run_id: "run-1" } }) as const),
    writeSessionRun: vi.fn(async () => ({ ok: true, value: { run_id: "run-1" } }) as const),
    resizeSessionRun: vi.fn(async () => ({ ok: true, value: { run_id: "run-1" } }) as const),
    stopSessionRun: vi.fn(async () => ({ ok: true, value: { run_id: "run-1" } }) as const),
    onSessionRunMessage: (on: typeof hear) => {
      hear = on;
      return () => undefined;
    },
  };
  return { api, say: (message: SessionRunMessage) => act(async () => hear("run-1", message)) };
}

const pressed = (name: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((one) => (one.getAttribute("aria-label") ?? one.textContent) === name);
const refusedReads = (said: string, terminalRun?: PanelReads["terminalRun"]): PanelReads => ({
  ...readsOf(async () => ({ refused: said, root: "/code/tidewater" })),
  ...(terminalRun === undefined ? {} : { terminalRun }),
});

test("a refusal that names a command draws Run and Copy, and Run runs it in the repository and draws what it prints", async () => {
  const fleet = runner();
  await mount(refusedReads(FIX_SAID, fleet.api));
  await act(async () => {});
  expect(pressed("Copy command")).not.toBeUndefined();
  await act(async () => pressed("Run")!.click());
  expect(fleet.api.startTerminalRun).toHaveBeenCalledWith(FIX, "/code/tidewater");
  await fleet.say({ kind: "output", text: "Logged in as ines\r\n" });
  await vi.waitFor(() => expect(document.querySelector(".xterm")?.textContent).toContain("Logged in as ines"));
  await fleet.say({ kind: "exited", code: 0, stopped: false });
  expect(pressed("Open the pull request again")).not.toBeUndefined();
});

test("a refusal that names no command draws neither Run nor Copy", async () => {
  await mount(refusedReads("GitHub refused access to harbor-labs/tidewater. Open it again later.", runner().api));
  await act(async () => {});
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  expect(pressed("Run")).toBeUndefined();
  expect(pressed("Copy command")).toBeUndefined();
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
    { login: "ines", kind: "person", verdict: "approved", name: "Ines Alvarez", url: "https://forge.example/ines" },
    { login: "dockside", kind: "team" },
  ] as const;
  await mount(readsOf(async () => ({ ...READ, page: { ...PAGE, reviewers: [...reviewers] } })));
  const plate = document.querySelector('section[aria-label="Reviewers"]');
  // The team is its own group's heading, not a list item; ines is a person no team placed.
  expect(plate?.querySelectorAll("li")).toHaveLength(1);
  expect(plate?.querySelector('[role="group"][aria-label="dockside"]')).not.toBeNull();
  expect(plate?.textContent).toContain("ines");
  expect(plate?.querySelector('[role="img"][aria-label="Team"]')).not.toBeNull();
  expect(plate?.querySelector('[role="img"][aria-label="Approved"]')).not.toBeNull();
  expect(plate?.querySelector('[role="img"][aria-label="Waiting"]')).not.toBeNull();
  // A login with a url opens through the host's opener; one with none is text.
  const link = [...(plate?.querySelectorAll("button") ?? [])].find((one) => one.textContent === "ines");
  await act(async () => link!.click());
  expect(openLink).toHaveBeenCalledWith("https://forge.example/ines");
  expect([...(plate?.querySelectorAll("button") ?? [])].some((one) => one.textContent === "dockside")).toBe(false);
  expect(plate?.querySelector('li [aria-describedby]')?.textContent).toBe("ines");
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
  const row = () => [...list()!.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.textContent?.includes("cache.rs"))!;
  expect(list()!.textContent).toContain("src/tide/");
  expect(row().getAttribute("aria-current")).toBeNull();
  await act(async () => row().click());
  expect(scrolled.mock.contexts).toContain(sections()[1]);
  expect(row().getAttribute("aria-current")).toBe("true");
  await press("Commits");
  expect(list()).toBeNull();
  scrolled.mockRestore();
});

// ---- A diff is the one the forge shows now: the head moves, and what was drawn of the old one goes ---------------

/** A forge whose answer moves: its page names the head, its patches are that head's. */
function forge(head: string, line: string) {
  const now = { head, line };
  const page = (): PullRequestRead => ({ ...WITH_FILES, page: { ...WITH_FILES.page, head: now.head } });
  const patches = async (): Promise<PatchesRead> => ({ ok: true, files: [patch("src/tide/tables.rs", { patch: `@@ -1 +1 @@\n-let days = 1;\n+${now.line}` })] });
  return { now, reads: { ...readsOf(async () => page()), pullRequestPatches: patches } satisfies PanelReads };
}

test("a pull request opened again shows the diff of the head the forge names now, not the one drawn before", async () => {
  const tide = forge("h1", "let days = 2;");
  await mount(tide.reads);
  await press("Files");
  expect(filesPanel()?.textContent).toContain("+let days = 2;");
  tide.now.head = "h2";
  tide.now.line = "let days = 3;";
  await act(async () => void openInPanel({ ...ITEM }));
  expect(filesPanel()?.textContent).toContain("+let days = 3;");
  expect(filesPanel()?.textContent).not.toContain("+let days = 2;");
});

test("Files entered again shows the diff of the head the forge names now, not the one drawn before", async () => {
  const tide = forge("h1", "let days = 2;");
  await mount(tide.reads);
  await press("Files");
  expect(filesPanel()?.textContent).toContain("+let days = 2;");
  await press("Conversation");
  tide.now.head = "h2";
  tide.now.line = "let days = 3;";
  await press("Files");
  expect(filesPanel()?.textContent).toContain("+let days = 3;");
  expect(filesPanel()?.textContent).not.toContain("+let days = 2;");
});

// ---- Notes for the Job: written on a line of the diff, kept on this Mac, never posted --------------------------

/** The panel's reads with Fleet's notes over memory, each test starting with none. */
const noted = (): PanelReads => {
  forgetNotes();
  return { ...readsOf(async () => WITH_FILES), pullRequestPatches: async () => PATCHES, notes: pullRequestNotesReads(mockPullRequestNotesApi()) };
};
/** What the window holds of this pull request's notes. */
const kept = () => readNotes({ manifestId: "tidewater", number: 418 } as NoteHandle);

/** Types into a controlled box the way a person does, so React sees the change. */
const type = (box: HTMLTextAreaElement, words: string) =>
  act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(box, words);
    box.dispatchEvent(new Event("input", { bubbles: true }));
  });
const gutter = (label: string) => [...document.querySelectorAll<HTMLButtonElement>("button.armada-diff__gutter")].find((one) => one.getAttribute("aria-label") === label);

test("a press on a diff line opens a note box, and Save puts the note under that line and keeps it on this Mac", async () => {
  await mount(noted());
  await press("Files");
  await act(async () => gutter("Note for the job on +let days = 2;")!.click());
  const box = filesPanel()!.querySelector<HTMLTextAreaElement>('textarea[aria-label="Note for the job"]')!;
  await type(box, "Name the unit.");
  await act(async () => [...filesPanel()!.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.textContent === "Save note")!.click());
  expect(sections()[0]!.querySelector('[aria-label="Note for the job"]')?.textContent).toContain("Name the unit.");
  expect(filesPanel()!.querySelector('textarea[aria-label="Note for the job"]')).toBeNull();
  expect(kept()).toMatchObject([{ path: "src/tide/tables.rs", line: 1, side: "right" }]);
});

const inFiles = (name: string) => [...filesPanel()!.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.getAttribute("aria-label") === name || one.textContent === name);

test("a note is edited in place and removed, until it is sent", async () => {
  await mount(noted());
  await press("Files");
  await act(async () => gutter("Note for the job on +let days = 2;")!.click());
  await type(filesPanel()!.querySelector<HTMLTextAreaElement>('textarea[aria-label="Note for the job"]')!, "Name the unit.");
  await act(async () => inFiles("Save note")!.click());

  await act(async () => inFiles("Edit the note")!.click());
  const box = filesPanel()!.querySelector<HTMLTextAreaElement>('textarea[aria-label="Note for the job"]')!;
  expect(box.value).toBe("Name the unit.");
  await type(box, "Name the unit, in seconds.");
  await act(async () => inFiles("Save note")!.click());
  expect(sections()[0]!.textContent).toContain("Name the unit, in seconds.");
  expect(kept()[0]!.body).toContain("in seconds");

  await act(async () => inFiles("Remove the note")!.click());
  expect(sections()[0]!.textContent).not.toContain("in seconds");
  expect(kept()).toEqual([]);
});

/** A window holding one Job, which holds the pull request, and the messages it was sent. */
function heldJob(others: readonly { id: string; title: string; dead?: "ended" }[] = []) {
  const send = vi.fn(async (_id: string, _message: { text: string }) => true);
  const session = { id: "job-1", title: "Tide cache", attachments: [{ kind: "pull_request", number: 418, title: "Keep the tide tables a day", branch: "ines/tide-cache", address: ADDRESS, checks: { state: "passed" }, state: "open", auto: false }], waitingFor: [], rows: [] };
  const sessions = [session, ...others.map((one) => ({ ...one, attachments: [], waitingFor: [], rows: [] }))];
  const held = { get: () => sessions, subscribe: () => () => undefined, send } as unknown as SessionsDraft;
  return { send, held };
}
async function mountHeld(reads: PanelReads, held: SessionsDraft | undefined) {
  host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
  await act(async () =>
    root!.render(
      <SessionsFrom held={held}>
        <SharedPanelFrom reads={reads}>
          <Watch />
          <SharedPanelHost />
        </SharedPanelFrom>
      </SessionsFrom>,
    ),
  );
  await act(async () => void openInPanel(ITEM));
}
const noteOn = async (line: string, words: string) => {
  await act(async () => gutter(`Note for the job on ${line}`)!.click());
  await type(filesPanel()!.querySelector<HTMLTextAreaElement>('textarea[aria-label="Note for the job"]')!, words);
  await act(async () => inFiles("Save note")!.click());
};
const sendButton = () => [...document.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.textContent === "Send to job");

test("Send to job sends one numbered message naming each note's file and line, and marks them sent", async () => {
  const job = heldJob();
  await mountHeld(noted(), job.held);
  await press("Files");
  expect(sendButton()).toBeUndefined();
  await noteOn("+let days = 2;", "Name the unit.");
  await noteOn("+pub struct Cache;", "Why public?");
  // The bar says which Job the press goes to.
  expect(document.querySelector('[role="group"][aria-label="Notes for the job"]')?.textContent).toContain("2 notes for Tide cache");
  await act(async () => sendButton()!.click());
  expect(job.send).toHaveBeenCalledTimes(1);
  expect(job.send.mock.calls[0]![0]).toBe("job-1");
  expect(job.send.mock.calls[0]![1].text).toBe(
    "Notes on #418 from the owner, kept on his Mac and not posted to the pull request. When you have dealt with a note, call resolve_pull_request_note with its id.\n\n1. src/tide/tables.rs:1 [note-1]: Name the unit.\n2. src/tide/cache.rs:1 [note-2]: Why public?",
  );
  expect(sendButton()).toBeUndefined();
  expect(sections()[0]!.querySelector('[aria-label="Note for the job"]')?.textContent).toContain("Name the unit.");
  expect(kept().map((one) => [one.sent, one.title])).toEqual([[true, "Tide cache"], [true, "Tide cache"]]);
});

test("no Send to job is drawn where no Job holds the pull request", async () => {
  await mount(noted());
  await press("Files");
  await noteOn("+let days = 2;", "Name the unit.");
  expect(sendButton()).toBeUndefined();
});

test("the Send to job menu lists the other running Jobs, and picking one sends the notes there and marks them sent", async () => {
  const job = heldJob([
    { id: "job-2", title: "Cache key" },
    { id: "job-3", title: "Old one", dead: "ended" },
  ]);
  await mountHeld(noted(), job.held);
  await press("Files");
  await noteOn("+let days = 2;", "Name the unit.");
  await act(async () => document.querySelector<HTMLButtonElement>('button[aria-label="Send to a job"]')!.click());
  const menu = document.querySelector('[role="menu"][aria-label="Send to a job"]')!;
  // The Job that holds it is the press itself, a Job that ended is not offered, and the one left is.
  expect([...menu.querySelectorAll('[role="menuitem"]')].map((one) => one.textContent)).toEqual(["Cache key"]);
  await act(async () => menu.querySelector<HTMLButtonElement>('[role="menuitem"]')!.click());
  expect(job.send).toHaveBeenCalledTimes(1);
  expect(job.send.mock.calls[0]![0]).toBe("job-2");
  expect(job.send.mock.calls[0]![1].text).toContain("1. src/tide/tables.rs:1 [note-1]: Name the unit.");
  expect(sendButton()).toBeUndefined();
});

test("a note the Job resolved is folded away when Files is entered again, and Show resolved draws it", async () => {
  const job = heldJob();
  await mountHeld(noted(), job.held);
  await press("Files");
  await noteOn("+let days = 2;", "Name the unit.");
  await act(async () => sendButton()!.click());
  expect(sections()[0]!.textContent).toContain("Name the unit.");
  // The Job calls resolve_pull_request_note while the panel is open; Bridge reads again the next time Files is entered.
  resolvePullRequestNoteInMock("tidewater", 418, "note-1");
  await press("Conversation");
  await press("Files");
  expect(sections()[0]!.textContent).not.toContain("Name the unit.");
  const toggle = () => [...filesPanel()!.ownerDocument.querySelectorAll<HTMLButtonElement>("button")].find((one) => one.textContent === "Show resolved (1)");
  await act(async () => toggle()!.click());
  expect(sections()[0]!.textContent).toContain("Name the unit.");
  expect(sections()[0]!.querySelector('[aria-label="Note for the job"]')?.textContent).toContain("Resolved by Tide cache");
});

// ---- Ready for review: the forge assigns reviewers a few seconds after the press, so the panel reads again ----------

const DRAFT: PullRequestRead = { ...READ, facts: { ...READ.facts, state: "draft" } };
const ASSIGNED: PullRequestRead = { ...READ, page: { ...PAGE, reviewers: [{ login: "ines", kind: "person" }] } };
const plate = () => document.querySelector('section[aria-label="Reviewers"]');

/** A forge that names reviewers from its `after`th read on, counting each read. */
function assigning(after: number) {
  const seen = { reads: 0 };
  const pullRequest = async (): Promise<PullRequestRead> => (++seen.reads > after ? ASSIGNED : DRAFT);
  return { seen, reads: { ...readsOf(pullRequest), actOnPullRequest: async () => true } satisfies PanelReads };
}

/** Time passing a second at a time, each drawn before the next, as it would be: one jump would fire every timer before a read could land. */
const later = async (ms: number) => {
  for (let at = 0; at < ms; at += 1_000) await act(async () => void (await vi.advanceTimersByTimeAsync(1_000)));
};

test("Ready for review reads the pull request again, and the reviewers the forge assigns after it appear without the panel being opened again", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  try {
    const tide = assigning(1);
    await mount(tide.reads);
    expect(plate()).toBeNull();
    await act(async () => pressed("Ready for review")!.click());
    await later(2_000);
    expect(plate()?.textContent).toContain("ines");
  } finally {
    vi.useRealTimers();
  }
});

test("the reads after Ready for review stop once reviewers show, and a forge that names none is read four times more and no further", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  try {
    const named = assigning(1);
    await mount(named.reads);
    await act(async () => pressed("Ready for review")!.click());
    await later(60_000);
    expect(named.seen.reads).toBe(2);
    act(() => root?.unmount());
    host?.remove();
    forgetPanel();
    forgetReadings();
    const none = assigning(Infinity);
    await mount(none.reads);
    await act(async () => pressed("Ready for review")!.click());
    await later(60_000);
    expect(none.seen.reads).toBe(5);
  } finally {
    vi.useRealTimers();
  }
});
