// A Job's thread long enough to judge its spacing by, for the walk `spacious-thread`: his words, folded calls, the agent's
// prose, a run with its last lines, the document waiting on his Approve and Needs you on the composer. The Job is the video
// script's (`video-script-job.ts`) with more turns. `?thread=spacious` draws it in the spacious take; without it, as today.

import type { SessionRow } from "@armada/screens/src/draft/sessions";

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

const base = videoScriptScenario(
  "spacious-thread",
  "A Job's thread with every kind of row, to judge the spacious take by",
  1,
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
