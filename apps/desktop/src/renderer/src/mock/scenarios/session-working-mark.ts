// A Session mid-turn whose last row is a folded tool group. The walk `session-working-mark` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const working: Session = {
  id: "s20",
  title: "Backfill dry run",
  turn: { state: "working" },
  lastTurn: "14:01",
  rows: [
    { id: "w1", at: "14:00:12", kind: "message", from: { kind: "you" }, text: "Dry-run the backfill against the copy." },
    { id: "w2", at: "14:00:40", kind: "tool", text: "Read crates/store/src/backfill.rs" },
    { id: "w3", at: "14:01:02", kind: "tool", text: "Bash cargo run -p store --example backfill -- --dry-run" },
  ],
  attachments: [],
};

export const s205SessionWorkingMark: Scenario = {
  ...s200Sessions,
  name: "session-working-mark",
  says: "A Session mid-turn: the working mark on its last tool group",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [working]) },
};
