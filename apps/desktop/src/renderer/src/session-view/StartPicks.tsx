// What a held Job settles before any agent moves, in a strip pinned to the composer: its Workflow and its Delivery, beside the model and effort
// the bar already has. The first message is the box, Send reads Start, and a press begins the agent. Nothing here once the Job has started.
// Neither is ever blank once the proposer has read: each carries a default, and a mark only where the proposer or his usual chose it.

import { useState } from "react";
import type { ReactNode } from "react";
import { GitMerge, Quote, Repeat, TextCursor, Workflow } from "lucide-react";
import { Pick, Select, Tooltip } from "@armada/components";
import type { DeliveryChoice, RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";

/** How a Job's work lands, in the order Fleet offers it: `DeliveryChoice::label` and `detail` in `crates/ipc/src/session_delivery.rs`. */
const LANDS: readonly { choice: DeliveryChoice; label: string; detail: string }[] = [
  { choice: "auto_land", label: "Auto land", detail: "Open a pull request, watch ci, merge it." },
  { choice: "open_pr", label: "Open PR only", detail: "Stop at the open pull request." },
  { choice: "keep_local", label: "Keep local", detail: "Stop at the commit on the job's branch. Nothing is pushed." },
];

const plural = (name: string): string => {
  const lower = name.toLowerCase();
  return /(s|x|ch|sh)$/.test(lower) ? `${lower}es` : `${lower}s`;
};

/** The Kind and Lands picks for a held Job and the press that starts it; `undefined` for a Job that is not held. */
export function useStartPicks(session: Session, repositories: readonly RepositorySummary[], compact: boolean): { lead: ReactNode; begin: (text: string) => Promise<boolean> } | undefined {
  const draft = useSessionsDraft();
  const [answered, setAnswered] = useState<{ id: string; choice: DeliveryChoice } | undefined>(undefined);
  const start = session.start;
  if (start === undefined || draft === undefined) return undefined;

  const { id } = session;
  const proposal = session.work?.proposal;
  const reading = proposal?.reading === true;
  const picked = proposal?.picked ?? [];
  const kinds = (draft.work?.workflows ?? []).filter((one) => one.manifestId === undefined || session.manifestId === undefined || one.manifestId === session.manifestId);
  // Fleet will supply these defaults (`start.picked`, the Job's workflow) once wired; until then Bridge decides: the proposer's pick, else the
  // repository's `feature` workflow, else its first; the landing his words named, else his usual, else Auto land.
  const proposed = proposal?.workflow ?? null;
  const kind = reading ? null : (proposed ?? kinds.find((one) => one.id === "feature")?.id ?? kinds[0]?.id ?? null);
  const kindName = kinds.find((one) => one.id === kind)?.name;
  const repository = repositories.find((one) => one.manifest?.id === session.manifestId)?.manifest?.repository;
  const lands = start.asksDelivery;
  const landing = reading ? undefined : ((answered?.id === id ? answered.choice : undefined) ?? start.picked?.choice ?? "auto_land");

  // The standing choice is marked while it is the one in use: his usual, or the one his words named.
  const standing = start.picked !== undefined && landing === start.picked.choice ? start.picked.from : undefined;
  const standingSaid =
    standing === "named" ? "Named in your request" : kindName === undefined || repository === undefined ? "Your usual" : `Your usual for ${plural(kindName)} in ${repository}`;

  const begin = async (text: string): Promise<boolean> => {
    if (reading || draft.begin === undefined) return false;
    // The workflow shown is the one the Job runs under, so a default he did not change is recorded now.
    if (proposed === null && kind !== null) draft.work?.choose(id, kind);
    const sent = await draft.begin(id, lands ? landing : undefined, start.message === undefined ? undefined : text);
    return sent !== false;
  };

  const lead = (
    <>
      {reading ? (
        <Tooltip label="Reading the request">
          <span role="img" aria-label="Reading the request" className="armada-session-composer__proposed">
            <TextCursor size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
      <Pick Glyph={Workflow} compact={compact} proposed={kind !== null && kind === proposed && picked.includes("workflow")} label="Workflow">
        <Select aria-label="Workflow" value={kind ?? ""} disabled={reading} onChange={(event) => draft.work?.choose(id, event.target.value)}>
          {kind === null ? <option value="">Workflow</option> : null}
          {kinds.map((one) => (
            <option key={one.id} value={one.id}>
              {one.name}
            </option>
          ))}
        </Select>
      </Pick>
      {lands ? (
        <Pick
          Glyph={GitMerge}
          compact={compact}
          label={LANDS.find((one) => one.choice === landing)?.detail ?? "Delivery"}
          {...(standing === undefined ? {} : { mark: { Glyph: standing === "named" ? Quote : Repeat, label: standingSaid } })}
        >
          <Select aria-label="Delivery" value={landing ?? ""} disabled={reading} onChange={(event) => setAnswered({ id, choice: event.target.value as DeliveryChoice })}>
            {landing === undefined ? <option value="">Select delivery</option> : null}
            {LANDS.map((one) => (
              <option key={one.choice} value={one.choice} title={one.detail}>
                {one.label}
              </option>
            ))}
          </Select>
        </Pick>
      ) : null}
    </>
  );

  return { lead, begin };
}
