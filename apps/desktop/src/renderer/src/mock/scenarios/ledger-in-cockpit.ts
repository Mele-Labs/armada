// The Cockpit's Ledger panel on a Session that holds a pull request, a few artifacts and background commands,
// with two items waiting on the person in the thread. Over the `one-session-view` scenario, with its Session
// "Queue session for later" given the rest. The walk `ledger-in-cockpit` plays it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { backgroundAttachments } from "./ledger-background";
import { s261OneSessionView as base } from "./one-session-view";

const LIVE = "one-view-5";

const pull = (number: number, state: "open" | "draft", title: string): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch: "bridge/queue-for-later",
  address: `https://example.com/pull/${number}`,
  checks: { state: state === "draft" ? "pending" : "passed" },
  state,
  auto: false,
});

const holding = (one: Session): Session => ({
  ...one,
  attachments: [
    ...one.attachments,
    pull(2431, "open", "Queue a Session for later"),
    pull(2433, "draft", "Show the queue in the list"),
    { kind: "artifact", form: "doc", id: "https://example.com/artifact/queue-notes", title: "Queue notes" },
    { kind: "artifact", form: "file", id: "/tmp/queue-dry-run.txt", title: "Queue dry run" },
    ...backgroundAttachments,
  ],
});

export const s267LedgerInCockpit: Scenario = {
  ...base,
  name: "ledger-in-cockpit",
  says: "A Session selected on the Cockpit, its Ledger panel holding pull requests, artifacts and background commands, and items waiting in the thread",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(LIVE, holding);
      return store;
    },
  },
};
