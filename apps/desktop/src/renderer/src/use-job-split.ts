// A Job's split for its System tab, read once from Fleet when the tab opens. Nothing is drawn before the answer or where Fleet
// keeps none for the Job (it is still running, or ended before splits were kept), so a running Job's tab never draws a section
// and takes it back.

import { useEffect, useState } from "react";
import type { TimeJob, TimeSplit } from "@armada/components";

import { jobTimeOf } from "./usage-time-wire";

type Split = {
  job: TimeJob;
  usual: TimeSplit | undefined;
};

export function useJobSplit(sessionId: string, title: string | undefined): Split | undefined {
  const [split, setSplit] = useState<Split | undefined>(undefined);
  useEffect(() => {
    let live = true;
    setSplit(undefined);
    void window.armada.getJobTime(sessionId).then((answer) => {
      if (live && answer.ok) setSplit(jobTimeOf(answer.value, title));
    });
    return () => {
      live = false;
    };
  }, [
    sessionId,
    title,
  ]);
  return split;
}
