// Sessions on the other machine under this login, beside this machine's three. Each Fleet publishes a
// trimmed summary and reads the other's, so a row has a title, a repository, a state, since when and
// what it waits on, and nothing to open. One waits on a question, one runs, one is done, one is on a
// repository this machine does not have. Over `session-waiting-on-you`, whose three are this machine's.
// Time passing moves one to a question, then takes the whole machine quiet. The walk
// `other-machine-sessions` plays it.

import type { OtherMachine } from "@armada/components";

import type { Scenario } from "../moment";
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

export const s250OtherMachineSessions: Scenario = {
  ...s206SessionWaitingOnYou,
  name: "other-machine-sessions",
  says: "This machine's Sessions beside the other machine's, read-only, then that machine going quiet",
  draft: {
    ...s206SessionWaitingOnYou.draft,
    otherMachine: {
      machine: MACHINE,
      moments: [
        (was) => ({
          ...was,
          sessions: was.sessions.map((one) =>
            one.id === "r2" ? { ...one, state: "waiting", since: minutesAgo(1), waiting: ["Allow the toolchain download?"] } : one,
          ),
        }),
        (was) => ({ ...was, lastHeard: minutesAgo(12) }),
      ],
    },
  },
};
