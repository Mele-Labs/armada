// What the proposer read the request to be, filled in on the Session's view when the words are dispatched: the title, the workflow
// (none is a custom Session), the model and the effort. Each can be changed until the work starts, and Start begins it.

import { useState } from "react";
import { Button, Input, Select } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SvPanel, type Place } from "./SvPanel";

const AUTO = "auto";
const NONE = "none";

export function ProposalPanel({ session, draft, place }: { session: Session; draft: SessionsDraft; place: Place }) {
  const acts = draft.work;
  const [title, setTitle] = useState<string | undefined>();
  if (acts === undefined) return null;
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
  return (
    <SvPanel label="Proposal" area="proposal" place={place} state="asks" acts={<Button variant="primary" size="sm" onClick={() => acts.start(id)}>Start</Button>}>
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
        <Select label="Workflow" value={session.work?.proposal?.workflow ?? NONE} onChange={(event) => acts.choose(id, event.target.value === NONE ? null : event.target.value)}>
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
    </SvPanel>
  );
}
