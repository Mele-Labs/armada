// One repository's battlefield with a Session building something on it, for the walk `battlefieldProgress`: its drone, two
// ordinary drones beside it, and a twelve-hour story the walk steps through (`battlefield-story.ts`). The Session's turn runs
// until its pull request opens, then ends; the forge shows the pull request open, queued and merged. The Session's `build` is the wire's
// own (`SessionBuild`), one for each moment, set on its record as Fleet would. #2301.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { HubMerged, HubPullRequest, MainStanding, MergeLines, TimelineEvent, TimelineSubject, WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { FEATURED, SUBAGENTS, SWARM, buildAt, subagentsBuild, swarmBuild } from "../battlefield-story";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore, type SessionsStore } from "../sessions/script";

const ARMADA = repository().manifest!.id;
const FORGE = "https://git.example/armada/pull/";
const BRANCH = "armada/dock-panel";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const PR = 2310;

const tool = (id: string, at: string, text: string) => ({ id, at, kind: "tool", text }) as const;
const ask = (id: string, at: string, text: string) => ({ id, at, kind: "message", from: { kind: "you" }, text }) as const;

const dock: Session = {
  id: FEATURED,
  title: "Rebuild the dock panel",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [ask("b1-u", "06:00:00", "Rebuild the dock panel, with the keys it has today."), tool("b1-t", "14:11:10", "Edit apps/desktop/src/renderer/src/cockpit/dock/keys.ts")],
  attachments: [{ kind: "branch", name: BRANCH, slot: 1 }],
  build: buildAt(0),
};

const clock: Session = {
  id: "bp2",
  title: "Pin the store clock",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(2),
  standing: "working",
  rows: [ask("b2-u", "13:20:00", "Pin the clock in the store tests."), tool("b2-t", "14:09:10", "Edit crates/store/tests/clock.rs")],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 2 }],
};

/** The Session with subagents: no ring until the walk's last moment, when its list and two subagents' pieces are drawn. */
const crew: Session = {
  id: SUBAGENTS,
  title: "Draw subagents in the ring",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [ask("b3-u", "14:00:00", "Draw each running subagent in the ring."), tool("b3-t", "14:10:10", "Edit apps/desktop/src/renderer/src/cockpit/deck/ring.tsx")],
  attachments: [{ kind: "branch", name: "armada/ring-workers", slot: 4 }],
};

/** The Session with ten subagents: no ring until the walk's last moment. */
const swarm: Session = {
  id: SWARM,
  title: "Split the cockpit panels",
  manifestId: ARMADA,
  turn: { state: "working" },
  lastTurnAt: ago(1),
  standing: "working",
  rows: [ask("b4-u", "14:00:00", "Split the cockpit into panels, one subagent for each."), tool("b4-t", "14:11:10", "Edit apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx")],
  attachments: [{ kind: "branch", name: "armada/cockpit-panels", slot: 5 }],
};

/** The turn ends when the pull request opens: the Session waits on its Checks. */
const SETTLED: Partial<Session> = { turn: { state: "idle" }, standing: "idle", lastTurnAt: new Date().toISOString() };

function sessions(): SessionsStore {
  const store = sessionsStore([dock, clock, crew, swarm]);
  const own = new Set([dock.id, clock.id, crew.id, swarm.id]);
  let moment = 0;
  let from: readonly Session[] | undefined;
  let only: readonly Session[] = [];
  return {
    ...store,
    get: () => {
      const all = store.get();
      if (all !== from) {
        from = all;
        only = all.filter((one) => own.has(one.id));
      }
      return only;
    },
    stop: (id) => store.edit(id, (one) => ({ ...one, turn: { state: "idle" }, standing: "idle", lastTurnAt: new Date().toISOString() })),
    later() {
      moment += 1;
      if (moment === 9) store.edit(SUBAGENTS, (one) => ({ ...one, build: subagentsBuild() }));
      else if (moment === 10) store.edit(SWARM, (one) => ({ ...one, build: swarmBuild() }));
      else store.edit(FEATURED, (one) => ({ ...one, build: buildAt(moment), ...(moment === 5 ? SETTLED : {}) }));
    },
  };
}

const COMMIT = "9f3c1d2e4b5a69788796a5b4c3d2e1f001122334";
const GREEN: MainStanding = { state: "green", commit: COMMIT, read_at: new Date().toISOString() };

const open = (ci: string, more: Partial<HubPullRequest> = {}): HubPullRequest => ({ number: PR, title: dock.title!, branch: BRANCH, url: `${FORGE}${PR}`, ci, opened_at: ago(20), ...more });
const merged = (): HubMerged => ({ number: PR, title: dock.title!, branch: BRANCH, url: `${FORGE}${PR}`, merged_at: ago(1), main_run: { state: "passed" } });

