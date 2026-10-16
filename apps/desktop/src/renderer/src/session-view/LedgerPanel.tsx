// What the Session holds. The branch and the bay are two chips in the band, since they are one fact each and drew a section each;
// the rest is the ledger as it was: pull requests, Jobs, subagents, pages, files and what waits on the person. The Field draws only the
// subagents at work, so a finished one is opened from here.

import { GitBranch, Rows3 } from "lucide-react";
import { SessionLedger, Tooltip } from "@armada/components";
import type { LedgerEntry } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { SvPanel, type Place } from "./SvPanel";

const SHOWN_ELSEWHERE = new Set(["slot", "branch"]);

export function LedgerPanel({ session, entries, place }: { session: Session; entries: readonly LedgerEntry[]; place: Place }) {
  const branch = attachmentsOf(session, "branch")[0];
  const bay = attachmentsOf(session, "slot")[0]?.slot ?? branch?.slot;
  const acts = (
    <>
      {branch === undefined ? null : (
        <Tooltip label={`Branch ${branch.name}`}>
          <span className="sv-hold" role="img" aria-label={`Branch ${branch.name}`}>
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            <code>{branch.name.split("/").at(-1)}</code>
          </span>
        </Tooltip>
      )}
      {bay === undefined ? null : (
        <Tooltip label={`Bay ${bay}`}>
          <span className="sv-hold" role="img" aria-label={`Bay ${bay}`}>
            <Rows3 size={12} strokeWidth={2} aria-hidden />
            <code>{bay}</code>
          </span>
        </Tooltip>
      )}
    </>
  );
  // The branch and the bay are in the band: with nothing else held the body is nothing, and the panel is its band.
  const rest = entries.filter((one) => !SHOWN_ELSEWHERE.has(one.kind));
  return (
    <SvPanel label="Ledger" area="ledger" place={place} acts={acts} empty={rest.length === 0}>
      {rest.length === 0 ? null : (
        <div className="sv-ledger">
          <SessionLedger entries={rest} />
        </div>
      )}
    </SvPanel>
  );
}
