// What the Session holds. The branch and the bay are two chips in the band, since they are one fact each and drew a section each;
// the subagents are on the Field. What is left is the ledger as it was: pull requests, Jobs, pages, files and what waits on the person.

import { GitBranch, Rows3 } from "lucide-react";
import { SessionLedger, Tooltip } from "@armada/components";
import type { LedgerEntry } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { SvPanel, type Place } from "./SvPanel";

const SHOWN_ELSEWHERE = new Set(["slot", "branch", "subagent"]);

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
  return (
    <SvPanel label="Ledger" area="ledger" place={place} acts={acts}>
      <div className="sv-ledger">
        <SessionLedger entries={entries.filter((one) => !SHOWN_ELSEWHERE.has(one.kind))} />
      </div>
    </SvPanel>
  );
}
