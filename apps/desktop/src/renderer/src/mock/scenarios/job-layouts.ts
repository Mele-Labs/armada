// A Job's area in panes the person arranges: five Jobs on the Cockpit, each with a story, a shell to open in a Terminal tab and a browser to open in a Browser tab. Two layouts are saved
// already, QA split and Review, beside Default. Each time the walk lets time pass, the second Job opens a browser page itself, which joins the pane that holds a browser, or opens
// as a split on the right. Over `job-story`. The walk `jobLayouts` plays it. Mock only: nothing is kept.

import type { ModChecked, ModSummary, PullReviewLists, PullReviewRow, WorktreeSlot } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { seedJobBrowser } from "../../job-browser/store";
import { seedBuilt } from "../../job-panes/custom";
import { forgetPanes, jobOpens, look, peek, seedLayouts } from "../../job-panes/store";
import { arrive, build, panesOf } from "../../job-panes/tree";
import { resetCheckout, shopPage } from "../job-browser-page";
import { MOCK_PATCH, MOCK_PATCH_LOG, MOCK_PATCH_TOOLTIP, patchesOf } from "../job-layouts-patches";
import { unanswered, type Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { sharedPanelReads } from "../shared-panel-reads";
import { reviewQueueModule } from "./job-layouts-tab";
import { sJobStory } from "./job-story";
import { jobAt } from "../workspaces-lanes-data";
import { throughWire } from "../workspaces-lanes-wire";
import { seedSystem } from "../slices/session-system";
import { disposeShells, shellApi } from "./session-shell";

/** The Jobs `job-story` serves. */
const JOBS = ["story-1", "story-2", "story-3", "story-4", "story-5"];
/** The Job that opens a page by itself. */
const OPENS = "story-2";
/** The Job with changes to choose between, and the Job it started. */
const CHANGES = "story-1";
const SUB = "story-4";
const KB = 1024;
const GB = KB * KB * KB;
const SHOP = ["http://localhost:5173/cart", "http://localhost:5173/checkout"];

// ---- The Custom tab's Mod Job ---------------------------------------------------------------------------------------------------------------------------
// A description sent from a Custom tab starts a Mod Job. It reads and writes for a few seconds (or until the walk lets time pass), then Fleet's mod list holds the tab it
// wrote, `review-queue`, a mod of kind `tab`, and the tab draws the view. The pull requests waiting on him are what `getReviewLists` answers, across three repositories.

const MOD_NAME = "review-queue";
const MOD_DIR = "~/Library/Application Support/Armada/mods/review-queue";
const waiting = (repository: string, number: number, title: string, branch: string, author: string, minutes: number, more: Partial<PullReviewRow> = {}): PullReviewRow => ({
  manifest_id: repository.split("/")[1]!,
  repository,
  number,
  title,
  branch,
  address: `https://git.example/${repository}/pull/${number}`,
  draft: false,
  checks: "passed",
  review: "required",
  head: `${number}`.padStart(7, "a"),
  author,
  updated_at: new Date(Date.now() - minutes * 60_000).toISOString(),
  ...more,
});
const WAITING: PullReviewRow[] = [
  waiting("northwind/storefront", 812, "Hold the cart total while a coupon applies", "cart/coupon-total", "dana", 22),
  waiting("northwind/storefront", 809, "Retry the address lookup once on a timeout", "checkout/address-retry", "ravi", 95, { checks: "running" }),
  waiting("northwind/billing-api", 377, "Round tax per line, not per order", "tax/per-line", "mei", 140, { checks: "failed" }),
  waiting("northwind/billing-api", 371, "Reject a refund larger than the charge", "refunds/cap", "ola", 410),
  waiting("northwind/design-system", 96, "Give the toast a close button", "toast/close", "sam", 700, { review: "changes_requested" }),
];

// The Jobs the first Job started, in the states a Jobs tab draws: running, needs you, merged, failed and waiting, beside the one in the story.
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const said = (id: string, text: string): SessionRow => ({ id, at: "13:40:00", kind: "message", from: { kind: "agent" }, text });
const started = (id: string, title: string, more: Partial<Session>): Session => ({
  id,
  title,
  manifestId: repository().manifest!.id,
  cwd: repository().root,
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  rows: [],
  attachments: [],
  ...more,
});
const LINKED: Session[] = [
  started("lay-run", "Retry the lease when the pool is contended", { turn: { state: "working" }, standing: "working", lastTurnAt: ago(1), rows: [said("l1", "The back off is written. Running the lease tests.")] }),
  started("lay-ask", "Name the attempt in the bay tooltip", { lastTurnAt: ago(6), asked: { command: "git push origin armada/retry-lease-tooltip" }, rows: [said("l2", "The tooltip names the attempt. Asking to push the branch.")] }),
  started("lay-merged", "Show the retry count on the bay tile", {
    lastTurnAt: ago(95),
    rows: [said("l3", "Merged. The bay tile shows the count.")],
    attachments: [{ kind: "pull_request", number: 2366, title: "Show the retry count on the bay tile", branch: "armada/retry-count", address: "https://git.example/armada/pull/2366", checks: { state: "passed" }, state: "merged", auto: false }],
  }),
  started("lay-failed", "Cap how long the lease backs off", { standing: "stuck", lastTurnAt: ago(40), rows: [said("l4", "The cap test fails on the second attempt and I cannot find why.")] }),
  started("lay-waiting", "Write up the retry settings", {
    lastTurnAt: ago(12),
    waits: { on: { id: "lay-run", title: "Retry the lease when the pool is contended" }, when: "lands", base: "main", needs: "the retry to land", brief: "Write up the retry settings once the retry lands.", by: "you" },
  }),
];

// ---- A Mod Job held on a question --------------------------------------------------------------------------------------------------------------------
// The fifth Job's panes hold a Pull queue tab, and the Mod Job changing it has asked to write the tab's file. It has no row in the Jobs list: the tab wears the needs-you
// mark, its card holds the ask, and the fifth Job's row wears the mark too. Allow once lets it go on with the change. On a real Fleet, the Mod Job's start would carry the
// Job id and the tab id, which `modFor` stands in for here.
const HOLDS = "story-5";
/** The tab the fifth Job holds is a mod of its own, so the Custom tab a walk builds beside it is the only one that reads `review-queue`. */
const ASK_MOD = "pull-queue";
const ASK_JOB = "mod-ask";
const askJobOf = (tab: string): Session =>
  started(ASK_JOB, "List the oldest first", {
    mod: true,
    modFor: { job: HOLDS, tab },
    turn: { state: "working" },
    standing: "working",
    lastTurnAt: ago(0),
    rows: [
      { id: "ask-1", at: "13:40:00", kind: "message", from: { kind: "you" }, text: "List the oldest first, and show how long each has waited" },
      { id: "ask-2", at: "13:40:03", kind: "tool", text: `Read ${MOD_DIR}/tab.js` },
    ],
    asked: { command: `Write ${MOD_DIR}/tab.js` },
  });
let askTab = "";
let askJob: Session | undefined;
const askListeners = new Set<() => void>();
const editAsk = (change: (one: Session) => Session) => {
  if (askJob === undefined) return;
  askJob = change(askJob);
  askListeners.forEach((on) => on());
};

let handle: { publish: (change: { mods: { mods: ModSummary[] } }) => void } | undefined;
let modStore: SessionsStore | undefined;
let mods: ModSummary[] = [];
/** How far the Mod Job has got in rewriting the tab's module, which `validate_mod` answers. */
let revision = 0;
const turns = new Map<string, () => void>();
const timers = new Set<number>();
const changeTimers = new Set<number>();
/** The Jobs a Custom tab started, which a message to builds the tab again. */
const modJobs = new Set<string>();
let rowSeq = 0;

/** The Mod Job's turn: rows as it reads and writes, then the mod in Fleet's list. Idempotent, so a walk's time passing may end it early. */
function modTurn(store: SessionsStore, id: string, said: string): void {
  const now = () => new Date().toISOString();
  const row = (kind: "tool" | "agent", text: string): SessionRow => (kind === "tool" ? { id: `mod-${(rowSeq += 1)}`, at: now(), kind: "tool", text } : { id: `mod-${(rowSeq += 1)}`, at: now(), kind: "message", from: { kind: "agent" }, text });
  const steps: [number, () => void][] = [
    [700, () => store.edit(id, (one) => ({ ...one, rows: [...one.rows, row("tool", "Read packages/protocol/src/pull-reviews.ts")] }))],
    [1500, () => store.edit(id, (one) => ({ ...one, rows: [...one.rows, row("tool", "Read apps/desktop/src/renderer/src/pr-review/marks.tsx")] }))],
    [2300, () => store.edit(id, (one) => ({ ...one, rows: [...one.rows, row("tool", `Write ${MOD_DIR}/mod.toml`)] }))],
    [3100, () => store.edit(id, (one) => ({ ...one, rows: [...one.rows, row("tool", `Write ${MOD_DIR}/tab.js`)] }))],
  ];
  const finish = () => {
    if (!turns.delete(id)) return;
    timers.forEach((one) => window.clearTimeout(one));
    timers.clear();
    store.edit(id, (one) => ({
      ...one,
      rows: [...one.rows, row("agent", "Review queue is saved and on.")],
      turn: { state: "idle" },
      lastTurnAt: now(),
    }));
    // A Job that fixes the tab changes the mod in place, so it stays the first Job's.
    const maker = mods.find((one) => one.name === MOD_NAME)?.made_by ?? id;
    const row_: ModSummary = { name: MOD_NAME, kind: "tab", glyph: "git-pull-request", enabled: true, valid: true, made_by: maker, changed_at: now(), description: said === "" ? "" : "Pull requests waiting on your review, across repositories" };
    mods = [...mods.filter((one) => one.name !== MOD_NAME), row_];
    handle?.publish({ mods: { mods } });
  };
  turns.set(id, finish);
  for (const [at, run] of steps) timers.add(window.setTimeout(run, at));
  timers.add(window.setTimeout(finish, 4000));
}

/** The Job writes the module again: the mod's files change, so its row's `changed_at` moves and Bridge loads the module again. */
function rewrite(step: number): void {
  revision = step;
  mods = mods.map((one) => (one.name === MOD_NAME ? { ...one, changed_at: new Date().toISOString() } : one));
  handle?.publish({ mods: { mods } });
}

/** A message to the Mod Job that built the tab: it reads the tab, then rewrites its view a step at a time, and the tab beside its card changes with each. Idempotent as the first build is. */
function changeTurn(store: SessionsStore, id: string): void {
  if (turns.has(id)) return;
  const now = () => new Date().toISOString();
  const tool = (text: string): SessionRow => ({ id: `mod-${(rowSeq += 1)}`, at: now(), kind: "tool", text });
  const add = (more: SessionRow) => store.edit(id, (one) => ({ ...one, rows: [...one.rows, more] }));
  rewrite(0);
  const steps: [number, () => void][] = [
    [600, () => add(tool(`Read ${MOD_DIR}/tab.js`))],
    [1400, () => (add(tool(`Write ${MOD_DIR}/tab.js: the oldest lead`)), rewrite(1))],
    [2800, () => (add(tool(`Write ${MOD_DIR}/tab.js: how long each has waited`)), rewrite(2))],
    [4200, () => (add(tool(`Write ${MOD_DIR}/tab.js: the heading`)), rewrite(3))],
  ];
  const finish = () => {
    if (!turns.delete(id)) return;
    changeTimers.forEach((one) => window.clearTimeout(one));
    changeTimers.clear();
    rewrite(3);
    store.edit(id, (one) => ({
      ...one,
      rows: [...one.rows, { id: `mod-${(rowSeq += 1)}`, at: now(), kind: "message", from: { kind: "agent" }, text: "Review queue lists the oldest first, with how long each has waited." } satisfies SessionRow],
      turn: { state: "idle" },
      lastTurnAt: now(),
    }));
  };
  turns.set(id, finish);
  for (const [at, run] of steps) changeTimers.add(window.setTimeout(run, at));
  changeTimers.add(window.setTimeout(finish, 5400));
}

/** A message asking the tab to `import` something: the Job writes a static import, and Fleet's check refuses the mod. Fix starts a Mod Job whose build makes it valid again. */
function breakTurn(store: SessionsStore, id: string): void {
  if (turns.has(id)) return;
  const now = () => new Date().toISOString();
  const finish = () => {
    if (!turns.delete(id)) return;
    changeTimers.forEach((one) => window.clearTimeout(one));
    changeTimers.clear();
    store.edit(id, (one) => ({
      ...one,
      rows: [...one.rows, { id: `mod-${(rowSeq += 1)}`, at: now(), kind: "tool", text: `Write ${MOD_DIR}/tab.js: import the forge client` } satisfies SessionRow],
      turn: { state: "idle" },
      lastTurnAt: now(),
    }));
    mods = mods.map((one) => (one.name === MOD_NAME ? { ...one, valid: false, reason: "tab.js line 1: an `import` cannot be resolved, the tab is handed `host` instead", changed_at: now() } : one));
    handle?.publish({ mods: { mods } });
  };
  turns.set(id, finish);
  changeTimers.add(window.setTimeout(finish, 400));
}

export const sJobLayouts: Scenario = {
  ...sJobStory,
  name: "job-layouts",
  says: "A Job's area in panes: split, tabs of any kind, saved layouts, and a page the Job opens by itself",
  behaves: (fleet) => {
    resetCheckout();
    for (const id of JOBS) seedJobBrowser(id, { visited: [...SHOP], page: shopPage });
    handle = fleet;
    mods = [];
    // Layouts are kept across restarts for real, so the mock starts from none a person made and offers two of its own.
    forgetPanes();
    seedLayouts({
      "QA split": build({ dir: "row", ratio: 0.55, a: { dir: "column", ratio: 0.6, a: ["thread"], b: ["terminal"] }, b: ["browser"] }, undefined, { browser: "localhost:5173", terminal: "zsh" }),
      Review: build({ dir: "row", ratio: 0.5, a: ["thread"], b: ["changes"] }),
    });
    // The fifth Job opens with a Pull queue tab in its own pane, built by a Mod Job that is held on a question.
    // A Custom tab is known by its id alone, across Jobs, so this one is minted far from the ids other Jobs mint.
    look(HOLDS, (arrangement) => arrive({ ...arrangement, next: 900 }, "custom", "Pull queue"));
    askTab = panesOf(peek(HOLDS).root).flatMap((one) => one.tabs).find((one) => one.kind === "custom")!.id;
    seedBuilt(askTab, ASK_MOD);
    mods = [{ name: ASK_MOD, kind: "tab", glyph: "git-pull-request", enabled: true, valid: true, made_by: ASK_JOB, changed_at: new Date().toISOString(), description: "Pull requests waiting on your review, across repositories" }];
    handle.publish({ mods: { mods } });
    return {
      ...sJobStory.behaves?.(fleet),
      ...shellApi(),
      // What Fleet does for a Custom tab's description: starts a Job marked `mod`, tells it the words, and lets it build.
      startModSession: async (start) => {
        const store = modStore;
        const id = store === undefined ? undefined : await store.start();
        if (store === undefined || id === undefined) return { ok: false, outcome: unanswered("/mods/session") };
        const title = start.title ?? "Mod";
        const said = start.said ?? "";
        store.edit(id, (one) => ({
          ...one,
          title,
          mod: true as const,
          ...(start.from_job === undefined || start.from_tab === undefined ? {} : { modFor: { job: start.from_job, tab: start.from_tab } }),
          rows: [...one.rows, { id: `mod-${(rowSeq += 1)}`, at: new Date().toISOString(), kind: "message", from: { kind: "you" }, text: said }],
          turn: { state: "working" },
        } satisfies Session));
        modJobs.add(id);
        modTurn(store, id, said);
        return { ok: true, value: { session_id: id, title } };
      },
      setModEnabled: async (name, enabled) => {
        mods = mods.map((one) => (one.name === name ? { ...one, enabled } : one));
        handle?.publish({ mods: { mods } });
        return { ok: true };
      },
      validateMod: async (name): Promise<ModChecked | null> =>
        name === MOD_NAME || name === ASK_MOD ? { name, valid: true, problems: [], tab: { glyph: "git-pull-request", code: reviewQueueModule(revision) } } : null,
      getReviewLists: async () => {
        const value: PullReviewLists = { me: "you", fetched_at: new Date().toISOString(), requested: WAITING, team: [], assigned: [], mine: [] };
        return { ok: true, value };
      },
    };
  },
  // The bay the first Job holds, so its Bays and System tabs have something to draw.
  held: {
    ...sJobStory.held,
    slots: [
      ...(sJobStory.held?.slots ?? []),
      {
        manifest_id: repository().manifest!.id,
        slot: 3,
        path: "/Users/user/armada/.armada/slots/slot-3",
        base: "main",
        warm: true,
        behind: 0,
        held: { state: "hosted_session", session_id: CHANGES, session_title: "Retry the slot lease" },
        branch: "armada/retry-lease",
      } satisfies WorktreeSlot,
    ],
  },
  draft: {
    ...sJobStory.draft,
    // The Changes tab reads each pull request's patches from the forge: these are what it answers for the Jobs' pull requests.
    sharedPanel: () => {
      const base = sharedPanelReads({ lines: [] });
      const patches: Record<number, string> = { 2390: MOCK_PATCH, 2391: MOCK_PATCH_TOOLTIP, 2366: MOCK_PATCH_LOG };
      return { ...base, pullRequestPatches: async (item) => (patches[item.number] === undefined ? (base.pullRequestPatches?.(item) ?? { ok: false, said: "The forge could not find this pull request." }) : patchesOf(item.manifestId ?? "", item.number, patches[item.number]!)) };
    },
    sessions: (board, more) => {
      seedSystem({
        session_id: CHANGES,
        processes: [
          { pid: 48211, command: "vite --port 5173", port: 5173, state: "running" },
          { pid: 47802, command: "vitest watch", state: "stopped" },
        ],
        cpu_percent: 38,
        memory_bytes: 1.8 * GB,
        memory_of_bytes: 8 * GB,
        disk_bytes: 6.4 * GB,
        disk_of_bytes: 20 * GB,
        worktrees: [],
        logs: [{ name: "session.log", path: `/mock/keepers/${CHANGES}.log`, bytes: 96 * KB }],
      });
      const store = sJobStory.draft!.sessions!(board, more);
      modStore = store;
      askJob = askJobOf(askTab);
      modJobs.add(ASK_JOB);
      // The store as the held Mod Job sees it: its edits land on its own Session, which the store does not hold.
      const asking: SessionsStore = { ...store, edit: (id, change) => (id === ASK_JOB ? editAsk(change) : store.edit(id, change)) };
      // The first Job has changes from three places, so a Changes tab has a choice: its branch, two pull requests of its own, and the Job it started, which has a pull request.
      store.edit(CHANGES, (one) => ({
        ...one,
        // The Workflow tab draws these lanes, as a Job that holds lifecycles has them.
        lanes: throughWire(CHANGES, jobAt(2)),
        // The context window the Job's head draws as its tank, with what fills it.
        context: { tokens: 186_000, window: 400_000 },
        contextBreakdown: {
          categories: [
            { name: "System prompt", tokens: 3_100, kind: "used" },
            { name: "System tools", tokens: 12_400, kind: "used" },
            { name: "Memory files", tokens: 4_600, kind: "used" },
            { name: "Messages", tokens: 165_900, kind: "used" },
            { name: "Free space", tokens: 134_000, kind: "free" },
            { name: "Autocompact buffer", tokens: 80_000, kind: "buffer" },
          ],
          memoryFiles: [{ path: "NOTES.md", tokens: 2_100 }],
          mcpTools: [],
          totalTokens: 186_000,
          maxTokens: 400_000,
          autocompactThreshold: 320_000,
        },
        attachments: [
          ...one.attachments,
          { kind: "branch", name: "armada/retry-lease", slot: 3 },
          { kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: "armada/retry-lease", address: "https://git.example/armada/pull/2390", checks: { state: "passed" }, state: "open", auto: false },
          { kind: "pull_request", number: 2391, title: "Name the attempt in the bay tooltip", branch: "armada/retry-lease-tooltip", address: "https://git.example/armada/pull/2391", checks: { state: "pending" }, state: "draft", auto: false },
          { kind: "started_to", id: SUB },
          ...LINKED.map((one): SessionAttachment => ({ kind: "started_to", id: one.id })),
          { kind: "artifact", form: "doc", id: "docs/design/lease-retry.md", title: "How the lease retries" },
          { kind: "artifact", form: "page", id: "http://localhost:5173/checkout", title: "The checkout page" },
          { kind: "artifact", form: "file", id: "crates/fleet/src/lease.rs", title: "lease.rs" },
        ],
      }));
      // The Jobs it started stand beside the story's own; the merged list keeps one identity until the store changes, as a store's does.
      let base: readonly Session[] | undefined;
      let held: Session | undefined;
      let merged: readonly Session[] = [];
      return {
        ...store,
        get: () => {
          const now = store.get();
          if (now !== base || askJob !== held) {
            base = now;
            held = askJob;
            merged = [...now, ...LINKED, ...(askJob === undefined ? [] : [askJob])];
          }
          return merged;
        },
        subscribe: (on: () => void) => {
          askListeners.add(on);
          const off = store.subscribe(on);
          return () => {
            askListeners.delete(on);
            off();
          };
        },
        // Allow once settles the ask, and the Job goes on with its change.
        answer: (id: string, ...more_: Parameters<typeof store.answer> extends [string, ...infer R] ? R : never) => {
          if (id !== ASK_JOB) return store.answer(id, ...more_);
          editAsk(({ asked, ...rest }) => (void asked, rest));
          modJobs.add(id);
          changeTurn(asking, id);
        },
        terminalRuns: true as const,
        // A message to a Mod Job is the person's row and its next turn, and not the canned reply the store plays for any other Job.
        send: (id: string, sent: Parameters<typeof store.send>[1]) => {
          if (!modJobs.has(id)) return store.send(id, sent);
          asking.edit(id, (one) => ({ ...one, rows: [...one.rows, { id: `mod-${(rowSeq += 1)}`, at: new Date().toISOString(), kind: "message", from: { kind: "you" }, text: sent.text } satisfies SessionRow], turn: { state: "working" } }));
          if (/\bimport\b/i.test(sent.text)) breakTurn(asking, id);
          else changeTurn(asking, id);
          return Promise.resolve(true);
        },
        later: () => {
          store.later();
          for (const finish of [...turns.values()]) finish();
          // The tab reads the host of the page the Job opened, on the dev server's other port; the mock's browser is the one it already has.
          jobOpens(OPENS, "browser", "localhost:5174");
        },
        dispose: () => {
          timers.forEach((one) => window.clearTimeout(one));
          timers.clear();
          changeTimers.forEach((one) => window.clearTimeout(one));
          changeTimers.clear();
          modJobs.clear();
          askJob = undefined;
          askListeners.clear();
          revision = 0;
          turns.clear();
          disposeShells();
          store.dispose();
        },
      };
    },
  },
};
