// A File row's reading: the part of the Job's diff its paths name. A task row
// draws the same part for its own files.

import { useMemo } from "react";
import { UnifiedDiff } from "@armada/components";
import type { Diff } from "@armada/protocol";

import { Eyebrow } from "./regions";
import { drawn as drawnPatch, whyNoDiff } from "./review";

/**
 * The part of the Job's diff a File row names.
 *
 * **The branch's change to the file, not the task's.** Fleet serves one patch
 * for the Job, so a file two tasks wrote shows both, and the eyebrow says whose
 * diff it is. Nothing while it is being read or where it holds no section for
 * the file; a read that failed keeps `whyNoDiff`'s words.
 */
export function FileDiff({ paths, diff, jobId }: { paths: readonly string[]; diff: Diff; jobId: string }) {
  const mine = diff.state !== "none" && diff.jobId === jobId ? diff : null;
  const work = mine?.state === "read" ? mine.work : undefined;
  const patch = useMemo(() => (work === undefined ? undefined : drawnPatch(work)), [work]);
  if (mine?.state === "failed") {
    return (
      <section className="armada-ledger__read-section">
        <Eyebrow>{JOB_DIFF}</Eyebrow>
        <p className="armada-ledger__note">{whyNoDiff(diff, jobId)}</p>
      </section>
    );
  }
  if (patch === undefined || paths.length === 0) return null;
  const files = patch.files.filter((file) =>
    paths.some((path) => file.path === path || file.path.startsWith(path.endsWith("/") ? path : `${path}/`)),
  );
  if (files.length === 0) return null;
  // The bound can fall inside the last file drawn, so that file says so.
  const last = patch.files[patch.files.length - 1];
  const cut = patch.cut !== undefined && last !== undefined && files.includes(last) ? patch.cut : undefined;
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>{JOB_DIFF}</Eyebrow>
      <UnifiedDiff files={files} emptyNote="" {...(cut === undefined ? {} : { cut })} />
    </section>
  );
}

/** Whose diff a File row draws: the branch's, which any task may have added to. */
const JOB_DIFF = "Diff on the branch";