/** The forge at a moment: nothing yet, the pull request open, queued, merged. */
function lines(moment: number): MergeLines {
  const pull_requests = moment < 5 ? [] : moment === 5 ? [open("running")] : moment === 6 ? [open("running", { queue: { state: "queued", position: 1 } })] : [];
  return { lines: [{ root: repository().root, line: [], off: [], landed: [], sent_back: [], hub: { main: GREEN, pull_requests, merged: moment >= 7 ? [merged()] : [] } }] };
}

const bay = (slot: number, held: WorktreeSlot["held"], rest: Partial<WorktreeSlot> = {}): WorktreeSlot => ({ manifest_id: ARMADA, slot, path: `/Users/user/armada/.armada/slots/slot-${slot}`, base: "main", warm: true, behind: 0, held, ...rest });

function pool(): WorktreesHeld {
  return {
    slots: [
      bay(1, { state: "hosted_session", session_id: dock.id, session_title: dock.title! }, { branch: BRANCH, since: ago(720) }),
      bay(2, { state: "hosted_session", session_id: clock.id, session_title: clock.title! }, { branch: "fix/pin-store-clock", since: ago(40) }),
      bay(4, { state: "hosted_session", session_id: crew.id, session_title: crew.title! }, { branch: "armada/ring-workers", since: ago(15) }),
      bay(5, { state: "hosted_session", session_id: swarm.id, session_title: swarm.title! }, { branch: "armada/cockpit-panels", since: ago(12) }),
    ],
  };
}

let count = 0;
type Who = { subject: TimelineSubject; title: string };
const sessionWho = (one: Session): Who => ({ subject: { kind: "session", id: one.id }, title: one.title ?? one.id });
function event(minutes: number, who: Who, kind: TimelineEvent["kind"], tone: TimelineEvent["tone"], says: string, rest: { detail?: string; items?: string[]; live?: boolean } = {}): TimelineEvent {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: ARMADA, ...rest };
}
const edited = (minutes: number, who: Who, items: string[]) => event(minutes, who, "session.edited", "run", `Edited ${items.length} files`, { detail: items[0]!, items, live: true });
const lease = (minutes: number, who: Who, slot: number) => event(minutes, who, "slot.leased", "idle", `Slot ${slot} leased`, { detail: `slot-${slot}` });

/** What places the drones: the directories each has edited, as the checkout spells them, and who holds each bay. The story's own record is the construction. */
function placed(): TimelineEvent[] {
  return [
    event(720, sessionWho(dock), "session.started", "run", "Started", { live: true }),
    // Scoped: a walk of the current dock is shown and two questions are asked before anything is planned.
    event(690, sessionWho(dock), "session.walked", "ask", "Walk shown", { detail: "The dock today" }),
    event(680, sessionWho(dock), "session.asked", "ask", "Asked: which panel owns the dock?"),
    event(670, sessionWho(dock), "session.asked", "ask", "Asked: keep the dock keys?"),
    edited(60, sessionWho(dock), ["apps/desktop/src/renderer/src/cockpit/dock/Page.tsx", "apps/desktop/src/renderer/src/cockpit/dock/keys.ts"]),
    event(40, sessionWho(clock), "session.started", "run", "Started", { live: true }),
    event(15, sessionWho(crew), "session.started", "run", "Started", { live: true }),
    edited(5, sessionWho(crew), ["apps/desktop/src/renderer/src/cockpit/deck/ring.tsx", "apps/desktop/src/renderer/src/cockpit/deck/build.ts"]),
    event(12, sessionWho(swarm), "session.started", "run", "Started", { live: true }),
    edited(4, sessionWho(swarm), ["apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx", "apps/desktop/src/renderer/src/cockpit/deck/Bays.tsx"]),
    edited(20, sessionWho(clock), ["crates/store/tests/clock.rs", "crates/store/tests/pool.rs"]),
    lease(720, sessionWho(dock), 1),
    lease(40, sessionWho(clock), 2),
    lease(15, sessionWho(crew), 4),
    lease(12, sessionWho(swarm), 5),
  ];
}

function build(): Scenario {
  const base = holding("battlefield-progress", "One repository's battlefield with a Session building a dock panel over twelve hours, as a ring");
  return {
    ...base,
    state: { ...base.state, repository: null, mergeLines: lines(0) },
    held: pool(),
    // Moments 1 to 8: the Session's turn ends and its pull request opens at the fifth, is queued at the sixth and merges at the seventh. The ninth and tenth draw other Sessions' subagents.
    later: Array.from({ length: 10 }, (_, i) => ({ mergeLines: lines(i + 1) })),
    draft: { sessions, events: placed(), areas: mockAreas },
  };
}

export const s253BattlefieldProgress: Scenario = build();
