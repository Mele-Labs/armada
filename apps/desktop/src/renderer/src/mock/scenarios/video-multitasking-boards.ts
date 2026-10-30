// The hi-fi boards for the multitasking film: the real Bridge shell on a demo product, Relay, in the state of each shot. Relay is four
// repositories (web, api, app and docs) and one project, Relay 2.4, holds a task in each. The four tasks become four Jobs through the
// Dispatch sheet, one sheet for each repository, and nine `later` moments carry the film: the drones lift off, Flight 2
// asks, Flight 3 shows a screen, Flight 4 starts by itself, and four pull requests (#412 to #415) land on a green main.
//
// **The cast is fixed by order.** The first Job the sheet starts is Flight 1, the second Flight 2, and so on, whatever the sheet was
// given, so a replay always lands on the same four. Mock only: the walk `video-multitasking-boards` plays it, and `hud=1` on the
// page adds the mission clock.

import type { HubMerged, HubPullRequest, MainStanding, MergeLines, Projects, RepositorySummary, TimelineEvent, WaitingItem, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionChecks, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Area } from "../../cockpit/deck/areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { BayControl, SessionsStore } from "../sessions/script";
import { projectsLink } from "../slices/projects";

const rootOf = (repo: string) => `/Users/user/code/${repo}`;
const FORGE = "https://git.example/relay/pull/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const now = () => new Date().toISOString();

const served = (id: string): RepositorySummary => ({
  root: rootOf(id),
  records_root: `/records/${id}`,
  manifest: { ...repository().manifest!, id, repository: id, path: `${id}/armada.yml` },
});
const REPOSITORIES = ["relay-web", "relay-api", "relay-app", "relay-docs"].map(served);

/** The four Jobs, in the order the cards are dispatched. */
type Flight = { title: string; repo: string; branch: string; slot: number; pull: number; files: string[] };
const FLIGHTS: readonly Flight[] = [
  { title: "Fix the flaky checkout test", repo: "relay-web", branch: "fix/flaky-checkout-test", slot: 1, pull: 412, files: ["tests/checkout/checkout.spec.ts", "tests/checkout/helpers.ts"] },
  { title: "Upgrade the payments SDK", repo: "relay-api", branch: "chore/payments-sdk-v4", slot: 1, pull: 413, files: ["payments/sdk/client.ts", "payments/sdk/config.ts", "payments/webhooks/register.ts"] },
  { title: "Dark mode for Settings", repo: "relay-app", branch: "feat/settings-dark-mode", slot: 1, pull: 414, files: ["settings/ui/Theme.tsx", "settings/ui/tokens.css"] },
  { title: "Release notes for 2.4", repo: "relay-docs", branch: "docs/release-notes-2-4", slot: 1, pull: 415, files: ["releases/2.4/notes.md"] },
];

/** The ids the store gave the four Jobs as the sheet started them. Emptied with each window. */
const ids: string[] = [];

// ---- The project --------------------------------------------------------------------------------------------------------------

const task = (id: string, flight: Flight) => ({ kind: "task" as const, id, manifest_id: flight.repo, title: flight.title, notes: "", state: "open" as const });
const PROJECTS: Projects = {
  projects: [
    {
      id: "relay-2-4",
      name: "Relay 2.4",
      rows: [
        task("task-checkout", FLIGHTS[0]!),
        task("task-payments", FLIGHTS[1]!),
        task("task-dark", FLIGHTS[2]!),
        task("task-notes", FLIGHTS[3]!),
      ],
    },
  ],
  loose: [],
};

// ---- Each repository's directories, two deep: the slabs of its territory ---------------------------------------------------------

const AREAS: Readonly<Record<string, readonly Area[]>> = {
  [rootOf("relay-web")]: [
    { dir: "tests", kids: [["checkout", 34], ["cart", 21], ["search", 18]] },
    { dir: "src", kids: [["routes", 40], ["components", 52]] },
  ],
  [rootOf("relay-api")]: [
    { dir: "payments", kids: [["sdk", 40], ["webhooks", 26]] },
    { dir: "src", kids: [["routes", 36], ["models", 28]] },
  ],
  [rootOf("relay-app")]: [
    { dir: "settings", kids: [["ui", 38], ["store", 15]] },
    { dir: "src", kids: [["screens", 60], ["nav", 18]] },
  ],
  [rootOf("relay-docs")]: [
    { dir: "releases", kids: [["2.4", 6], ["2.3", 9], ["2.2", 8]] },
    { dir: "guides", kids: [["setup", 14], ["billing", 11]] },
  ],
};

// ---- Bays ---------------------------------------------------------------------------------------------------------------------

