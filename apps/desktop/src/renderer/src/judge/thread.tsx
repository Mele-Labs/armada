// The judge in the thread and on the plan card. A reading that finished is a quiet line in the thread, where the Job stood when it ended, and a press on it opens
// the artifact at its notes. It raises no call: a finished judge asks nothing of the person. On the plan card the judge's press stands in the review's, and the
// notes are pinned to the groups and tasks the way the review's findings were.

import { useMemo } from "react";
import type { AdversarialReviewProps, SessionThreadRow } from "@armada/components";
import { keyFor } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openAtNotes } from "./artifacts";
import { JudgeAct } from "./JudgeAct";
import { reviewOf, useJudgeNotes, useNotesRead } from "./notes";
import { openJudge, useJudgeApi, useJudgeRecords, type JudgeArtifact } from "./store";

/** The thread's rows with a line for each reading that finished, after the row that was last when it did. */
export function useJudgedRows(session: Session, rows: readonly SessionThreadRow[]): readonly SessionThreadRow[] {
  const records = useJudgeRecords(session.id);
  return useMemo(() => {
    const done = records.filter((one) => one.state === "done" && one.finished_at !== undefined);
    if (done.length === 0) return rows;
    const line = (one: (typeof done)[number]): SessionThreadRow => ({ id: `judged:${one.id}`, at: one.finished_at!, kind: "judged", title: one.artifact.title, clean: one.notes.length === 0, onOpen: () => openAtNotes(session, one.artifact) });
    const out: SessionThreadRow[] = [];
    const placed = new Set<string>();
    for (const row of rows) {
      out.push(row);
      for (const one of done) if (one.after === row.id && !placed.has(one.id)) (placed.add(one.id), out.push(line(one)));
    }
    for (const one of done) if (!placed.has(one.id)) out.push(line(one));
    return out;
    // `session` changes with every row; the lines only need its attachments and id to open the artifact.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [records, rows, session.id, session.attachments]);
}

/** The plan as a judge has read it, in the shape the plan card draws a review. Absent where no judge answers, or the plan is not the one waiting. */
export function usePlanJudge(session: Session): (AdversarialReviewProps & { artifact: JudgeArtifact }) | undefined {
  const api = useJudgeApi();
  const version = session.work?.plan?.version ?? 1;
  const artifact: JudgeArtifact = { kind: "plan", id: String(version), title: `Plan v${version}` };
  const { record, send, dismiss } = useJudgeNotes(session.id, artifact);
  const reading = record === undefined ? undefined : record;
  useNotesRead(session.id, artifact, record !== undefined && record.notes.length > 0);
  if (api === undefined || session.work?.plan === undefined || session.work.plan.approved) return undefined;
  return {
    artifact,
    review: reviewOf(reading),
    onRun: () => openJudge(session.id, artifact),
    onSend: send,
    onDismiss: dismiss,
    shortcut: keyFor("plan_adversarial_review"),
    press: <JudgeAct job={session.id} artifact={artifact} label="Judge plan" shortcut={keyFor("plan_adversarial_review")} />,
  };
}
