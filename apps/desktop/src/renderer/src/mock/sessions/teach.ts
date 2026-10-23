// The Job `session-teach` opens: an agent about to open a pull request for a Jira key, and a later message that proposes an idea
// in prose. The first holds the words the owner teaches a link from, the second the words he teaches a habit from.

import type { SessionRow } from "@armada/protocol";

import { held, hosted } from "../sessions-fleet";

export const TEACH_JOB = "01SESSIONAAAAAAAAAAAAAAAAA";
const AT = "2026-10-22T09:12:00.000Z";

/** The message the Jira key is taught from. */
export const KEY_MESSAGE = "I am going to create a PR for STORE-1234 and then I will monitor CI for pass/fail.";

/** A proposal in prose, which the owner would rather have as a sketch with a list under it. */
export const IDEA_MESSAGE =
  "One idea for the retry work is to show the policy as a state machine. Each failure moves the request to the next state, and the give-up rule is the last one, so a reader sees where a request ends without reading the table.";

export const teachSessions = () => [hosted(TEACH_JOB, { title: "Pin the clock in the store", attachments: [held("slot", "3"), held("branch", "armada/store-pin-clock")] })];

export const teachThreads = (): Record<string, SessionRow[]> => ({
  [TEACH_JOB]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Fix the flaky store test and get it reviewed." },
    { kind: "tool", id: "t1", at: AT, text: "Bash cargo test -p store store_open" },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: KEY_MESSAGE },
    { kind: "message", id: "m3", at: AT, from: { kind: "you" }, text: "Is there anything else open on that ticket?" },
    {
      kind: "message",
      id: "m4",
      at: AT,
      from: { kind: "agent" },
      text: "STORE-1250 touches the same file, so STORE-1234 lands first. The branch is `STORE-1234-pin-clock` and its name stays as it is.",
    },
    { kind: "message", id: "m5", at: AT, from: { kind: "agent" }, text: IDEA_MESSAGE },
  ],
});