const bay = (repo: string, slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
  manifest_id: repo,
  slot,
  path: `${rootOf(repo)}/.armada/slots/slot-${slot}`,
  base: "main",
  warm: true,
  behind: 0,
  held,
  ...rest,
});
const freeBays = (): WorktreesHeld => ({ slots: FLIGHTS.flatMap((one) => [1, 2].map((slot) => bay(one.repo, slot, { state: "free" }))) });

// ---- Rows a Job writes --------------------------------------------------------------------------------------------------------

let written = 0;
const say = (text: string): SessionRow => ({ id: `video-${(written += 1)}`, at: now(), kind: "message", from: { kind: "agent" }, text });
const tool = (text: string): SessionRow => ({ id: `video-${(written += 1)}`, at: now(), kind: "tool", text });

const pullOf = (flight: Flight, state: "open" | "merged", checks: SessionChecks, queued: boolean): SessionAttachment => ({
  kind: "pull_request",
  number: flight.pull,
  title: flight.title,
  branch: flight.branch,
  address: `${FORGE}${flight.pull}`,
  checks,
  state,
  auto: false,
  ...(queued ? { queued: true as const } : {}),
});

/** The Job with its pull request replaced by this one, its branch kept. */
const withPull = (one: Session, pull: SessionAttachment): Session => ({ ...one, attachments: [...one.attachments.filter((it) => it.kind !== "pull_request"), pull] });

/** The Job without a key it no longer carries. */
function without(one: Session, key: "asked" | "waits"): Session {
  const { [key]: _gone, ...rest } = one;
  void _gone;
  return rest;
}

/** Flight 2's question, in the wire's own shape. */
const QUESTION = "v4 drops the v1 webhooks. Migrate now, or pin v3?";
const ASKED: NonNullable<Session["asked"]> = {
  command: QUESTION,
  call: "flight-2-ask",
  offers: ["allow_once", "refuse"],
  questions: [
    {
      question: QUESTION,
      header: "Webhooks",
      multi_select: false,
      options: [
        { label: "Migrate now", description: "Moves the webhooks to v4 in this change" },
        { label: "Pin v3", description: "Keeps the v1 webhooks until the next release" },
      ],
    },
  ],
};

// ---- The Sessions store, and the moments that move it -----------------------------------------------------------------------

