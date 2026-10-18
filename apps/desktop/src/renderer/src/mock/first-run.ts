// The first Session on a folder with no Manifest, and the offer that follows: Armada remembering how the repository runs, so the next agent does not guess.
// Mock only: Fleet starts no Session without a Manifest yet and keeps no such memory. **The agent is a script**; each fact is written here as Fleet would derive
// it, from a command that exited and a port that was listened on, never from what the agent said.
// Three takes of one offer, by `?take=`: the Session asks at the end of its turn (`ask`, the default), the next Session's start offers it (`start`), or the
// repository holds it, marked in the picker (`repo`).

import type { RepositoryRuns, Session, SessionQuestion, SessionRow, WorkFile } from "@armada/screens/src/draft/sessions";

import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

export type Take = "ask" | "start" | "repo";

/** Which take the page is on. */
export const firstRunTake = (): Take => {
  const take = new URLSearchParams(window.location.search).get("take");
  return take === "start" || take === "repo" ? take : "ask";
};

/** The folder the mock's picker answers with. */
export const FOLDER = "/Users/user/code/my-app";
const NAME = "my-app";
export const REQUEST = "Add a dark mode toggle to settings";

/** What Fleet saw run, by job. Lint ran nowhere. */
const JOBS: readonly { id: string; job: string; command?: string; port?: number }[] = [
  { id: "tests", job: "Tests", command: "pnpm test" },
  { id: "dev", job: "Dev server", command: "pnpm dev", port: 5173 },
  { id: "lint", job: "Lint" },
];

const KEPT = "Tests pnpm test, Dev server pnpm dev :5173";

/** The Session's own question at the end of its turn, in the shape an agent's question takes. */
const QUESTION: SessionQuestion = {
  question: `npm test failed and pnpm test passed. The dev server is pnpm dev on :5173. Remember these for ${NAME}?`,
  header: "Remember",
  multi_select: false,
  options: [
    { label: "Remember", description: "The next Job starts from them" },
    { label: "No", description: "The next Job finds them again" },
  ],
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
export function firstRunStore(take: Take = firstRunTake()): SessionsStore {
  const inner = sessionsStore([], [], new Map(), []);
  const mine = new Set<string>();
  const timers = new Set<number>();
  let parent: string | undefined;
  let rows = 0;
  let clock = 0;
  // The turn has ended, and what Fleet saw is kept for the next Session.
  let ended = false;
  let kept = false;

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

  /** The folder made a repository, then the edits: what the Session does with no Manifest. */
  const opening = (id: string) => {
    inner.edit(id, (one) => ({ ...one, title: "Dark mode toggle", cwd: FOLDER, standing: "working" }));
    after(500, () => {
      add(id, [tool(`git init ${FOLDER}`), tool("Read src/settings/Settings.tsx"), tool("Read src/theme/tokens.css")]);
      working(id, 0, [], "write", SETTINGS);
    });
    after(1300, () => {
      add(id, [tool(`Write ${PROVIDER}`), tool(`Edit ${SETTINGS}`), tool("Edit src/theme/tokens.css")]);
      working(id, 2, FILES.slice(0, 3), "write", SETTINGS);
    });
  };

  /** The end of the turn: the tasks done and the Session idle. Held on a question until it is answered. */
  const finish = (id: string) =>
    inner.edit(id, (one) => ({
      ...one,
      turn: { state: "idle" },
      standing: "replied",
      lastTurn: stamp().slice(0, 5),
      lastTurnAt: new Date().toISOString(),
      work: { tasks: tasks(4), field: [], files: FILES },
    }));

  /** What Fleet saw of how the repository runs, once there is something to show: the take decides where it is drawn. */
  const seen = (): RepositoryRuns | undefined =>
    (take === "ask" ? kept : ended)
      ? {
          repository: NAME,
          source: "From the last Job",
          inPicker: take === "repo",
          rows: JOBS.filter((one) => take === "repo" || one.id !== "lint").map((one) => ({
            ...one,
            state: one.id === "lint" ? "not_seen" : kept ? "remembered" : "seen",
          })),
        }
      : undefined;

  /** The tests and the dev server, each guessed in turn, then the end of the turn and what Fleet kept. */
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
      if (take !== "ask") return finish(parent);
      // Held on the question; the turn ends when it is answered.
      inner.edit(parent, (one) => ({
        ...one,
        asked: { command: QUESTION.question, call: "first-run-q1", offers: ["allow_once", "refuse"], questions: [QUESTION] },
        lastTurnAt: new Date().toISOString(),
        work: { tasks: tasks(4), field: [], files: FILES },
      }));
    },
  ];
  let moment = 0;

  const store: SessionsStore = {
    ...inner,
    start(tag, root, request, tuning, wait) {
      const id = inner.start(tag, root, request, tuning, wait);
      if (typeof id === "string") mine.add(id);
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
    },
    later() {
      moments[moment]?.();
      moment += 1;
    },
    answer(id, answer, answers) {
      if (!mine.has(id) || inner.get().find((one) => one.id === id)?.asked?.questions === undefined) return inner.answer(id, answer, answers);
      inner.edit(id, (one) => {
        const { asked: _asked, ...rest } = one;
        void _asked;
        return rest;
      });
      const chose = answer !== "refuse" && answers?.[0]?.chosen[0] === "Remember";
      // What was chosen is the person's own message, and what was kept is the next line.
      add(id, [row((rid, at) => ({ id: rid, at, kind: "message", from: { kind: "you" }, text: (answers ?? []).map((one) => one.chosen.join(", ")).join("\n") }))]);
      if (chose) {
        kept = true;
        add(id, [row((rid, at) => ({ id: rid, at, kind: "change", what: `How ${NAME} runs`, from: "Not remembered", to: KEPT }))]);
      }
      finish(id);
    },
    runs: {
      of: (root) => (root === FOLDER ? seen() : undefined),
      remember: () => {
        kept = true;
        // The list is a new array, so what draws from it draws again.
        if (parent !== undefined) inner.edit(parent, (one) => ({ ...one }));
      },
    },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      inner.dispose();
    },
  };
  return store;
}
