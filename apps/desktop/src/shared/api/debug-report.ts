// Copy debug report's two reads: Fleet's text over the connection, and where Fleet does not answer, the text
// `armada report` prints, which main runs. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { DebugReport, Outcome } from "@armada/protocol";

/** What asking Fleet for the report came to: its text, or the refusal to word. */
export type DebugReportRead =
  | {
      ok: true;
      value: DebugReport;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** What running `armada report` came to: what it printed, or one sentence on why it printed none. */
export type ArmadaReportRun =
  | {
      ok: true;
      text: string;
    }
  | {
      ok: false;
      why: string;
    };

export type DebugReportApi = {
  /** `GET /debug_report`, with `root` the repository selected in the window, or `null` for All repositories. */
  getDebugReport: (root: string | null) => Promise<DebugReportRead>;
  /** Runs `armada report` in `root`, or in the home folder for `null`. Read-only. */
  runArmadaReport: (root: string | null) => Promise<ArmadaReportRun>;
};

export const DEBUG_REPORT_CHANNELS = {
  getDebugReport: "bridge:get-debug-report",
  runArmadaReport: "bridge:run-armada-report",
} as const;