function sessions(control: BayControl): SessionsStore {
  ids.length = 0;
  const store = sessionsStore([], [], new Map(REPOSITORIES.map((one) => [one.root, one.manifest!.id] as const)), []);
  // The store gives ids s7 to s16 a fixture script of their own (a window that opens, takes, a sketch): ten starts are spent here and
  // kept off the list, so the four Jobs the sheet starts land on ids with none.
  const spent = new Set<unknown>(Array.from({ length: 10 }, () => store.start()));
  let bays = freeBays();
  const setBay = (repo: string, slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}) => {
    bays = { slots: (bays.slots ?? []).map((one) => (one.manifest_id === repo && one.slot === slot ? bay(repo, slot, held, rest) : one)) };
    control.pool?.set(bays);
  };
  const flight = (at: number, change: (one: Session) => Session) => {
    const id = ids[at];
    if (id !== undefined) store.edit(id, change);
  };
  const hold = (at: number) => {
    const id = ids[at];
    const one = FLIGHTS[at]!;
    if (id !== undefined) setBay(one.repo, one.slot, { state: "hosted_session", session_id: id, session_title: one.title }, { branch: one.branch, since: now() });
  };
  const land = (at: number) => {
    const one = FLIGHTS[at]!;
    flight(at, (it) => ({ ...withPull(it, pullOf(one, "merged", { state: "passed" }, false)), dead: "ended" as const, turn: { state: "idle" }, standing: "done", lastTurnAt: now() }));
    setBay(one.repo, one.slot, { state: "free" });
  };

  let moment = 0;
  const moments: Record<number, () => void> = {
    // The drones lift off: three Jobs at work in their directories.
    1: () => {
      for (const at of [0, 1, 2]) flight(at, (one) => ({ ...one, turn: { state: "working" }, lastTurnAt: now(), rows: [...one.rows, tool(`Edit ${FLIGHTS[at]!.files[0]}`)] }));
    },
    // Flight 2 asks.
    2: () =>
      flight(1, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "idle",
        lastTurnAt: now(),
        rows: [...one.rows, say("The v4 SDK drops the v1 webhooks. Migrating them touches the handler and its tests.")],
        asked: ASKED,
      })),
    // Flight 2 is back at work, and Flight 3 shows a screen.
    3: () => {
      flight(1, (one) => ({ ...without(one, "asked"), turn: { state: "working" }, standing: "working", lastTurnAt: now(), rows: [...one.rows, tool("Edit payments/webhooks/register.ts")] }));
      const page = new URL("relay-settings.html", window.location.href).toString();
      const item: WaitingItem = { id: `walk:${page}`, text: "Dark mode for Settings is ready to look at", since: now(), source: "walk", act: { kind: "walk", target: page } };
      flight(2, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "idle",
        lastTurnAt: now(),
        rows: [...one.rows, say("Dark mode is on the Settings screen.")],
        waitingFor: [item],
      }));
    },
    // Flights 1 and 2 push. Flight 3 is approved and carries on. Flight 4 starts by itself and takes bay 4.
    4: () => {
      flight(0, (one) => ({ ...withPull(one, pullOf(FLIGHTS[0]!, "open", { state: "passed" }, true)), rows: [...one.rows, tool("gh pr create")] }));
      flight(1, (one) => ({ ...withPull(one, pullOf(FLIGHTS[1]!, "open", { state: "pending" }, false)), turn: { state: "idle" }, standing: "idle", lastTurnAt: now(), rows: [...one.rows, tool("gh pr create")] }));
      flight(2, (one) => ({ ...one, turn: { state: "working" }, standing: "working", waitingFor: [], lastTurnAt: now() }));
      flight(3, (one) => ({
        ...without(one, "waits"),
        attachments: [{ kind: "branch", name: FLIGHTS[3]!.branch, slot: FLIGHTS[3]!.slot }],
        turn: { state: "working" },
        standing: "working",
        lastTurnAt: now(),
        rows: [...one.rows, say("Flight 2 ended. Starting from main."), tool("Edit docs/releases/2.4.md")],
      }));
      hold(3);
    },
    // Flights 2 and 3 are queued behind Flight 1.
    5: () => {
      flight(1, (one) => withPull(one, pullOf(FLIGHTS[1]!, "open", { state: "passed" }, true)));
      flight(2, (one) => ({ ...withPull(one, pullOf(FLIGHTS[2]!, "open", { state: "passed" }, true)), turn: { state: "idle" }, standing: "idle", lastTurnAt: now() }));
    },
    // Each landing frees its bay.
    6: () => {
      land(0);
      flight(3, (one) => ({ ...withPull(one, pullOf(FLIGHTS[3]!, "open", { state: "pending" }, false)), turn: { state: "idle" }, standing: "idle", lastTurnAt: now() }));
    },
    7: () => {
      land(1);
      flight(3, (one) => withPull(one, pullOf(FLIGHTS[3]!, "open", { state: "passed" }, true)));
    },
    8: () => land(2),
    9: () => land(3),
  };

  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  const spend: SessionsStore = {
    ...store,
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => !spent.has(one.id));
      }
      return only;
    },
    // The sheet starts the cast in order. Fleet holds the written tasks a start carries, so the cards show the Job from that moment.
    start: (tag, root, request, tuning, after, held, projectRows) => {
      const id = store.start(tag, root, request, tuning, after, held);
      const one = FLIGHTS[ids.length];
      if (typeof id !== "string" || one === undefined) return id;
      ids.push(id);
      const branch: SessionAttachment = { kind: "branch", name: one.branch, slot: one.slot };
      store.edit(id, (it) => ({ ...it, title: one.title, manifestId: one.repo, ...(after === undefined ? { attachments: [branch] } : {}) }));
      if (after === undefined) hold(ids.length - 1);
      if (projectRows !== undefined) projectsLink.hold?.(id, one.title, projectRows);
      return id;
    },
    later() {
      moment += 1;
      moments[moment]?.();
    },
  };
  projectsLink.jobs = spend;
  return spend;
}

// ---- What Fleet did, for the battlefield --------------------------------------------------------------------------------------

let count = 0;
type Who = { subject: TimelineEvent["subject"]; title: string; repo: string };
const jobWho = (at: number): Who | undefined => (ids[at] === undefined ? undefined : { subject: { kind: "session", id: ids[at]! }, title: FLIGHTS[at]!.title, repo: FLIGHTS[at]!.repo });
const pullWho = (at: number): Who => ({ subject: { kind: "pull_request", number: FLIGHTS[at]!.pull, url: `${FORGE}${FLIGHTS[at]!.pull}` }, title: FLIGHTS[at]!.title, repo: FLIGHTS[at]!.repo });

