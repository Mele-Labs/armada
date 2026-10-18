// The first Session on a folder with no Manifest, and what is offered after it. Mock only: Fleet starts no Session without a Manifest yet
// and makes no suggestions. **The agent is a script**; the suggestions are written here as Fleet would derive them, each from a fact it
// saw (a command that exited, a port that was listened on, a remote that is set), never from what the agent said.
// Two takes of one offer, by `?take=`: the thread's last card (`thread`, the default), or items in the Needs you band (`band`).

import type { Session, SessionNext, SessionRow, WorkFile } from "@armada/screens/src/draft/sessions";

import { sessionsStore } from "./sessions/script";
import type { SessionsStore } from "./sessions/script";

export type Take = "thread" | "band";

/** Which take the page is on. */
export const firstRunTake = (): Take => (new URLSearchParams(window.location.search).get("take") === "band" ? "band" : "thread");

/** The folder the mock's picker answers with. */
export const FOLDER = "/Users/user/code/my-app";
const NAME = "my-app";
export const REQUEST = "Add a dark mode toggle to settings";

/** What the Session did, seen by Fleet, and the suggestion each fact gives. */
const ITEMS: SessionNext["items"] = [
  { id: "check", text: "Check every change with pnpm test", fact: "pnpm test ran and exited 0" },
  { id: "command", text: "Keep pnpm dev as a Command", fact: "pnpm dev ran in the background and listened on :5173" },
  { id: "pull-request", text: "Open a pull request", fact: "origin is github.com/user/my-app" },
];

const SETTINGS = "src/settings/Settings.tsx";
const PROVIDER = "src/theme/ThemeProvider.tsx";
const FILES: readonly WorkFile[] = [
  { path: PROVIDER, act: "write" },
  { path: SETTINGS, act: "edit" },
  { path: "src/theme/tokens.css", act: "edit" },
  { path: "src/settings/Settings.test.tsx", act: "edit" },
];

const brief = (): string =>
  [`Write armada.yml for ${NAME} from what Fleet saw in the Session before this one:`, ...ITEMS.map((one) => `- ${one.text}. ${one.fact}.`)].join("\n");

/** The Sessions store for the walk `first-run`: nothing open, and a Session started from the prompt plays its script. */
export function firstRunStore(take: Take = firstRunTake()): SessionsStore {
  const inner = sessionsStore([], [], new Map(), []);
  const mine = new Set<string>();
  const timers = new Set<number>();
  let parent: string | undefined;
  let rows = 0;
  let clock = 0;
  // What a person put off for a repository stays off until the page is reloaded.
  let hidden = false;

  const stamp = () => {
    clock += 9;
    return `14:${String(2 + Math.floor(clock / 60)).padStart(2, "0")}:${String(clock % 60).padStart(2, "0")}`;
  };
  const row = (make: (id: string, at: string) => SessionRow): SessionRow => make(`fr${(rows += 1)}`, stamp());
  const tool = (text: string) => row((id, at) => ({ id, at, kind: "tool", text }));
  const said = (text: string) => row((id, at) => ({ id, at, kind: "message", from: { kind: "agent" }, text }));
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

  /** The tests and the dev server, then the end of the turn and what Fleet offers. */
  const moments: (() => void)[] = [
    () => {
      if (parent === undefined) return;
      add(parent, [tool("pnpm test"), tool("pnpm dev"), said("pnpm test passed. The dev server is on :5173.")]);
      working(parent, 3, FILES, "run");
    },
    () => {
      if (parent === undefined) return;
      add(parent, [said("Settings has a dark mode toggle. The theme follows the system until it is switched.")]);
      inner.edit(parent, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "replied",
        lastTurn: stamp().slice(0, 5),
        lastTurnAt: new Date().toISOString(),
        work: { tasks: tasks(4), field: [], files: FILES },
        ...(hidden ? {} : { next: { repository: NAME, at: take, items: ITEMS } }),
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
    next: {
      setUp(id) {
        const child = inner.start(undefined, FOLDER, brief());
        if (typeof child !== "string") return;
        mine.add(child);
        const title = `Set up ${NAME}`;
        inner.edit(child, (one) => ({
          ...one,
          title,
          cwd: FOLDER,
          turn: { state: "working" },
          standing: "working",
          attachments: [{ kind: "started_by", id }],
          rows: [row((rid, at) => ({ id: rid, at, kind: "message", from: { kind: "session", id, title: inner.get().find((it) => it.id === id)?.title ?? id }, text: brief() }))],
          work: { tasks: [{ id: "c1", subject: "Write armada.yml", state: "working" }], field: [], files: [] },
        }));
        inner.edit(id, (one) => {
          const { next: _next, ...rest } = one;
          void _next;
          return { ...rest, attachments: [...one.attachments, { kind: "started_to", id: child }], rows: [...one.rows, row((rid, at) => ({ id: rid, at, kind: "started", session: { id: child, title }, fork: false }))] };
        });
        after(1800, () => {
          add(child, [tool("Read package.json"), tool("Write armada.yml"), said("armada.yml has the Check pnpm test and the Command pnpm dev.")]);
          inner.edit(child, (one) => ({ ...one, turn: { state: "idle" }, standing: "replied", lastTurnAt: new Date().toISOString(), work: { tasks: [{ id: "c1", subject: "Write armada.yml", state: "done" }], field: [], files: [] } }));
        });
      },
      notNow(id) {
        hidden = true;
        inner.edit(id, (one) => {
          const { next: _next, ...rest } = one;
          void _next;
          return rest;
        });
      },
    },
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      inner.dispose();
    },
  };
  return store;
}
