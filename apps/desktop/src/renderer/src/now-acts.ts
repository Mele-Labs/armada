// The Cockpit's and the Dashboard's Now, wired: the feed read from every active Job, and what a press
// on a glyph of one does. Apart from `App.tsx` for the reason `commands.ts` is.

import { useEffect, useMemo } from "react";
import type { Active, JobSummary } from "@armada/protocol";
import type { NowActed } from "@armada/jobs";
import type { Confirming } from "./ConfirmAct";
import type { useCommands } from "./commands";
import { feedOf, type Draft } from "./real-feed";

type Args = {
  live: boolean;
  active: Active;
  jobs: readonly JobSummary[];
  draft: Draft | undefined;
  commands: ReturnType<typeof useCommands>;
  onConfirm: (what: Confirming) => void;
  onOpen: (jobId: string) => void;
};

/**
 * Every active Job's Now, read while connected. **The draft wins for a Job it names**, as `feedOf` has it.
 * `onNowAct` sends a glyph's act; `checkActs` is the three Check acts `JobDetail` takes by their prop names.
 */
export function useNowActs({ live, active, jobs, draft, commands, onConfirm, onOpen }: Args) {
  useEffect(() => {
    if (!live) return;
    void window.armada.followActive(true);
    return () => void window.armada.followActive(false);
  }, [live]);
  const feed = useMemo(() => feedOf(active, jobs, draft), [active, jobs, draft]);
  const onNowAct = (jobId: string, { glyph, check }: NowActed) => {
    if (glyph === "skip_all") void commands.skipChecks(jobId);
    else if (glyph === "retry_step") onConfirm({ act: "restart_step", jobId });
    else if (glyph === "redirect") onOpen(jobId);
    else if (check !== undefined) void (glyph === "retry" ? commands.retryCheck : commands.skipCheck)(jobId, check);
  };
  const checkActs = {
    onRetryCheck: (jobId: string, check: string) => void commands.retryCheck(jobId, check),
    onSkipCheck: (jobId: string, check: string) => void commands.skipCheck(jobId, check),
    onSkipChecks: (jobId: string) => void commands.skipChecks(jobId),
  };
  return { feed, onNowAct, checkActs };
}
