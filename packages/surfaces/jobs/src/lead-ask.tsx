// What the Overview's lead held, as an ask in the Now panel. **The lead is gone from the tab** (owner,
// 11 Oct 2026: *now that we have the Now panel, the lead panel is not as useful*), and everything it
// offered to be answered moves here, in the Cockpit's own call card: the approval, the review gate and
// a plan at it, a Drone's question, a held command, a gaming check, the proposer, a Trigger's alert, the
// stopped step's recovery and the fix a failed Check is waiting on. Nothing the lead offered is dropped.
//
// A lead with nothing in it to answer or open (a Job running well, or landed) draws no card: its
// sentence was a status, and the header's badge says the same.

import { AskActs, Button, JobAlertMark, Prose } from "@armada/components";
import type { NowSlotAsk } from "@armada/components";
import type { JobAlert } from "@armada/protocol";
import { Bot, Eye, Hand, OctagonAlert, ShieldX, Stamp, Cpu } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import type { JobLead } from "./lead";

/** One thing the lead had under it, which a person answers: what it is called in the band, and its mark. */
export type LeadSlot = { band: string; icon: LucideIcon; node: ReactNode };

/** The slots in the order the lead drew them: a Drone's question, a held command, a held check, the gate, the proposer. */
export const LEAD_SLOTS = {
  question: { band: "Drone question", icon: Bot },
  command: { band: "Command held", icon: Hand },
  gaming: { band: "Check held", icon: ShieldX },
  verdict: { band: "Review", icon: Eye },
  proposer: { band: "Proposer", icon: Cpu },
} as const;

export type LeadAskReading = {
  lead: JobLead;
  /** The slots present, in order. */
  slots: readonly LeadSlot[];
  /** The acts the lead carried: the stopped step's recovery, Walk in Bridge, the one act, or the approval. */
  acts: ReactNode;
  alerts: readonly { job: string; name: string; alert: JobAlert; onOpen?: () => void }[];
  parked: readonly { job: string; title: string; onOpen?: () => void }[];
  fix?: { title: string; rest: string; onOpen?: () => void };
  /** Whether the Job's own read is still out, so the lead's sentence is a guess. */
  reading: boolean;
};

export function leadAskOf({ lead, slots, acts, alerts, parked, fix, reading }: LeadAskReading): NowSlotAsk | undefined {
  const acting = acts !== undefined && acts !== null && acts !== false;
  if (reading || (slots.length === 0 && !acting && alerts.length === 0 && parked.length === 0 && fix === undefined)) return undefined;
  const first = slots[0];
  const [band, icon] =
    lead.approves === true
      ? ["Approve dispatch", Stamp]
      : first !== undefined
        ? [first.band, first.icon]
        : alerts.length > 0 && !acting
          ? ["Trigger alert", OctagonAlert]
          : lead.tone === "completed-failed"
            ? ["Stopped", OctagonAlert]
            : ["Needs you", Hand];
  return {
    key: "lead",
    kind: "slot",
    band,
    icon,
    title: lead.said,
    ...(lead.elapsed === undefined ? {} : { age: lead.elapsed }),
    children: (
      <>
        {lead.asked === undefined ? null : (
          <div className="armada-callcard__asked">
            <Prose text={lead.asked} />
          </div>
        )}
        {lead.because === "" && fix === undefined ? null : (
          <p className="armada-callcard__origin">
            {fix === undefined ? null : (
              <>
                {fix.onOpen === undefined ? (
                  fix.title
                ) : (
                  <Button variant="ghost" size="sm" onClick={fix.onOpen}>
                    {fix.title}
                  </Button>
                )}
                {fix.rest}
                {lead.because === "" ? null : " · "}
              </>
            )}
            {lead.because}
          </p>
        )}
        {alerts.length === 0 ? null : (
          <ul className="armada-callcard__list" aria-label="Alerts">
            {alerts.map((one) => (
              <li key={one.job}>
                <JobAlertMark alert={one.alert} />
                {one.onOpen === undefined ? (
                  `${one.name}, ${one.alert.trigger}`
                ) : (
                  <Button variant="ghost" size="sm" onClick={one.onOpen}>
                    {`${one.name}, ${one.alert.trigger}`}
                  </Button>
                )}
              </li>
            ))}
          </ul>
        )}
        {parked.length === 0 ? null : (
          <>
            <p className="armada-callcard__origin">Waiting on this fix</p>
            <ul className="armada-callcard__list" aria-label="Waiting on this fix">
              {parked.map((one) => (
                <li key={one.job}>
                  {one.onOpen === undefined ? (
                    one.title
                  ) : (
                    <Button variant="ghost" size="sm" onClick={one.onOpen}>
                      {one.title}
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          </>
        )}
        {slots.map((one, at) => (
          <div key={at} className="armada-callcard__slot">
            {one.node}
          </div>
        ))}
        {acting ? <AskActs>{acts}</AskActs> : null}
      </>
    ),
  };
}
