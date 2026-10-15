// The file open beside a Session's pull request panel: which one, Fleet's patch for it, and the diff
// that patch draws as. Apart from `session-pull-request-page.ts`, which holds the page.

import { useEffect, useRef, useState } from "react";
import type { UnifiedDiffProps } from "@armada/components";
import { drawnOf } from "@armada/screens/src/drawn-patch";
import type { PullRequestFileRead, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** What a cut patch says. The forge holds the rest, and that is where it is read. */
const cutSentence = (drawnLines: number, total: number): string =>
  `The first ${drawnLines.toLocaleString()} of ${total.toLocaleString()} lines. Open the pull request in the browser for the rest.`;

const NO_CHANGE = "No lines changed in this file.";

/**
 * The diff for one file. **Empty with no note while the read is in flight**, as a Job's diff says
 * nothing while it is read; a refusal and a file with no patch say Fleet's own sentence.
 *
 * The forge's patch has no `diff --git` header, so one is written for the split to name the file by.
 */
export function fileDiffOf(path: string, read: PullRequestFileRead | undefined): UnifiedDiffProps {
  if (read === undefined) return { files: [] };
  if (!read.ok) return { files: [], emptyNote: read.said };
  const patch = read.file.patch;
  if (patch === undefined) return { files: [], emptyNote: read.file.said ?? NO_CHANGE };
  const drawn = drawnOf(`diff --git a/${path} b/${path}\n${patch}`, cutSentence);
  if (drawn.files.length === 0) return { files: [], emptyNote: NO_CHANGE };
  return { files: drawn.files, ...(drawn.cut === undefined ? {} : { cut: drawn.cut }) };
}

/** The file pressed in the Files tab and what Fleet read for it. Pressing the open one again closes it. */
export function usePullRequestFile(sessionId: string, number: number, draft: SessionsDraft | undefined) {
  const held = useRef(draft);
  held.current = draft;
  const here = useRef(true);
  const [open, setOpen] = useState<string | undefined>(undefined);
  const [read, setRead] = useState<{ path: string; answer: PullRequestFileRead } | undefined>(undefined);
  const served = draft?.pullRequestFile !== undefined;

  useEffect(() => {
    here.current = true;
    return () => {
      here.current = false;
    };
  }, []);

  useEffect(() => {
    if (open === undefined) return;
    let current = true;
    void held.current?.pullRequestFile?.(sessionId, number, open).then((answer) => {
      if (current && here.current) setRead({ path: open, answer });
    });
    return () => {
      current = false;
    };
  }, [open, sessionId, number]);

  const answer = read?.path === open ? read?.answer : undefined;
  const file = open === undefined ? undefined : { path: open, diff: fileDiffOf(open, answer) };
  return { file, onFile: served ? (path: string | null) => setOpen(path ?? undefined) : undefined };
}
