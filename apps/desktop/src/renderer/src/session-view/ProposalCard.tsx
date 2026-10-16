// What the proposer read the request to be, a card at the head of the Session's thread above the dispatched words: the title, the
// workflow (none is a custom Session), the model and the effort. Each can be changed until Start; after it the card stays as the
// record of what the Session began with.

import { useState } from "react";
import { Button, Input, Select } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SvPanel } from "./SvPanel";

const AUTO = "auto";
const NONE = "none";

export function ProposalCard({ session, draft }: { session: Session; draft: SessionsDraft }) {
  const acts = draft.work;
  const proposal = session.work?.proposal;
  const [title, setTitle] = useState<string | undefined>();
  if (acts === undefined || proposal === undefined) return null;
  const { id } = session;
  const saved = session.title ?? "";
  const save = () => {
    const next = (title ?? saved).trim();
    setTitle(undefined);
    if (next !== "" && next !== saved) draft.rename?.(id, next);
  };
  const mode = session.mode ?? "auto";
  const tune = (change: { model?: string | null; effort?: string | null }) =>
    draft.tune(id, { model: session.model ?? null, effort: session.effort ?? null, mode, ...change });
  if (proposal.started === true) {
    const settled: readonly (readonly [string, string])[] = [
      ["Workflow", proposal.workflow ?? "None"],
      ["Model", session.model ?? "Auto"],
      ["Effort", session.effort ?? "Auto"],
    ];
    return (
      <SvPanel
        label="Proposal"
        area="proposal"
        place="thread"
        acts={
          <dl className="sv-proposal--settled">
            {settled.map(([name, value]) => (
              <div key={name}>
                <dt>{name}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        }
      />
    );
  }
  return (
    <SvPanel label="Proposal" area="proposal" place="thread" state="asks" acts={<Button variant="primary" size="sm" onClick={() => acts.start(id)}>Start</Button>}>
      <div className="sv-proposal">
        <Input
          aria-label="Title"
          value={title ?? saved}
          onChange={(event) => setTitle(event.target.value)}
          onBlur={save}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
          }}
        />
        <div className="sv-proposal__row">
          <Select label="Workflow" value={proposal.workflow ?? NONE} onChange={(event) => acts.choose(id, event.target.value === NONE ? null : event.target.value)}>
            <option value={NONE}>None</option>
            {acts.workflows.map((one) => (
              <option key={one.name} value={one.name}>
                {one.name}
              </option>
            ))}
          </Select>
          <Select label="Model" value={session.model ?? AUTO} onChange={(event) => tune({ model: event.target.value === AUTO ? null : event.target.value })}>
            <option value={AUTO}>Auto</option>
            {draft.models.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
          <Select label="Effort" value={session.effort ?? AUTO} onChange={(event) => tune({ effort: event.target.value === AUTO ? null : event.target.value })}>
            <option value={AUTO}>Auto</option>
            {draft.efforts.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
        </div>
      </div>
    </SvPanel>
  );
}
