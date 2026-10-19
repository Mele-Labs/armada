// The first Job on a folder with no Manifest, and Armada keeping how the repository runs, so the next agent does not guess.
// **The agent is a script**, and the thread is the only part of this that is: the folders Claude Code kept sessions for, how a repository runs and where that is
// kept are answered by the onboarding slice, through the same calls a real Fleet gets. A new project is a folder made, then a Job whose first event is `git init`.
// Add takes the folders in, with one terminal Claude Code session shown as a Job once the mod is installed.

import type { Session, SessionRow, WorkFile } from "@armada/screens/src/draft/sessions";

import { COMMANDS, sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

/** The folder the mock's picker answers with. */
export const FOLDER = "/Users/user/code/my-app";
const NAME = "my-app";
export const REQUEST = "Add a dark mode toggle to settings";

/** The repository's id, which a Job names it by: the folder's own name. */
export const manifestOf = (folder: string): string => folder.split("/").pop() ?? NAME;

/** The folder Claude Code is said to hold sessions for, beside the picker's: the onboarding slice lists them. */
export const HOME = "/Users/user/code";

/** The session running in a terminal, which is a Job once the mod is installed. */
const TERMINAL = "fr-terminal";
const terminal: Session = {
  id: TERMINAL,
  terminal: true,
  title: "Rate limit on the login route",
  cwd: `${HOME}/api`,
  model: "opus",
  effort: "medium",
  mode: "ask",
  commands: COMMANDS,
  turn: { state: "working" },
  lastTurn: "14:02",
  lastTurnAt: new Date().toISOString(),
  rows: [
    { id: "frt-1", at: "14:01:12", kind: "message", from: { kind: "you" }, text: "Rate limit the login route" },
    { id: "frt-2", at: "14:01:20", kind: "tool", text: "Read src/routes/login.ts" },
    { id: "frt-3", at: "14:01:31", kind: "tool", text: "Edit src/routes/login.ts" },
  ],
  attachments: [],
};

const SETTINGS = "src/settings/Settings.tsx";
const PROVIDER = "src/theme/ThemeProvider.tsx";
const FILES: readonly WorkFile[] = [
  { path: PROVIDER, act: "write" },
  { path: SETTINGS, act: "edit" },
  { path: "src/theme/tokens.css", act: "edit" },
  { path: "src/settings/Settings.test.tsx", act: "edit" },
];

/** The Sessions store for the walk `first-run`: nothing open, and a Session started from the prompt plays its script. */
export function firstRunStore(): SessionsStore {
  const inner = sessionsStore([terminal], [], new Map(), []);
  const mine = new Set<string>();
  /** The folder each Job of the walk started in. */
  const startedIn = new Map<string, string>();
  // The terminal session is shown once the mod is installed.
  let installed = false;
  let shown: { from: readonly Session[]; list: readonly Session[] } | undefined;
  const timers = new Set<number>();
  let parent: string | undefined;
  let rows = 0;
  let clock = 0;

  const bump = () => inner.edit(parent ?? TERMINAL, (one) => ({ ...one }));
  const stamp = () => {
    clock += 9;
    return `14:${String(2 + Math.floor(clock / 60)).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
  };
  const row = (make: (id: string, at: string) => SessionRow): SessionRow => make(`fr${(rows += 1)}`, stamp());
  const tool = (text: string) => row((id, at) => ({ id, at, kind: "tool", text }));
  const said = (text: string) => row((id, at) => ({ id, at, kind: "message", from: { kind: "agent" }, text }));
  /** A command that exited, as the thread folds it: the exit and what it printed. */
  const ran = (command: string, code: number | null, tail: string) => row((id, at) => ({ id, at, kind: "ran", command, code, tail }));
  const add = (id: string, more: SessionRow[]) => inner.edit(id, (one) => ({ ...one, rows: [...one.rows, ...more] }));
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const tasks = (done: number): NonNullable<Session["work"]>["tasks"] =>
    ["Add a theme setting", "Switch the colours from the setting", "Add the toggle to Settings", "Run the tests"].map((subject, at) => ({
      id: `t${at + 1}`,
      subject,
      state: at < done ? "done" : at === done ? "working" : "open",
    }));
  const working = (id: string, done: number, files: readonly WorkFile[], doing: "write" | "run", file?: string) =>
    inner.edit(id, (one) => ({ ...one, work: { tasks: tasks(done), field: [{ id: "main", path: file === undefined ? "src/settings" : file.split("/").slice(0, 2).join("/"), doing, ...(file === undefined ? {} : { file }) }], files } }));

  /** The folder made a repository, then the edits: what the Job does with no Manifest. A new project has nothing to read yet. */
  const opening = (id: string) => {
    const folder = startedIn.get(id) ?? FOLDER;
    const fresh = folder !== FOLDER;
    inner.edit(id, (one) => ({ ...one, title: "Dark mode toggle", cwd: folder, standing: "working" }));
    after(500, () => {
      add(id, [tool(`git init ${folder}`), ...(fresh ? [tool("Write package.json")] : [tool("Read src/settings/Settings.tsx"), tool("Read src/theme/tokens.css")])]);
      working(id, 0, [], "write", SETTINGS);
    });
    after(1300, () => {
      add(id, [tool(`Write ${PROVIDER}`), tool(`${fresh ? "Write" : "Edit"} ${SETTINGS}`), tool("Edit src/theme/tokens.css")]);
      working(id, 2, FILES.slice(0, 3), "write", SETTINGS);
    });
  };

  /** The end of the turn: the tasks done and the Session idle. */
  const finish = (id: string) =>
    inner.edit(id, (one) => ({
      ...one,
      turn: { state: "idle" },
      standing: "replied",
      lastTurn: stamp().slice(0, 5),
      lastTurnAt: new Date().toISOString(),
      work: { tasks: tasks(4), field: [], files: FILES },
    }));

  /** The tests and the dev server, each guessed in turn, then the end of the turn. */
  const moments: (() => void)[] = [
    () => {
      if (parent === undefined) return;
      add(parent, [
        ran("npm test", 1, 'npm error Missing script: "test"\nnpm error\nnpm error To see a list of scripts, run:\nnpm error   npm run'),
        ran("pnpm test", 0, " ✓ src/settings/Settings.test.tsx\n ✓ src/theme/theme.test.ts"),
        ran("pnpm dev", null, "  VITE ready\n  Local:   http://localhost:5173/"),
        said("Tests pass with pnpm test. The dev server starts on :5173."),
      ]);
      working(parent, 3, FILES, "run");
    },
    () => {
      if (parent === undefined) return;
      add(parent, [said("Settings has a dark mode toggle. The theme follows the system until it is switched.")]);
      finish(parent);
    },
  ];
  let moment = 0;

  /** A Job after the first, which Armada already knows the repository for: it leases a slot and cuts a branch, and keeps working. */
  const later = (id: string) => {
    inner.edit(id, (one) => ({ ...one, title: "Docs dark mode", cwd: startedIn.get(id) ?? FOLDER, standing: "working", context: { tokens: 96_300, window: 1_000_000 } }));
    after(500, () => {
      add(id, [tool("Read docs/theme.md"), row((rid, at) => ({ id: rid, at, kind: "lease", slot: 2, branch: "docs/dark-mode" })), tool("Edit docs/theme.md")]);
      inner.edit(id, (one) => ({ ...one, attachments: [...one.attachments, { kind: "slot", slot: 2 }, { kind: "branch", name: "docs/dark-mode", slot: 2 }] }));
    });
  };

  modInstalled = () => {
    installed = true;
    bump();
  };

  const store: SessionsStore = {
    ...inner,
    // The terminal session is in the list once its mod is installed, and the list is the same array until it changes.
    get: () => {
      const all = inner.get();
      if (shown?.from !== all) shown = { from: all, list: installed ? all : all.filter((one) => one.id !== TERMINAL) };
      return shown.list;
    },
    start(tag, root, request, tuning, wait) {
      const id = inner.start(tag, root, request, tuning, wait);
      if (typeof id === "string") {
        mine.add(id);
        if (root !== undefined) startedIn.set(id, root);
        // What a Job just started carries in its header: the repository it is in, a context window with the agent's own prompt in it, and the Loadout.
        // One started with nothing to say is named for its folder until the agent titles it.
        inner.edit(id, (one) => ({
          ...one,
          manifestId: manifestOf(root ?? FOLDER),
          ...(request === undefined && root !== undefined ? { title: manifestOf(root), cwd: root } : {}),
          context: { tokens: 18_400, window: 1_000_000 },
          loadout: { knows: 214, told: 6 },
        }));
      }
      return id;
    },
    send(id, sent) {
      if (!mine.has(id)) return inner.send(id, sent);
      const first = parent === undefined;
      if (first) parent = id;
      inner.edit(id, (one) => ({
        ...one,
        rows: [...one.rows, row((rid, at) => ({ id: rid, at, kind: "message", from: { kind: "you" }, text: sent.text }))],
        turn: { state: "working" },
      }));
      if (first) opening(id);
      else later(id);
    },
    move: async () => undefined,
    later() {
      moments[moment]?.();
      moment += 1;
    },
    dispose: () => {
      modInstalled = undefined;
      timers.forEach((one) => window.clearTimeout(one));
      inner.dispose();
    },
  };
  return store;
}

/** What the install of the mod does in this window: the terminal session is shown as a Job. The store of the window that mounted last answers. */
let modInstalled: (() => void) | undefined;
export const installedMod = (): void => modInstalled?.();
