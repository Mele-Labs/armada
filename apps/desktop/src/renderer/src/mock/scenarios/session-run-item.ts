// A Session waiting on the person to run two commands it also wrote in the thread: either is run from
// the Needs you band or from the thread's Run, and a clean exit settles its item. Over the `sessions`
// scenario. The walk `session-run-item` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const STOP = "kill $(lsof -ti tcp:5788)";
const INSTALL = "pnpm install --frozen-lockfile";

const waiting: Session = {
  id: "s16",
  title: "Debug Bridge on Merge line",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "k1", at: "22:12:40", kind: "message", from: { kind: "you" }, text: "Clean up after the merge line fix." },
    {
      id: "k2",
      at: "22:20:43",
      kind: "message",
      from: { kind: "agent" },
      text: `The slot guard won't let me stop the mock server on port 5788:\n\n\`\`\`bash\n${STOP}\n\`\`\`\n\nThe preview also needs its packages again:\n\n\`\`\`bash\n${INSTALL}\n\`\`\``,
    },
  ],
  attachments: [{ kind: "slot", slot: 10 }, { kind: "branch", name: "fix/merge-line", slot: 10 }],
  waitingFor: [
    { id: "r1", text: "Stop the mock server on 5788", since: minutesAgo(2), source: "agent", act: { kind: "run", target: STOP } },
    { id: "r2", text: "Install the preview's packages", since: minutesAgo(2), source: "agent", act: { kind: "run", target: INSTALL } },
  ],
};

export const s211SessionRunItem: Scenario = {
  ...s200Sessions,
  name: "session-run-item",
  says: "A Session waiting on the person to run two commands it wrote in the thread",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
