// A Job's thread names another Job by its id, an issue and a pull request. Each is a link in the thread's
// own type: the Job opens in the Cockpit, the Job's own pull request opens its panel, and the issues open
// on the forge. Code stays plain, except a span that is exactly a Job id. The walk `threadLinks` plays it.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { withPullRequestPages } from "../session-pr-pages";
import { s200Sessions } from "./sessions";

const PULL = "https://github.com/mele-labs/armada/pull/";
const now = new Date().toISOString();
const quiet = { turn: { state: "idle" }, lastTurn: "14:40", lastTurnAt: now } as const;
const FENCE = "```";

const found: Session = {
  ...quiet,
  id: "9e2e41a1c07b",
  title: "Retry loop findings",
  attachments: [],
  rows: [{ id: "f1", at: "14:12:08", kind: "message", from: { kind: "agent" }, text: "The loop restarts its backoff on every reconnect. Traced and written up." }],
};

const follow: Session = {
  ...quiet,
  id: "4a9c0d13f7e2",
  title: "Store retry follow-up",
  attachments: [
    {
      kind: "pull_request",
      number: 2209,
      title: "Open pull requests in Armada",
      branch: "claude/pr-panel-redesign",
      address: `${PULL}2209`,
      checks: { state: "passed" },
      state: "open",
      auto: false,
      body: "Pull request links open the same way everywhere.",
    },
  ],
  rows: [
    { id: "t1", at: "14:36:02", kind: "message", from: { kind: "you" }, text: "Read what s-9e2e41a1 found, then carry on from there." },
    { id: "t2", at: "14:38:20", kind: "message", from: { kind: "agent" }, text: "s-9e2e41a1 traced the loop to #2214. The change is #2209, and the earlier report is mele-labs/armada#2212." },
    {
      id: "t3",
      at: "14:40:11",
      kind: "message",
      from: { kind: "agent" },
      text: `Checks on ${PULL}2209 pass. Another repository's report: https://github.com/mele-labs/storefront/issues/412.\n\nIn code: \`s-9e2e41a1 --resume\` and \`#2214\`.\n\n${FENCE}sh\n# see #2214 on s-9e2e41a1\nscripts/check --changed\n${FENCE}`,
    },
  ],
};

function line(): MergeLine {
  return {
    root: repository().root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: { pull_requests: [{ number: 2209, title: "Open pull requests in Armada", branch: "claude/pr-panel-redesign", url: `${PULL}2209` }] },
  };
}

export const s282ThreadLinks: Scenario = {
  ...s200Sessions,
  name: "thread-links",
  says: "A Job's thread names another Job, an issue and a pull request",
  state: { ...s200Sessions.state, mergeLines: { lines: [line()] } },
  draft: { sessions: (board) => withPullRequestPages(s200Sessions.draft!.sessions!(board, [found, follow])) },
};
