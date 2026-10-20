// A Session whose agent ran `armada check test` a dozen times in a row, a typecheck once between,
// and the last `test` still out. The runs of one Check that sit side by side fold into one row; the
// typecheck is the boundary. The walk `fold-repeated-check-runs` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

type Run = Extract<Session["rows"][number], { kind: "check" }>;

const test = (n: number, at: string, state: Run["state"], target?: string): Run => ({
  id: `f${n}`,
  at,
  kind: "check",
  name: "test",
  run: n,
  state,
  ...(target === undefined ? {} : { target }),
});

const folded: Session = {
  id: "s40",
  title: "Session check folds",
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  rows: [
    { id: "f0", at: "14:50:10", kind: "message", from: { kind: "you" }, text: "Get the store reads passing." },
    { id: "f0a", at: "14:50:40", kind: "tool", text: "Edit crates/store/src/reads.rs" },
    test(1, "14:51:02", "failed", "store::reads_the_last_row"),
    test(2, "14:52:18", "failed", "store::reads_the_last_row"),
    test(3, "14:53:30", "passed", "store::reads_the_last_row"),
    test(4, "14:54:41", "passed", "--changed store"),
    test(5, "14:55:50", "failed", "--changed fleet, ipc"),
    test(6, "14:57:03", "passed"),
    test(7, "14:58:20", "passed", "store::writes_in_order"),
    test(8, "14:59:12", "failed", "--changed fleet, ipc, store"),
    { id: "f9", at: "15:00:30", kind: "check", name: "desktop:typecheck", run: 9, state: "passed" },
    test(10, "15:01:44", "passed", "--changed fleet"),
    test(11, "15:03:10", "failed", "fleet::wakes_a_parked_drone"),
    test(12, "15:04:36", "passed", "fleet::wakes_a_parked_drone"),
    test(13, "15:06:02", "passed"),
    test(14, "15:12:00", "running", "--changed fleet, ipc"),
  ],
  attachments: [],
};

export const s235FoldRepeatedCheckRuns: Scenario = {
  ...s200Sessions,
  name: "fold-repeated-check-runs",
  says: "A Session whose agent ran one Check over and over: the runs fold into one row, and a different Check between them starts another",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [folded]) },
};
