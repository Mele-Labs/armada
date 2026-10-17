// The Cockpit's one Session view over `one-session-view`, with two Sessions that each hold a pull request in a merge line of four. The
// Field opens on the whole floor for each Session, the Ledger stands above Status, and the Merge line frames the open Session's own
// pull requests. The walk `sessionViewFeedback` plays it.

import type { MergeLine } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView } from "./one-session-view";

// The Sessions of `one-session-view` by id: "Retry the slot lease" and "Rotate the audit log".
const RETRY = "one-view-1";
const ROTATE = "one-view-3";

const pull = (number: number, title: string, branch: string, ci: "passed" | "running") => ({ number, title, branch, url: `https://git.example/armada/pull/${number}`, ci });

const attached = (number: number, title: string, branch: string): Session["attachments"][number] => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `https://git.example/armada/pull/${number}`,
  checks: { state: "passed" },
  state: "draft",
  auto: false,
});

const root = s261OneSessionView.state.holds.repositories?.[0]?.root ?? "";

const line: MergeLine = {
  root,
  line: [],
  off: [],
  landed: [],
  sent_back: [],
  hub: {
    pull_requests: [
      pull(2371, "Name the store migrations", "fix/migration-names", "passed"),
      pull(2388, "Rotate the audit log", "armada/audit-rotation", "running"),
      pull(2390, "Retry the slot lease", "armada/lease-retry", "passed"),
      pull(2394, "Link the readme", "docs/readme-links", "passed"),
    ],
  },
};

export const s265SessionViewFeedback: Scenario = {
  ...s261OneSessionView,
  name: "session-view-feedback",
  says: "The one Session view: the Field fitted whole for each Session, the Ledger above Status, and the Merge line framing the open Session's pull requests",
  state: { ...s261OneSessionView.state, mergeLines: { lines: [line] } },
  draft: {
    ...s261OneSessionView.draft,
    sessions: (board, more) => {
      const store = s261OneSessionView.draft!.sessions!(board, more);
      store.edit(RETRY, (one) => ({ ...one, attachments: [...one.attachments, attached(2390, "Retry the slot lease", "armada/lease-retry")] }));
      // The Session at the audit log has drones over two parts of the repository, so its Field is not the Session's alone.
      store.edit(ROTATE, (one) => ({
        ...one,
        attachments: [...one.attachments, attached(2388, "Rotate the audit log", "armada/audit-rotation")],
        work: {
          ...one.work,
          field: [
            { id: "main", path: "crates/store", doing: "write", file: "crates/store/src/audit.rs" },
            { id: "bridge", agent: "bridge-engineer", task: "List rotated files in System", path: "apps/desktop", doing: "write", file: "apps/desktop/src/renderer/src/system/Audit.tsx" },
          ],
          files: [{ path: "crates/store/src/audit.rs", act: "edit" }, { path: "apps/desktop/src/renderer/src/system/Audit.tsx", act: "write" }],
        },
      }));
      return store;
    },
  },
};
