// The Job's story with every zone full, to see whether Completed, Now and Next balance: the real Job of `job-real-issues-tab` (ten marks in progress, in two groups), its three
// finished groups each carrying an issue and a pull request, a dozen more things it made (documents, sketches, issues, pull requests), and Next with four groups. Mock only;
// the `jobStoryFull` walk and `?scenario=job-story-full&take=` open it.

import type { Session, SessionAttachment, WorkGroup, WorkTask } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { arrow, box, scene } from "../scenes";
import { s333JobRealIssuesTab as base } from "./job-real-issues-tab";

const BUSY = "one-view-3";
const GIT = "https://git.example/armada/";

const issue = (number: string, title: string, state: "open" | "closed", pulls: number[] = []): SessionAttachment => ({ kind: "issue", number, title, address: `${GIT}issues/${number}`, state, ...(pulls.length === 0 ? {} : { pull_requests: pulls }) });
const pull = (number: number, title: string, state: "open" | "merged", closes: number[] = []): SessionAttachment => ({ kind: "pull_request", number, title, branch: `armada/change-${number}`, address: `${GIT}pull/${number}`, checks: { state: "passed" }, state, auto: state === "open", ...(state === "open" ? { queued: true as const } : {}), closes });
const doc = (id: string, title: string): SessionAttachment => ({ kind: "artifact", form: "doc", id: `https://docs.example/d/${id}`, title });
const sketch = (title: string): SessionAttachment => ({
  kind: "sketch",
  id: `sketch:${title}`,
  title,
  by: "agent",
  drawing: scene([box("a", 0, 0, "Tab", "Reads the query", { icon: "globe", step: 1 }), box("b", 360, 0, "Fleet", "Asks the tracker", { icon: "server", step: 2 })], [arrow("a", "b", "search", undefined, 2)]),
});

/** The first groups the real Job has finished each worked an issue; naming it puts the issue and its pull request under the group in Completed. */
const ISSUE_OF: Record<string, string> = { g2: "3001", g6: "3002", g3: "3003" };

const task = (id: string, text: string): WorkTask => ({ id, text, state: "open" });
const MORE: WorkGroup[] = [
  { id: "g8", agent: "bridge-engineer", title: "Settings page for the tracker", paths: ["apps/desktop/src/renderer/src/settings"], tasks: [task("t40", "Pick a tracker per repo"), task("t41", "Show the credential command and its last result")] },
  { id: "g9", agent: "docs-writer", title: "Release notes", paths: ["docs"], tasks: [task("t42", "Add the Issues tab to the notes")] },
];

const LEDGER: readonly SessionAttachment[] = [
  issue("3001", "Search issues by a query in the tracker's language", "closed", [3011]),
  pull(3011, "Search issues by query", "merged", [3001]),
  issue("3002", "A tracker mod can run a CLI command", "closed", [3012]),
  pull(3012, "Tracker mods over a CLI", "merged", [3002]),
  issue("3003", "Saved queries survive a restart", "closed", [3013]),
  pull(3013, "Keep saved queries per repo", "merged", [3003]),
  issue("3004", "The Jira query is not ANDed into the selection", "open"),
  issue("3005", "A query with no matches reads as an error", "open"),
  pull(3014, "Name the tracker in the repo picker", "open", [3004]),
  doc("tracker-search", "How a tracker answers a query"),
  doc("mod-kinds", "The tracker kind of mod"),
  sketch("Query path"),
  sketch("Mod lifecycle"),
];

export const s334JobStoryFull: Scenario = {
  ...base,
  name: "job-story-full",
  says: "The real Job with every zone full: Completed holds three finished groups and a dozen things made, Now has ten marks in progress, Next has four groups",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(
        BUSY,
        (one): Session => {
          const plan = one.work?.plan;
          if (plan === undefined) return one;
          const groups = plan.groups.map((group): WorkGroup => (ISSUE_OF[group.id] === undefined ? group : { ...group, issue: ISSUE_OF[group.id]!, tasks: group.tasks.map((one): WorkTask => ({ ...one, state: "done" })) }));
          return { ...one, attachments: [...one.attachments, ...LEDGER], work: { ...one.work, plan: { ...plan, groups: [...groups, ...MORE] } } };
        },
      );
      return store;
    },
  },
};
