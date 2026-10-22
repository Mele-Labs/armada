// `session-ledger` with the Job that wrote up the store clock on a workflow, as every Job is now, so the Cockpit draws its Flight
// control and the page, file and Doc it made are rows of the Artifacts layer. The walk `session-artifacts-panel` plays it;
// `sessions-artifacts.test.tsx` stays on the Job without one.

import { onAWorkflow } from "../sessions/on-a-workflow";
import { s202SessionLedger } from "./session-ledger";

export const s202SessionLedgerWork = onAWorkflow(s202SessionLedger, "session-ledger-work", "A Job on a workflow that made a page, a file and a Doc", ["s10"]);
