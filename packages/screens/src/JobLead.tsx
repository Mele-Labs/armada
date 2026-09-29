// The banner that leads a Job's Overview — a sentence, what it is holding up,
// and at most one act.
//
// **One copy, because the train already had one.** `plan-lead.tsx` is the scar
// that says why: a sentence spelled twice came back wrong the first time one
// of the two was fixed.

import type { ReactNode } from "react";

export type JobLeadProps = {
  said: string;
  because: string;
  /** Colours the edge — the thing outstanding is found before a word is read. */
  tone?: "awaiting-review" | "completed-failed";
  /** The one act, where the screen has somewhere to send a person. */
  act?: ReactNode;
};

export function JobLead({ said, because, tone, act }: JobLeadProps) {
  return (
    <div className="armada-lead" data-tone={tone}>
      <div className="armada-lead__said">
        <p className="armada-lead__headline">{said}</p>
        {because === "" ? null : <p className="armada-lead__because">{because}</p>}
      </div>
      {act}
    </div>
  );
}
