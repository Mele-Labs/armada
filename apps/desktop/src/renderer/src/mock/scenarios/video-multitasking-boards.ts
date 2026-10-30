// The hi-fi boards for the multitasking film: the real Bridge shell on a demo product, Relay, in the state of each shot. Relay is four
// repositories (web, api, app and docs). Its project, Relay 2.4, holds one card, "Ship Relay 2.4", with a subtask in each repository.
// The card is dispatched through one sheet into a lead Job, which starts four Jobs of its own, linked under it, and eighteen `later`
// moments carry the film: the four start, the drones lift off, Flight 2 asks for a sign-in in a terminal, Flight 3 shows a walk,
// Flight 4 asks for a document to be read, Flight 1 drives its Job browser, four pull requests (#412 to #415) land on a green main
// and the lead ends.
//
// **The cast is fixed by order.** The first Job the sheet starts is the lead; the four it starts are Flights 1 to 4, in that order, so
// a replay always lands on the same five. Mock only: the walk `video-multitasking-boards` plays it, and `hud=1` on the page adds the
// mission clock.

import type { HubMerged, HubPullRequest, MainStanding, MergeLines, Projects, RepositorySummary, TimelineEvent, WaitingItem, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionChecks, SessionRow } from "@armada/screens/src/draft/sessions";

import { navigate, seedJobBrowser } from "../../job-browser/store";

import type { Area } from "../../cockpit/deck/areas";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { NOTES_PAGE, NOTES_PATH, NOTES_TITLE } from "../relay-notes-page";
import { sessionsStore } from "../sessions/script";
import type { BayControl, SessionsStore } from "../sessions/script";
import { projectsLink } from "../slices/projects";
import { STAGING_CARD, STAGING_CART, STAGING_DONE, STAGING_PAY, stagingPage } from "../staging-checkout-page";
import { tailOf } from "../terminal-run/ansi";
import { startPaymentsLogin } from "../terminal-run/payments-login";
import type { Deploy } from "../terminal-run/wrangler";

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

/** The ids the store gave the four Jobs the lead started, in the order it started them. Emptied with each window. */
const ids: string[] = [];
/** The id the store gave the lead, which the sheet started. */
let leadId: string | undefined;

const LEAD = "Ship Relay 2.4";
const REQUEST = `${LEAD}\n  - ${FLIGHTS.map((one) => one.title).join("\n  - ")}`;

// ---- The project --------------------------------------------------------------------------------------------------------------

