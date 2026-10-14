// Sessions on the other machine under this login, beside this machine's three. Each Fleet publishes a
// trimmed summary and reads the other's, so a row has a title, a repository, a state, since when and
// what it waits on, and nothing to open. One waits on a question, one runs, one is done, one is on a
// repository this machine does not have. Over `session-waiting-on-you`, whose three are this machine's.
// Time passing moves one to a question, then takes the whole machine quiet. The walk
// `other-machine-sessions` plays it.
//
// The two takes of its waiting items on the Cockpit's Your move, over this machine's two Job calls and
// the same machine with both questions already asked, are `other-machine-strip` and `other-machine-cards`,
// played by the walks `other-machine-sessions-strip` and `other-machine-sessions-cards`.

import type { OtherMachine } from "@armada/components";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";
import { s206SessionWaitingOnYou } from "./session-waiting-on-you";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const MACHINE: OtherMachine = {
  name: "MacBook Air",
  sessions: [
    { id: "r1", repository: "armada", title: "Cap the retry backoff", state: "waiting", since: minutesAgo(6), waiting: ["Cap it at 5 or 8 tries?"] },
    { id: "r2", repository: "armada", title: "Pin the CI toolchain", state: "working", since: minutesAgo(9) },
    { id: "r3", repository: "armada", title: "Trim the changelog", state: "idle", since: minutesAgo(41) },
    { id: "r4", repository: "harbor-sync", title: "Port the sync adapter", state: "working", since: minutesAgo(18) },
  ],
};

const asking = (was: OtherMachine): OtherMachine => ({
  ...was,
  sessions: was.sessions.map((one) =>
    one.id === "r2" ? { ...one, state: "waiting", since: minutesAgo(1), waiting: ["Allow the toolchain download?"] } : one,
  ),
});

const quiet = (was: OtherMachine): OtherMachine => ({ ...was, lastHeard: minutesAgo(12) });

export const s250OtherMachineSessions: Scenario = {
  ...s206SessionWaitingOnYou,
  name: "other-machine-sessions",
  says: "This machine's Sessions beside the other machine's, read-only, then that machine going quiet",
  draft: {
    ...s206SessionWaitingOnYou.draft,
    otherMachine: { machine: MACHINE, moments: [asking, quiet] },
  },
};

/** This machine's two Job calls on Your move, and the other machine's two questions beside them. */
function onCockpit(name: string, take: "strip" | "cards"): Scenario {
  return {
    ...s200Sessions,
    name,
    says: `Your move with the other machine's waiting items as ${take === "strip" ? "a strip of lines" : "cards"}, then that machine going quiet`,
    draft: {
      ...s200Sessions.draft,
      // This machine has a Job called Cap the retry backoff on the same screen, so the other's is named apart.
      otherMachine: {
        machine: { ...asking({ ...MACHINE, sessions: MACHINE.sessions.map((one) => (one.id === "r1" ? { ...one, title: "Bound the retry tries" } : one)) }), onCockpit: take },
        moments: [quiet],
      },
    },
  };
}

export const s251OtherMachineStrip: Scenario = onCockpit("other-machine-strip", "strip");
export const s252OtherMachineCards: Scenario = onCockpit("other-machine-cards", "cards");
