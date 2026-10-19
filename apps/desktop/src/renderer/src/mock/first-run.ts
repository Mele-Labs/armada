// The first Job on a folder with no Manifest, and Armada keeping how the repository runs, so the next agent does not guess.
// Mock only: Fleet starts no Session without a Manifest yet and keeps no such memory. **The agent is a script**; each fact is written here as Fleet would derive
// it, from a command that exited and a port that was listened on, never from what the agent said. What passed is kept under the place the repository was added with.
// The stage's other two starts are served here too: a new project (a folder made, then a Job whose first event is `git init`), and the folders Claude Code
// kept sessions for, which Add takes in, with one terminal Claude Code session shown as a Job where the mod is installed and how `my-app` runs already found.

import type { RunsPlace } from "@armada/components";
import type { ClaudeFolder, Session, SessionRow, WorkFile } from "@armada/screens/src/draft/sessions";

import { keptRuns, type RunsRead } from "./runs-kept";
import { COMMANDS, sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

/** The folder the mock's picker answers with. */
export const FOLDER = "/Users/user/code/my-app";
const NAME = "my-app";
export const REQUEST = "Add a dark mode toggle to settings";

/** Where the place is drawn: in the Job's header, at the top of its thread, or in the message box's toolbar. The walks read it from `?take=`. */
export type PlaceTake = "header" | "thread" | "toolbar";
export function placeTake(search: string = typeof window === "undefined" ? "" : window.location.search): PlaceTake {
  const asked = new URLSearchParams(search).get("take");
  return asked === "thread" || asked === "toolbar" ? asked : "header";
}

/** What Fleet saw run, by job. */
const JOBS: RunsRead["jobs"] = [
  { id: "tests", job: "Tests", command: "pnpm test" },
  { id: "dev", job: "Dev server", command: "pnpm dev", port: 5173 },
];

/** Where the mock's folder picker answers for a new project's place, and the folders Claude Code is said to hold sessions for, by hours since the last. */
const HOME = "/Users/user/code";
const PAST: readonly { path: string; hours: number; git: boolean }[] = [
  { path: FOLDER, hours: 2, git: true },
  { path: `${HOME}/api`, hours: 3 * 24, git: true },
  { path: `${HOME}/site`, hours: 12 * 24, git: true },
  { path: `${HOME}/scratch`, hours: 41 * 24, git: false },
];

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
  /** The folder each Job of the walk started in, and the folders the person made new. */
  const startedIn = new Map<string, string>();
  /** The first Job on a repository asks where the runs are kept: the folders it is for and the place it stands on, until its first message keeps it. */
  const asks = new Map<string, { roots: readonly string[]; place: RunsPlace }>();
  const firsts = new Set<string>();
  /** The place each folder keeps its runs, once it is kept. */
  const placed = new Map<string, RunsPlace>();
  const folderOf = (id: string) => startedIn.get(id) ?? FOLDER;
  const made = new Set<string>();
  // Claude Code's folders were taken in, and the terminal session is shown once its mod is installed.
  let adopted = false;
  let installed = false;
  let shown: { from: readonly Session[]; list: readonly Session[] } | undefined;
  const timers = new Set<number>();
  let parent: string | undefined;
  let rows = 0;
  let clock = 0;
  // The turn has ended, so what passed is kept.
  let ended = false;

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
    const fresh = made.has(folder);
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

  /** What Fleet saw pass of how the repository runs, once there is something to show: the Job's last turn, or the sessions Claude Code kept. */
  const read = (root: string): RunsRead | undefined => {
    if (root !== FOLDER) return undefined;
    if (adopted) return { repository: NAME, source: "From Claude Code sessions", jobs: JOBS };
    return ended ? { repository: NAME, source: "From the last Job", jobs: JOBS } : undefined;
  };

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
      ended = true;
      finish(parent);
    },
  ];
  let moment = 0;

  const kept = keptRuns(read, bump);
  const asking: NonNullable<SessionsStore["runs"]>["asking"] = {
    begin(id, roots) {
      asks.set(id, { roots, place: { id: "mac" } });
      firsts.add(id);
      const folder = roots[0] ?? FOLDER;
      inner.edit(id, (one) => ({ ...one, title: folder.split("/").pop() ?? NAME, cwd: folder }));
    },
    of: (id) => asks.get(id)?.place ?? (mine.has(id) ? (placed.get(folderOf(id)) ?? { id: "mac" }) : undefined),
    first: (id) => firsts.has(id),
    set(id, place) {
      const ask = asks.get(id);
      if (ask !== undefined) asks.set(id, { ...ask, place });
      else if (mine.has(id)) {
        placed.set(folderOf(id), place);
        kept.place(folderOf(id), place);
      }
      inner.edit(id, (one) => ({ ...one }));
    },
  };

  const store: SessionsStore = {
    ...inner,
    // The terminal session is in the list once its mod is installed, and the list is the same array until it changes.
    get: () => {
      const all = inner.get();
      if (shown?.from !== all) shown = { from: all, list: installed ? all : all.filter((one) => one.id !== TERMINAL) };
      return shown.list;
    },
    begin: {
      home: HOME,
      create: async (parent, name) => {
        const path = `${parent}/${name}`;
        made.add(path);
        return path;
      },
      found: async (): Promise<readonly ClaudeFolder[]> => PAST.map((one) => ({ path: one.path, git: one.git, lastAt: new Date(Date.now() - one.hours * 3_600_000).toISOString() })),
      adopt: async (_paths, mod) => {
        adopted = true;
        installed = mod;
        bump();
      },
    },
    start(tag, root, request, tuning, wait) {
      const id = inner.start(tag, root, request, tuning, wait);
      if (typeof id === "string") {
        mine.add(id);
        if (root !== undefined) startedIn.set(id, root);
      }
      return id;
    },
    send(id, sent) {
      if (!mine.has(id)) return inner.send(id, sent);
      // The first message keeps the place the Job stood on, and the question goes.
      const ask = asks.get(id);
      if (ask !== undefined) {
        asks.delete(id);
        for (const root of ask.roots) {
          placed.set(root, ask.place);
          kept.place(root, ask.place);
        }
      }
      const first = parent === undefined;
      if (first) parent = id;
      inner.edit(id, (one) => ({
        ...one,
        rows: [...one.rows, row((rid, at) => ({ id: rid, at, kind: "message", from: { kind: "you" }, text: sent.text }))],
        turn: { state: "working" },
      }));
      if (first) opening(id);
    },
    later() {
      moments[moment]?.();
      moment += 1;
    },
    runs: { ...kept, asking, take: placeTake() },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      inner.dispose();
    },
  };
  return store;
}
