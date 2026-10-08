// A fix's file, opened in its diff: the repair or side-run branch against the Job's own, in the
// sheet the Job's diff opens in. Read once when it opens, from the repository, so it answers after
// the Drone is done and until the fix is placed.

import { useEffect, useMemo, useState } from "react";

import { JobDiffSheet, railOfPatch, UnifiedDiff } from "@armada/components";
import type { SideBranch } from "@armada/components";
import type { Work } from "@armada/protocol";

import { drawn } from "./review";
import type { FixOf } from "./repair-branch";

/** Reads a fix's patch against the Job's branch. `work` is absent where Fleet has no branch to read. */
export type ReadRepairDiff = (jobId: string, of: FixOf) => Promise<{ ok: true; work?: Work } | { ok: false }>;

/** A fix's file being opened: which fix, and the file the rail starts on. */
export type RepairFileOpen = { trigger: SideBranch; path: string };

export const fixNamed = (trigger: SideBranch): FixOf => (trigger.addition === undefined ? { trigger: trigger.name } : { addition: trigger.addition });

export function RepairDiffSheet({
  jobId,
  open,
  read,
  floor,
  onClose,
}: {
  jobId: string;
  open: RepairFileOpen;
  read: ReadRepairDiff;
  floor: boolean;
  onClose: () => void;
}) {
  const { trigger } = open;
  const [work, setWork] = useState<Work | null>(null);
  const [selected, setSelected] = useState(open.path);
  useEffect(() => {
    let live = true;
    setWork(null);
    setSelected(open.path);
    void read(jobId, fixNamed(trigger)).then((answer) => {
      if (live && answer.ok && answer.work !== undefined) setWork(answer.work);
    });
    return () => {
      live = false;
    };
  }, [jobId, trigger, open.path, read]);
  const files = useMemo(() => (work === null ? null : drawn(work).files), [work]);
  const shown = files?.find((file) => file.path === selected);
  return (
    <JobDiffSheet
      open
      floor={floor}
      title="Fix diff"
      branch={trigger.repair?.branch ?? trigger.name}
      files={files === null ? null : railOfPatch(files)}
      {...(work?.measured_from === undefined ? {} : { measuredFrom: work.measured_from })}
      selected={selected}
      onSelect={setSelected}
      note=""
      onClose={onClose}
    >
      {shown === undefined ? null : <UnifiedDiff files={[shown]} />}
    </JobDiffSheet>
  );
}
