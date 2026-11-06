// A Job's thread long enough to judge its spacing by, for the walk `spacious-thread`: his words, folded calls, the agent's
// prose, a run with its last lines, the document waiting on his Approve and Needs you on the composer. The Job is the video
// script's (`video-script-job.ts`) with more turns; beside it, a Job working, one whose agent reasoned (a row only the mock
// draws) and one done. `?thread=spacious` draws them in the spacious take; without it, as today.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";

const ROWS: readonly SessionRow[] = [
  {
    id: "sp1",
    at: "10:02:00",
    kind: "message",
    from: {
      kind: "you",
    },
    text: "Write the script for a 60 second demo video of Armada for the landing page.",
  },
  {
    id: "sp2",
    at: "10:02:06",
    kind: "tool",
    text: "Read docs/scope.md",
  },
  {
    id: "sp3",
    at: "10:02:12",
    kind: "tool",
    text: "Read docs/journeys/first-job.md",
  },
  {
    id: "sp4",
    at: "10:02:30",
    kind: "message",
    from: {
      kind: "agent",
    },
    text: "Four shots: the Board, a plan card, a walk and the merge queue, with a line of voice over each.\n\nThe Board opens on three Jobs already working, so the first second shows something moving. The plan card is the one from the store clock fix, since it is short enough to read on screen.",
  },
  {
    id: "sp5",
    at: "10:03:40",
    kind: "message",
    from: {
      kind: "you",
    },
    text: "Keep the merge queue shot under ten seconds.",
  },
  {
    id: "sp6",
    at: "10:04:02",
    kind: "ran",
    command: "pnpm exec vitest run src/landing/video.test.ts",
    code: 0,
    tail: [
      " ✓ src/landing/video.test.ts (3 tests) 41ms",
      " Test Files  1 passed (1)",
      "      Tests  3 passed (3)",
    ].join("\n"),
  },
  {
    id: "sp7",
    at: "10:04:20",
    kind: "message",
    from: {
      kind: "agent",
    },
    text: "The queue shot is 8 seconds. The page's timings check passes, and the script is ready for you to read.",
  },
  {
    id: "sp8",
    at: "10:04:31",
    kind: "tool",
    text: "Write .armada/documents/video-script.html",
  },
];

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A Job whose agent is working on his last message. */
export const spaciousThinkingJob: Session = {
  id: "s91",
  title: "Landing page copy",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(0),
  rows: [
    {
      id: "th1",
      at: "11:20:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Write the hero line for the landing page.",
    },
    {
      id: "th2",
      at: "11:20:40",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "Three lines to pick from:\n\n1. Agents that check their own work.\n2. Dispatch a Job, get back a merged change.\n3. Every Job verified before it lands.",
    },
    {
      id: "th3",
      at: "11:21:30",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Shorter, and say what it is in the first three words.",
    },
    {
      id: "th4",
      at: "11:21:34",
      kind: "reasoning",
      seconds: 4,
      live: true,
      text: "The first three words have to name the thing. Agents, Jobs or Armada could lead. Agents is the word people search for, so it goes first, and the line ends on what they get back.",
    },
  ],
  attachments: [],
};

/** A Job whose agent is partway through its calls. */
export const spaciousCallingJob: Session = {
  id: "s94",
  title: "Notes for 0.4",
  turn: {
    state: "working",
  },
  lastTurnAt: ago(0),
  rows: [
    {
      id: "rn1",
      at: "11:30:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Write the release notes for 0.4 from what merged since 0.3.",
    },
    {
      id: "rn2",
      at: "11:30:05",
      kind: "reasoning",
      seconds: 6,
      text: "What merged since the 0.3 tag is in the git log. The changelog has the last release's headings to follow.",
    },
    {
      id: "rn3",
      at: "11:30:11",
      kind: "tool",
      text: "Read CHANGELOG.md",
    },
    {
      id: "rn4",
      at: "11:30:14",
      kind: "tool",
      text: "Bash git log --oneline v0.3..main",
    },
    {
      id: "rn5",
      at: "11:30:20",
      kind: "tool",
      text: "Grep Breaking in docs/releases",
    },
  ],
  attachments: [],
};

/** A Job whose agent reasoned before it answered. */
export const spaciousReasonedJob: Session = {
  id: "s92",
  title: "Retry budget",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(4),
  rows: [
    {
      id: "rs1",
      at: "11:02:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Why do the push retries give up after one try on a slow network?",
    },
    {
      id: "rs2",
      at: "11:02:04",
      kind: "tool",
      text: "Read crates/fleet/src/push.rs",
    },
    {
      id: "rs3",
      at: "11:02:18",
      kind: "reasoning",
      seconds: 14,
      text: "The budget is 30 seconds in all, and each try waits on a 25 second timeout. A slow push uses the whole budget on its first try, so the loop ends before a second one starts. The fix is a budget per try, not one shared across tries.",
    },
    {
      id: "rs4",
      at: "11:02:20",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "One try eats the whole budget. Each push waits up to 25 seconds and the budget for all tries is 30, so a slow first try leaves no room for a second. I'd give each try its own 25 seconds and cap the tries at three.",
    },
  ],
  attachments: [],
};

/** A Job whose agent says it has nothing left to do. */
export const spaciousFinishedJob: Session = {
  id: "s93",
  title: "Quiet the slot logs",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(2),
  rows: [
    {
      id: "dn1",
      at: "10:40:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Drop the lease lines from the slot log unless a lease fails.",
    },
    {
      id: "dn2",
      at: "10:52:11",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "Merged. The slot log keeps a lease line only when the lease fails.",
    },
  ],
  attachments: [
    {
      kind: "pull_request",
      number: 1871,
      title: "Log a lease only when it fails",
      branch: "fix/quiet-slot-logs",
      address: "https://example.com/pull/1871",
      checks: {
        state: "passed",
      },
      state: "merged",
      auto: false,
    },
  ],
  done: {
    since: ago(2),
    summary: "The slot log keeps a lease line only when a lease fails",
  },
};

const base = videoScriptScenario(
  "spacious-thread",
  "A Job's thread with every kind of row, to judge the spacious take by",
  1,
  [
    spaciousThinkingJob,
    spaciousCallingJob,
    spaciousReasonedJob,
    spaciousFinishedJob,
  ],
);

export const spaciousThread: Scenario = {
  ...base,
  draft: {
    ...base.draft,
    sessions: (board, more) => {
      const store = base.draft!.sessions!(board, more);
      store.edit("s58", (one) => ({
        ...one,
        rows: ROWS,
      }));
      return store;
    },
  },
};
