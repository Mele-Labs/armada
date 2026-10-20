// A Session that waits on another and starts on its own. The Cockpit with two live Sessions, one of them (`Cockpit Map`) at
// work on a plan, and a brief typed in Dispatch that tags it with `@` to say a new Session waits on it. The waiting Session
// holds its words and no slot; the Session it waits on is told in its thread; its agent then names the pull request, which
// lands, so the waiting Session starts with the handoff as its first message. The Sessions are draft data
// (`packages/screens/src/draft/sessions.ts`), nothing here reaches Fleet. The walk `sessionAfterAnother` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import type { SessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

/** The Session the new one waits on: the walk picks it by this id. */
const MAP = "after-map";
const QUIET = "after-migrations";
const BRANCH = "armada/cockpit-map";
const PULL = 2388;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const map: Session = {
  id: MAP,
  title: "Cockpit Map",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  lastTurnAt: ago(2),
  standing: "working",
  rows: [
    you("a-1", "13:50:00", "Draw the Cockpit as a map on the Pixi Field."),
    { id: "a-2", at: "13:58:12", kind: "tool", text: "Edit apps/desktop/src/renderer/src/cockpit/field.ts" },
    agent("a-3", "14:00:05", "The Pixi Field draws the Sessions. The overlay is next."),
  ],
  attachments: [{ kind: "branch", name: BRANCH, slot: 1 }],
  work: {
    plan: {
      scope: ["apps/desktop"],
      approved: true,
      groups: [
        { id: "g1", agent: "bridge-engineer", title: "Draw the Pixi Field", paths: ["apps/desktop"], tasks: [{ id: "T1", text: "Place each Session on the Field", state: "done", seen: true }, { id: "T2", text: "Draw the roads between them", state: "done", seen: true }] },
        { id: "g2", agent: "bridge-engineer", title: "Add the overlay", paths: ["apps/desktop"], tasks: [{ id: "T3", text: "Expand the Field into the overlay", state: "working", seen: true }] },
      ],
    },
  },
};

const quiet: Session = {
  id: QUIET,
  title: "Name the store migrations",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(22),
  standing: "idle",
  rows: [agent("q-1", "13:41:10", "The migrations are named by their order.")],
  attachments: [{ kind: "branch", name: "fix/migration-names", slot: 2 }],
};

function sessions(): SessionsStore {
  const base = sessionsStore([map, quiet], [], new Map([[ROOT, OWNER]]));
  let rowId = 0;
  let moment = 0;
  const said = (text: string): SessionRow => ({ id: `after-r${(rowId += 1)}`, at: `14:${String(10 + rowId).padStart(2, "0")}:00`, kind: "message", from: { kind: "agent" }, text });
  const waiting = () => base.get().find((one) => one.waits?.on.id === MAP);

  // What time brings, once a walk lets it pass: the agent of the Session being waited on names its pull request on the waiting one, the
  // pull request lands, and the waiting Session starts with the handoff as its first message.
  const moments = [
    () => {
      const one = waiting();
      if (one === undefined) return;
      const address = `https://git.example/armada/pull/${PULL}`;
      base.edit(MAP, (it) => ({
        ...it,
        rows: [...it.rows, { id: "after-pr", at: "14:14:00", kind: "tool", text: "gh pr create --base main" }, said(`Opened #${PULL} on ${BRANCH}. It lands once its Checks pass.`)],
        attachments: [...it.attachments, { kind: "pull_request", number: PULL, title: "Cockpit Map", branch: BRANCH, address, checks: { state: "pending" }, state: "open", auto: false }],
      }));
      base.agentChanges(one.id, { when: "lands", pull: { number: PULL, branch: BRANCH } });
      base.edit(MAP, (it) => ({ ...it, attachments: it.attachments.map((one) => (one.kind === "pull_request" ? { ...one, state: "merged" as const } : one)) }));
      base.starts(one.id, `Pull request #${PULL} landed`);
    },
  ];
  base.later = () => {
    moments[moment]?.();
    moment += 1;
  };
  return base;
}

function build(): Scenario {
  const base = holding("session-after-another", "A Session dispatched to wait on another: the @ menu and the sentence over the request, its own row in the Work list, the thread of the Session it waits on, and its start");
  return {
    ...base,
    state: { ...base.state, repository: ROOT },
    draft: { sessions },
  };
}

export const sessionAfterAnother: Scenario = build();
