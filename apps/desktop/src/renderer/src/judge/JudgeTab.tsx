// The Judge of the whole Job, in a tab of its own: it has no artifact to stand in, as a pull request or a document has. At its foot, what judging is and a small box that asks;
// above that, what was asked and how the reading stands: its shape while it reads, why it failed, or its notes, each saying where it is. A clean reading says so. Readings of
// other artifacts stand in those artifacts (`notes.tsx`).

import { useEffect, useState } from "react";
import { CircleCheck } from "lucide-react";
import { SessionComposer, Skeleton } from "@armada/components";

import { JudgeNotes } from "./notes";
import { notesBrought, useJudgeApi, useJudgeRecord, useNotesAsked } from "./store";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

export function JudgeTab({ job, title }: { job: string; title: string }) {
  const artifact = { kind: "job", id: job, title } as const;
  const api = useJudgeApi();
  const record = useJudgeRecord(job, artifact);
  const draftKey = `judge:${job}`;
  // The Judge's press asked for this tab: its box takes the keyboard, and a tab that comes back on its own does not take it.
  const asked = useNotesAsked(job, artifact);
  const [at, setAt] = useState(0);
  useEffect(() => {
    if (!asked) return;
    setAt((was) => was + 1);
    notesBrought();
  }, [asked]);
  if (api === undefined) return null;
  return (
    <div className="judge-tab">
      <div className="judge-tab__reading">
        {record === undefined ? null : (
          <>
            <dl className="judge-tab__asked">
              <dt>{record.prompt === "" ? "Asked" : "Verify that"}</dt>
              <dd>{record.prompt === "" ? "What did we miss?" : record.prompt}</dd>
            </dl>
            {record.state === "reading" ? (
              <div className="judge-tab__pending" role="status" aria-label="Judge reading">
                <Skeleton width="30%" />
                <Skeleton width="90%" />
                <Skeleton width="60%" />
              </div>
            ) : record.state === "failed" ? (
              <p className="judge-tab__said" data-failed="">
                {record.failed}
              </p>
            ) : record.notes.length === 0 ? (
              <p className="judge-tab__said">
                <CircleCheck className="judge-act__clean" size={16} strokeWidth={2} aria-hidden="true" />
                Nothing missed
              </p>
            ) : (
              <JudgeNotes job={job} artifact={artifact} label="Notes from the judge" />
            )}
          </>
        )}
      </div>
      <div className="judge-tab__foot">
        <p className="judge-tab__what">A Judge reads this Job once on the strongest model, read-only. It sees what you asked for, the work and the repository, and never the agent's account of it. A note reaches the agent only if you send it.</p>
        <p className="judge-tab__what">Say what to verify. Left empty, it looks for what was missed.</p>
        <SessionComposer
          textOnly
          working={false}
          disabled={record?.state === "reading"}
          label="Verify that"
          sendLabel="Judge"
          sendEmpty
          model={null}
          effort={null}
          models={NONE}
          efforts={NONE}
          onTune={NOBODY}
          commands={NONE}
          taggable={NONE}
          tags={NONE}
          onTags={NOBODY}
          drawn={NONE}
          onDraw={NOBODY}
          onRemoveDrawn={NOBODY}
          draftKey={draftKey}
          {...(at === 0 ? {} : { focusFor: { key: draftKey, at } })}
          grows="panel"
          onSend={(sent) => api.ask({ job, artifact, prompt: sent.text.trim() })}
        />
      </div>
    </div>
  );
}
