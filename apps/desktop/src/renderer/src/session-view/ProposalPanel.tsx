// What the proposer read the request to be, filled in on the Session's view when the words are dispatched: the title, the workflow
// (none is a custom Session), the model and the effort. Each can be changed until the work starts, and Start begins it.

import { useState } from "react";
import { Button, Input, Select, Tooltip } from "@armada/components";
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
  const compact = place === "foot";
  const mode = session.mode ?? "auto";
  // In the foot there is no room for a label over each field: the field names itself in a tooltip.
  const field = (label: string, value: string, onChange: (value: string) => void, options: readonly (readonly [string, string])[]) => {
    const choices = options.map(([key, said]) => (
      <option key={key} value={key}>
        {said}
      </option>
    ));
    return compact ? (
      <Tooltip key={label} label={label}>
        <div>
          <Select aria-label={label} value={value} onChange={(event) => onChange(event.target.value)}>
            {choices}
          </Select>
        </div>
      </Tooltip>
    ) : (
      <Select key={label} label={label} value={value} onChange={(event) => onChange(event.target.value)}>
        {choices}
      </Select>
    );
  };
  const tune = (change: { model?: string | null; effort?: string | null }) =>
    draft.tune(id, { model: session.model ?? null, effort: session.effort ?? null, mode, ...change });
  return (
    <SvPanel label="Proposal" area="proposal" place={place} state="asks" acts={<Button variant="primary" size="sm" onClick={() => acts.start(id)}>Start</Button>}>
      <div className="sv-proposal" data-compact={compact || undefined}>
        {compact ? null : (
          <Input
            aria-label="Title"
            value={title ?? saved}
            onChange={(event) => setTitle(event.target.value)}
            onBlur={save}
            onKeyDown={(event) => {
              if (event.key === "Enter") event.currentTarget.blur();
            }}
          />
        )}
        {field("Workflow", session.work?.proposal?.workflow ?? NONE, (value) => acts.choose(id, value === NONE ? null : value), [[NONE, "None"], ...acts.workflows.map((one) => [one.name, one.name] as const)])}
        {field("Model", session.model ?? AUTO, (value) => tune({ model: value === AUTO ? null : value }), [[AUTO, "Auto"], ...draft.models.map((one) => [one, one] as const)])}
        {field("Effort", session.effort ?? AUTO, (value) => tune({ effort: value === AUTO ? null : value }), [[AUTO, "Auto"], ...draft.efforts.map((one) => [one, one] as const)])}
      </div>
    </SvPanel>
  );
}