/** The one card, which belongs to relay-web, and a subtask in each repository under it. */
const subtask = (id: string, flight: Flight) => ({ kind: "task" as const, id, manifest_id: flight.repo, title: flight.title, notes: "", state: "open" as const, parent: "task-ship" });
const PROJECTS: Projects = {
  projects: [
    {
      id: "relay-2-4",
      name: "Relay 2.4",
      rows: [
        { kind: "task", id: "task-ship", manifest_id: "relay-web", title: LEAD, notes: "", state: "open" },
        subtask("task-checkout", FLIGHTS[0]!),
        subtask("task-payments", FLIGHTS[1]!),
        subtask("task-dark", FLIGHTS[2]!),
        subtask("task-notes", FLIGHTS[3]!),
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

/** Flight 1's question: the staging checkout has been run in the Job browser, and the Job asks to go on. */
const QUESTION = "The checkout test passes on staging. Approve it?";
const ASKED: NonNullable<Session["asked"]> = {
  command: QUESTION,
  call: "flight-1-ask",
  offers: ["allow_once", "refuse"],
  questions: [
    {
      question: QUESTION,
      header: "Live check",
      multi_select: false,
      options: [
        { label: "Approve", description: "Opens the pull request" },
        { label: "Run it again", description: "Plays the checkout once more" },
      ],
    },
  ],
};

/** Flight 2's call: a command to run, which docks a terminal under the Job. */
const LOGIN = "payments login";
const loginItem = (): WaitingItem => ({ id: "payments-login", text: "Sign in to the payments sandbox", since: now(), source: "agent", act: { kind: "run", target: LOGIN } });

/** Flight 3's call: a walk of the new screen. */
const walkItem = (): WaitingItem => {
  const page = new URL("relay-settings.html", window.location.href).toString();
  return { id: `walk:${page}`, text: "Dark mode for Settings is ready to look at", since: now(), source: "walk", act: { kind: "walk", target: page } };
};

/** Flight 4's call: a document to read. The page itself is an artifact of the Job. */
const reviewItem = (): WaitingItem => ({ id: "review-notes", text: `${NOTES_TITLE} are ready to read`, since: now(), source: "agent" });
const NOTES: SessionAttachment = { kind: "artifact", form: "document", id: NOTES_PATH, title: NOTES_TITLE, revisions: [NOTES_PATH] };

const startedRow = (id: string, title: string): SessionRow => ({ id: `video-${(written += 1)}`, at: now(), kind: "started", session: { id, title }, fork: false });
const youSaid = (text: string): SessionRow => ({ id: `video-${(written += 1)}`, at: now(), kind: "message", from: { kind: "you" }, text });

// ---- The Sessions store, and the moments that move it -----------------------------------------------------------------------

function sessions(control: BayControl): SessionsStore {
  ids.length = 0;
  leadId = undefined;
  const store = sessionsStore([], [], new Map(REPOSITORIES.map((one) => [one.root, one.manifest!.id] as const)), []);
  // The store gives ids s7 to s16 a fixture script of their own (a window that opens, takes, a sketch): ten starts are spent here and
  // kept off the list, so the five Jobs the film starts land on ids with none.
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
  const working = (at: number, line: string) => flight(at, (one) => ({ ...one, turn: { state: "working" }, standing: "working", lastTurnAt: now(), rows: [...one.rows, tool(line)] }));
  const hold = (at: number) => {
    const id = ids[at];
    const one = FLIGHTS[at]!;
    if (id !== undefined) setBay(one.repo, one.slot, { state: "hosted_session", session_id: id, session_title: one.title }, { branch: one.branch, since: now() });
  };
  /** The lead starts a Job of its own in one repository, linked under it, and it takes a bay and goes to work. */
  const spawn = (at: number, quiet = false) => {
    const one = FLIGHTS[at]!;
    const lead = leadId;
    const id = store.start();
    if (lead === undefined || typeof id !== "string") return;
    ids[at] = id;
    store.edit(id, () => ({
      id,
      title: one.title,
      manifestId: one.repo,
      cwd: rootOf(one.repo),
      mode: "auto",
      turn: { state: "working" },
      standing: "working",
      lastTurnAt: now(),
      rows: [youSaid(`${one.title}. Part of ${LEAD}.`), tool(`Read ${one.files[0]}`)],
      attachments: [{ kind: "started_by", id: lead }, { kind: "slot", slot: one.slot }, { kind: "branch", name: one.branch, slot: one.slot }],
    }));
    store.edit(lead, (it) => ({ ...it, rows: [...it.rows, ...(quiet ? [] : [tool(`start_another_session ${one.title}`)]), startedRow(id, one.title)], attachments: [...it.attachments, { kind: "started_to", id }] }));
    hold(at);
  };
  const land = (at: number) => {
    const one = FLIGHTS[at]!;
    flight(at, (it) => ({ ...withPull(it, pullOf(one, "merged", { state: "passed" }, false)), dead: "ended" as const, turn: { state: "idle" }, standing: "done", lastTurnAt: now() }));
    setBay(one.repo, one.slot, { state: "free" });
  };

  /** The lead starts all four in one call, and the four rows arrive together. */
  const spawnAll = () => {
    if (leadId !== undefined) store.edit(leadId, (it) => ({ ...it, rows: [...it.rows, tool(`start_another_session ${FLIGHTS.map((one) => one.title).join(", ")}`)] }));
    FLIGHTS.forEach((_, at) => spawn(at, true));
  };

  let moment = 0;
  const moments: Record<number, () => void> = {
    // The lead starts its four Jobs, in one call.
    1: spawnAll,
    // The drones lift off.
    2: () => [0, 1, 2, 3].forEach((at) => working(at, `Edit ${FLIGHTS[at]!.files[0]}`)),
    // Flight 2 asks for a sign-in, in a command to run.
    3: () =>
      flight(1, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "idle",
        lastTurnAt: now(),
        rows: [...one.rows, say("The payments CLI needs a sign-in before it will register the webhooks. Run payments login and give it the code.")],
        waitingFor: [loginItem()],
      })),
    // Flight 3 shows a screen.
    4: () =>
      flight(2, (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", lastTurnAt: now(), rows: [...one.rows, say("Dark mode is on the Settings screen.")], waitingFor: [walkItem()] })),
    // Flight 3 is approved and carries on. Flight 4 has written its release notes and asks to have them read.
    5: () => {
      flight(2, (one) => ({ ...one, turn: { state: "working" }, standing: "working", waitingFor: [], lastTurnAt: now() }));
      flight(3, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "idle",
        lastTurnAt: now(),
        rows: [...one.rows, say("The release notes are written.")],
        attachments: [...one.attachments, NOTES],
        waitingFor: [reviewItem()],
      }));
    },
    // Flight 4 has its note and carries on. Flight 1 runs the staging checkout in its Job browser and asks to go on.
    6: () => {
      flight(3, (one) => ({ ...one, turn: { state: "working" }, standing: "working", waitingFor: [], lastTurnAt: now(), rows: [...one.rows, say("Leading with dark mode."), tool(`Edit ${FLIGHTS[3]!.files[0]}`)] }));
      const id = ids[0];
      if (id === undefined) return;
      seedJobBrowser(id, { visited: [STAGING_CART], page: stagingPage });
      flight(0, (one) => ({
        ...one,
        turn: { state: "idle" },
        standing: "idle",
        lastTurnAt: now(),
        rows: [...one.rows, tool(`browser_open ${STAGING_CART}`), say("Staging is up. I am running the checkout in the Job browser.")],
        asked: ASKED,
      }));
    },
    // Flight 1 drives the page: card details, Pay, the order confirmation.
    7: () => {
      const id = ids[0];
      if (id === undefined) return;
      navigate(id, STAGING_CARD);
      flight(0, (one) => ({ ...one, rows: [...one.rows, tool("browser_click Check out"), tool("browser_type 4242 4242 4242 4242 into Card number")] }));
    },
    8: () => {
      const id = ids[0];
      if (id === undefined) return;
      navigate(id, STAGING_PAY);
      flight(0, (one) => ({ ...one, rows: [...one.rows, tool("browser_click Pay $84.00")] }));
    },
    9: () => {
      const id = ids[0];
      if (id === undefined) return;
      navigate(id, STAGING_DONE);
      flight(0, (one) => ({ ...one, rows: [...one.rows, tool("browser_snapshot"), say("The order is confirmed. The checkout test passes.")] }));
    },
    // The four push. Flight 1 runs its Checks, and the other three are queued.
    10: () => {
      flight(0, (one) => ({ ...withPull(one, pullOf(FLIGHTS[0]!, "open", { state: "pending" }, false)), turn: { state: "idle" }, standing: "idle", lastTurnAt: now(), rows: [...one.rows, tool("gh pr create")] }));
      for (const at of [1, 2, 3]) {
        flight(at, (one) => ({ ...withPull(one, pullOf(FLIGHTS[at]!, "open", { state: "passed" }, true)), turn: { state: "idle" }, standing: "idle", lastTurnAt: now(), rows: [...one.rows, tool("gh pr create")] }));
      }
    },
    // Each landing frees its bay: relay-api, relay-app, relay-docs, then relay-web.
    11: () => {
      land(1);
      flight(0, (one) => withPull(one, pullOf(FLIGHTS[0]!, "open", { state: "passed" }, true)));
    },
    12: () => land(2),
    13: () => land(3),
    14: () => land(0),
    // The lead ends, and the tasks it held ask whether they are done.
    15: () => {
      const id = leadId;
      if (id === undefined) return;
      store.edit(id, (one) => ({
        ...one,
        dead: "ended" as const,
        turn: { state: "idle" },
        standing: "done",
        lastTurnAt: now(),
        rows: [...one.rows, say("All four pull requests are merged. Relay 2.4 is shipped.")],
      }));
      projectsLink.ask?.(id);
    },
  };

  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  const running = new Set<Deploy>();
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
    // The sheet starts the lead. Fleet holds the written tasks a start carries, so the cards show the Job from that moment.
    start: (tag, root, request, tuning, after, held, projectRows) => {
      const id = store.start(tag, root, request, tuning, after, held);
      if (typeof id !== "string" || leadId !== undefined) return id;
      leadId = id;
      store.edit(id, (it) => ({
        ...it,
        title: LEAD,
        manifestId: "relay-web",
        turn: { state: "working" },
        standing: "working",
        lastTurnAt: now(),
        rows: [youSaid(request ?? REQUEST), say("Four subtasks, one in each repository. I start a Job for each."), tool("list_repositories")],
      }));
      if (projectRows !== undefined) projectsLink.hold?.(id, LEAD, projectRows);
      return id;
    },
    // Flight 1's answer is a plain go ahead, and the Job carries on.
    answer: (id, answer, answers) => {
      if (id !== ids[0]) return store.answer?.(id, answer, answers);
      store.edit(id, (one) => ({
        ...without(one, "asked"),
        turn: { state: "working" },
        standing: "working",
        lastTurnAt: now(),
        rows: [...one.rows, youSaid((answers ?? []).map((it) => `**${it.question}**\n${it.chosen.join(", ")}`).join("\n\n")), say("Approved. Opening the pull request.")],
      }));
    },
    // The page Flight 4 wrote.
    readArtifact: async (id, path) => (path === NOTES_PATH ? { ok: true as const, bytes: new TextEncoder().encode(NOTES_PAGE), type: "text/html" } : store.readArtifact!(id, path)),
    // Flight 2's command runs in a terminal that asks for the sign-in code.
    terminalRuns: true as const,
    run: (id, command, on) => {
      if (command !== LOGIN) return store.run!(id, command, (event) => on(event.kind === "out" ? { kind: "out", text: event.text.replaceAll("\n", "\r\n") } : event));
      let printed = "";
      const program = startPaymentsLogin(
        command,
        (text) => {
          printed += text;
          on({ kind: "out", text });
        },
        (code) => {
          running.delete(program);
          if (code === 0) {
            store.edit(id, (one) => ({
              ...one,
              turn: { state: "working" },
              standing: "working",
              lastTurnAt: now(),
              waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== "payments-login"),
              rows: [...one.rows, { id: `video-${(written += 1)}`, at: now(), kind: "ran", command, code, tail: tailOf(printed, 20) }, say("Signed in. Registering the webhooks."), tool(`Edit ${FLIGHTS[1]!.files[2]}`)],
            }));
          }
          on(code === null ? { kind: "end", code, stopped: true } : { kind: "end", code });
        },
      );
      running.add(program);
      return { stop: program.stop, write: program.write };
    },
    later() {
      moment += 1;
      moments[moment]?.();
    },
    dispose: () => {
      running.forEach((one) => one.stop());
      store.dispose();
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

/** One batch for each moment, in step with `later`: moment n is `EVENTS_LATER[n - 1]`. */
const EVENTS_LATER: NonNullable<NonNullable<Scenario["draft"]>["eventsLater"]> = [
  () => [0, 1, 2, 3].flatMap((at) => started(at)),
  () => [0, 1, 2, 3].flatMap((at) => edited(at, 0.4)),
  () => event(0.1, jobWho(1), "session.turn", "ask", "Asking for a sign-in"),
  () => event(0.1, jobWho(2), "session.turn", "ask", "Walk ready"),
  () => [...event(0.1, jobWho(2), "session.turn", "run", "Editing the screens", { live: true }), ...event(0.1, jobWho(3), "session.turn", "ask", "Review requested")],
  () => [...event(0.1, jobWho(3), "session.turn", "run", "Editing the notes", { live: true }), ...event(0.1, jobWho(0), "session.turn", "ask", "Live check")],
  () => [],
  () => [],
  () => [],
  () => [0, 1, 2, 3].flatMap(opened),
  () => over(1),
  () => over(2),
  () => over(3),
  () => over(0),
  () => [],
];

// ---- The forge: four pull requests through the merge queue -------------------------------------------------------------------

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const GREEN: MainStanding = { state: "green", commit: COMMIT, read_at: new Date().toISOString() };

/** Where each pull request stands at each moment: not yet open, running its Checks, queued in its repository's line, or merged. */
type PullAt = "none" | "running" | "queued" | "merged";
const PULLS_AT: readonly (readonly [PullAt, PullAt, PullAt, PullAt])[] = [
  ...Array.from({ length: 13 }, (): [PullAt, PullAt, PullAt, PullAt] => ["none", "none", "none", "none"]),
  ["running", "queued", "queued", "queued"],
  ["queued", "merged", "queued", "queued"],
  ["queued", "merged", "merged", "queued"],
  ["queued", "merged", "merged", "merged"],
  ["merged", "merged", "merged", "merged"],
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
    "Relay 2.4 across four repositories, one card dispatched as a lead Job that starts four, each calling once through a different surface and landing through its merge line",
    REPOSITORIES,
  );
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: lines(0) },
    held: freeBays(),
    later: Array.from({ length: 18 }, (_, at) => (at + 1 >= 13 ? { mergeLines: lines(at + 1) } : {})),
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
