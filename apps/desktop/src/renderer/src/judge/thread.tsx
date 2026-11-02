// The judge in the thread and on the plan card. A reading that finished is a quiet line in the thread, where the Job stood when it ended, and a press on it opens
// the artifact at its notes. It raises no call: a finished judge asks nothing of the person. On the plan card the judge's press stands in the answers row, and the
// notes are pinned to the groups and tasks they name.

import { useMemo } from "react";
import type { JudgedPlanProps, SessionThreadRow } from "@armada/components";
import { keyFor } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openAtNotes } from "./artifacts";
import { JudgeAct } from "./JudgeAct";
import { findingOf, useJudgeNotes, useNotesRead } from "./notes";
import {
  openJudge,
  useJudgeApi,
  useJudgeRecords,
  type JudgeArtifact,
  type JudgeRecord,
} from "./store";

/** Where a reading that finished stands among the rows: after the last one that happened by then, or after the last when no row says when. */
const placeAfter = (rows: readonly SessionThreadRow[], finished: string): number => {
  const at = Date.parse(finished);
  let found = -1;
  let timed = false;
  rows.forEach((row, index) => {
    const when = Date.parse(row.at);
    if (Number.isNaN(when)) return;
    timed = true;
    if (when <= at) found = index;
  });
  return timed ? found : rows.length - 1;
};

/** The thread's rows with a line for each reading that finished, where the Job stood when it did. */
export function useJudgedRows(
  session: Session,
  rows: readonly SessionThreadRow[],
): readonly SessionThreadRow[] {
  const records = useJudgeRecords(session.id);
  return useMemo(() => {
    const done = records.filter((one) => one.state === "done" && one.finished_at !== undefined);
    if (done.length === 0) return rows;
    const line = (one: JudgeRecord): SessionThreadRow => ({
      id: `judged:${one.id}`,
      at: one.finished_at!,
      kind: "judged",
      title: one.artifact.title,
      clean: one.notes.length === 0,
      onOpen: () => openAtNotes(session, one.artifact),
    });
    const after = new Map<number, SessionThreadRow[]>();
    for (const one of done) {
      const at = placeAfter(rows, one.finished_at!);
      after.set(at, [
        ...(after.get(at) ?? []),
        line(one),
      ]);
    }
    return [
      ...(after.get(-1) ?? []),
      ...rows.flatMap((row, index) => [
        row,
        ...(after.get(index) ?? []),
      ]),
    ];
    // `session` changes with every row; the lines only need its attachments and id to open the artifact.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    records,
    rows,
    session.id,
    session.attachments,
  ]);
}

/** The plan as a judge has read it, in the shape the plan card draws it. Absent where no judge answers, or the plan is not the one waiting. */
export function usePlanJudge(session: Session):
  | (JudgedPlanProps & {
      artifact: JudgeArtifact;
    })
  | undefined {
  const api = useJudgeApi();
  const version = session.work?.plan?.version ?? 1;
  const artifact: JudgeArtifact = {
    kind: "plan",
    id: String(version),
    title: `Plan v${version}`,
  };
  const { record, notes, send, dismiss } = useJudgeNotes(session.id, artifact);
  useNotesRead(session.id, artifact, record !== undefined && record.notes.length > 0);
  if (api === undefined || session.work?.plan === undefined || session.work.plan.approved)
    return undefined;
  return {
    artifact,
    findings: notes.map(findingOf),
    onRun: () => openJudge(session.id, artifact),
    onSend: send,
    onDismiss: dismiss,
    press: (
      <JudgeAct
        job={session.id}
        artifact={artifact}
        label="Judge plan"
        shortcut={keyFor("plan_judge")}
      />
    ),
  };
}
