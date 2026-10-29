// What a held Job settles before any agent moves, in a strip pinned to the composer: its Workflow and its Delivery, beside the model and effort
// the bar already has. The first message is the box, Send reads Start, and a press begins the agent. Nothing here once the Job has started.
// Neither is ever blank once the proposer has read: each carries a default, and a mark only where the proposer or his usual chose it. Workflow lists the repository's
// workflows except those switched off for the machine or for the Job's repository (`workflows.off`, `../workflows-off.ts`), and None is always there.

import { useState } from "react";
import type { ReactNode } from "react";
import { GitMerge, LoaderCircle, Quote, Repeat, TextCursor, Workflow } from "lucide-react";
import { Pick, Select, Tooltip } from "@armada/components";
import type { DeliveryChoice, RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { useSetting } from "../use-setting";
import { WORKFLOWS_OFF_KEY, isOff, workflowsOffOf } from "../workflows-off";

/** How a Job's work lands, in the order Fleet offers it: `DeliveryChoice::label` and `detail` in `crates/ipc/src/session_delivery.rs`. */
const LANDS: readonly { choice: DeliveryChoice; label: string; detail: string }[] = [
  { choice: "auto_land", label: "Auto land", detail: "Open a pull request, watch ci, merge it." },
  { choice: "open_pr", label: "Open PR only", detail: "Stop at the open pull request." },
  { choice: "keep_local", label: "Keep local", detail: "Stop at the commit on the job's branch. Nothing is pushed." },
];

const NO_WORKFLOW = "none";

const plural =(name: string): string => {
  const lower = name.toLowerCase();
  return /(s|x|ch|sh)$/.test(lower) ? `${lower}es` : `${lower}s`;
};

/** The Kind and Lands picks for a held Job and the press that starts it; `undefined` for a Job that is not held. */
export function useStartPicks(session: Session, repositories: readonly RepositorySummary[], compact: boolean): { lead: ReactNode; begin: (text: string) => Promise<boolean> } | undefined {
  const draft = useSessionsDraft();
  // A workflow off on the machine, or in the Job's repository, is not offered, and a Fleet that has not served the setting leaves every one on: `workflows.off` in settings.json.
  const off = workflowsOffOf(useSetting(WORKFLOWS_OFF_KEY));
  const [answered, setAnswered] = useState<{ id: string; choice: DeliveryChoice } | undefined>(undefined);
  // Fleet reads a custom Job's workflow as none, which is also how a Job it has not read looks, so his pick is kept here.
  const [custom, setCustom] = useState<string | undefined>(undefined);
  const start = session.start;
  if (start === undefined || draft === undefined) return undefined;

  const { id } = session;
  const proposal = session.work?.proposal;
  const reading = proposal?.reading === true;
  const picked = proposal?.picked ?? [];
  const kinds = (draft.work?.workflows ?? []).filter((one) => (one.manifestId === undefined || session.manifestId === undefined || one.manifestId === session.manifestId) && !isOff(off, one.id, session.manifestId));
  // Fleet gives the Job its workflow once it has read; the `feature`-else-first fallback is only for a Fleet that has not.
  const proposed = proposal?.workflow ?? null;
  const none = custom === id;
  const kind = reading || none ? null : (proposed ?? kinds.find((one) => one.id === "feature")?.id ?? kinds[0]?.id ?? null);
  const kindName = kinds.find((one) => one.id === kind)?.name;
  const repository = repositories.find((one) => one.manifest?.id === session.manifestId)?.manifest?.repository;
  const lands = start.asksDelivery && !none;
  // `start.picked` is Fleet's landing; Auto land is only for a Fleet that sends none.
  const landing = reading ? undefined : ((answered?.id === id ? answered.choice : undefined) ?? start.picked?.choice ?? "auto_land");

  // Marked while in use, only where Fleet took it from his usual or his words; its own `default` is no choice of his.
  const from = start.picked !== undefined && landing === start.picked.choice ? start.picked.from : undefined;
  const standing = from === "named" || from === "usual" ? from : undefined;
  const standingSaid =
    standing === "named" ? "Named in your request" : kindName === undefined || repository === undefined ? "Your usual" : `Your usual for ${plural(kindName)} in ${repository}`;

  const queued = start.queued === true;
  const begin = async (text: string): Promise<boolean> => {
    if (queued || draft.begin === undefined) return false;
    // While the proposer reads, the start is queued with no landing: Fleet starts it with the reading's once it ends.
    // The workflow shown is the one the Job runs under, so a default he did not change is recorded now.
    if (!reading && proposed === null && kind !== null) draft.work?.choose(id, kind);
    // The box goes as he left it; with no held turn Fleet takes it as the first one.
    const sent = await draft.begin(id, lands ? landing : undefined, text.trim() === "" ? undefined : text);
    return sent !== false;
  };

  // None is a custom Job: Fleet is told at once, as for any other workflow, and no landing is asked of it.
  const pick = (value: string): void => {
    setCustom(value === NO_WORKFLOW ? id : undefined);
    draft.work?.choose(id, value === NO_WORKFLOW ? null : value);
  };

  const lead = (
    <>
      {queued ? (
        <Tooltip label="Starting">
          <span role="img" aria-label="Starting" className="armada-session-composer__proposed" data-starting="">
            <LoaderCircle size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
      {reading ? (
        <Tooltip label="Reading the request">
          <span role="img" aria-label="Reading the request" className="armada-session-composer__proposed">
            <TextCursor size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
      <Pick Glyph={Workflow} compact={compact} proposed={kind !== null && kind === proposed && picked.includes("workflow")} label="Workflow">
        <Select aria-label="Workflow" value={none ? NO_WORKFLOW : (kind ?? "")} disabled={reading} onChange={(event) => pick(event.target.value)}>
          {kind === null && !none ? <option value="">Workflow</option> : null}
          <option value={NO_WORKFLOW}>None</option>
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