function event(minutes: number, who: Who | undefined, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean; over?: boolean } = {}): TimelineEvent[] {
  if (who === undefined) return [];
  count += 1;
  return [{ id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: who.repo, ...rest }];
}
const started = (at: number) => [
  ...event(0.8, jobWho(at), "slot.leased", "idle", `Slot ${FLIGHTS[at]!.slot} leased`, { detail: `slot-${FLIGHTS[at]!.slot}` }),
  ...event(0.8, jobWho(at), "session.started", "run", "Started", { live: true }),
];
const edited = (at: number, minutes: number) => event(minutes, jobWho(at), "session.edited", "run", `Edited ${FLIGHTS[at]!.files.length} files`, { detail: FLIGHTS[at]!.files[0]!, items: FLIGHTS[at]!.files, live: true });
const opened = (at: number) => [
  ...event(0.3, pullWho(at), "pr.opened", "idle", "Opened", { detail: FLIGHTS[at]!.branch }),
  ...event(0.2, pullWho(at), "pr.checks", "ok", "Checks passed"),
];
const over = (at: number) => [
  ...event(0.1, jobWho(at), "session.ended", "ok", "Done", { over: true }),
  ...event(0.1, jobWho(at), "slot.released", "idle", `Slot ${FLIGHTS[at]!.slot} given back`, { detail: `slot-${FLIGHTS[at]!.slot}` }),
];

/** One batch for each moment after the first, in step with `later`. */
const EVENTS_LATER: NonNullable<NonNullable<Scenario["draft"]>["eventsLater"]> = [
  () => [0, 1, 2].flatMap((at) => [...started(at), ...edited(at, 0.4)]),
  () => [...event(0.1, jobWho(1), "session.turn", "ask", "Asking"), ...edited(0, 0.2), ...edited(2, 0.2)],
  () => [...event(0.1, jobWho(1), "session.turn", "run", "Editing the webhooks", { live: true }), ...event(0.1, jobWho(2), "session.turn", "ask", "Walk ready")],
  () => [...opened(0), ...opened(1), ...started(3), ...edited(3, 0.2), ...edited(2, 0.2)],
  () => opened(2),
  () => [...over(0), ...opened(3)],
  () => over(1),
  () => over(2),
  () => over(3),
];

// ---- The forge: four pull requests through the merge queue -------------------------------------------------------------------

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const GREEN: MainStanding = { state: "green", commit: COMMIT, read_at: new Date().toISOString() };

/** Where each pull request stands at each moment: not yet open, running its Checks, queued in its repository's line, or merged. */
type PullAt = "none" | "running" | "queued" | "merged";
const PULLS_AT: readonly (readonly [PullAt, PullAt, PullAt, PullAt])[] = [
  ["none", "none", "none", "none"],
  ["none", "none", "none", "none"],
  ["none", "none", "none", "none"],
  ["none", "none", "none", "none"],
  ["queued", "running", "none", "none"],
  ["queued", "queued", "queued", "none"],
  ["merged", "queued", "queued", "running"],
  ["merged", "merged", "queued", "queued"],
  ["merged", "merged", "merged", "queued"],
  ["merged", "merged", "merged", "merged"],
];

/** Each repository's own line and main light, with the one pull request of the film in it and an older one landed before. */
function lines(at: number): MergeLines {
  const stood = PULLS_AT[Math.min(at, PULLS_AT.length - 1)]!;
  return {
    lines: FLIGHTS.map((one, i) => {
      const here = stood[i]!;
      const open: HubPullRequest[] =
        here === "none" || here === "merged"
          ? []
          : [
              {
                number: one.pull,
                title: one.title,
                branch: one.branch,
                url: `${FORGE}${one.pull}`,
                ci: here === "running" ? "running" : "passed",
                opened_at: ago(2),
                updated_at: ago(0.5),
                ...(here === "queued" ? { queue: { state: "queued", position: 1 } } : {}),
              },
            ];
      const landed: HubMerged[] = here === "merged" ? [{ number: one.pull, title: one.title, branch: one.branch, url: `${FORGE}${one.pull}`, merged_at: ago(0.3), main_run: { state: "passed" } }] : [];
      const older: HubMerged = { number: 408 + (3 - i), title: "Bump the lint config", branch: "chore/bump-lint-config", url: `${FORGE}${408 + (3 - i)}`, merged_at: ago(240), main_run: { state: "passed" } };
      return { root: rootOf(one.repo), line: [], off: [], landed: [], sent_back: [], hub: { main: GREEN, pull_requests: open, merged: [...landed, older] } };
    }),
  };
}

function build(): Scenario {
  const base = holding(
    "video-multitasking-boards",
    "Relay 2.4 across four repositories, one task each, dispatched as four Jobs that fly, call, wait on each other and land through their merge lines",
    REPOSITORIES,
  );
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: lines(0) },
    held: freeBays(),
    later: [{}, {}, {}, { mergeLines: lines(4) }, { mergeLines: lines(5) }, { mergeLines: lines(6) }, { mergeLines: lines(7) }, { mergeLines: lines(8) }, { mergeLines: lines(9) }],
    draft: {
      sessions: (control) => sessions(control),
      events: [],
      eventsLater: EVENTS_LATER,
      areas: (root) => AREAS[root] ?? [],
      projectsTab: PROJECTS,
    },
  };
}

export const videoMultitaskingBoards: Scenario = build();
