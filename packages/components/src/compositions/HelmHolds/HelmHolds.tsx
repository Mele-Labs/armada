import { Check, CircleDot, GitBranch, GitPullRequest, KeyRound, Split } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PullRequestChecksMark } from "../SessionLedger/SessionLedger";
import type { LedgerEntry, LedgerKind } from "../SessionLedger/SessionLedger";

/**
 * What Helm holds once it has written something: its worktree slot and branch, the pull requests it
 * opened and the subagents it runs, as one row of chips under the dock's head. **The Session ledger's
 * entries, drawn compact**: each is the press the ledger's row is, so a chip opens what its row opens.
 * Nothing held draws nothing.
 */
const GLYPH: Partial<Record<LedgerKind, LucideIcon>> = {
  slot: KeyRound,
  branch: GitBranch,
  pull_request: GitPullRequest,
  subagent: Split,
};

export function HelmHolds({ entries }: { entries: readonly LedgerEntry[] }) {
  const held = entries.filter((one) => GLYPH[one.kind] !== undefined);
  if (held.length === 0) return null;
  return (
    <div className="armada-helm-holds" role="group" aria-label="Held by Helm">
      {held.map((one) => {
        const Glyph = GLYPH[one.kind]!;
        return (
          <Tooltip key={one.key} label={one.name} asChild>
            <button type="button" className="armada-helm-holds__chip" aria-label={`Open ${one.name}`} onClick={one.onOpen}>
              <Glyph size={12} strokeWidth={2} aria-hidden />
              <span className="armada-helm-holds__text">{one.text}</span>
              {one.pr !== undefined ? (
                <PullRequestChecksMark checks={one.pr.checks} said={one.pr.checksSaid} />
              ) : one.kind === "subagent" && one.mark !== undefined ? (
                <Tooltip label={one.mark.said}>
                  <span className="armada-session-mark" role="img" aria-label={one.mark.said} data-state-mark data-pulsing={one.mark.glyph === "running" || undefined}>
                    {one.mark.glyph === "running" ? <CircleDot size={12} strokeWidth={2} aria-hidden /> : <Check size={12} strokeWidth={2} aria-hidden />}
                  </span>
                </Tooltip>
              ) : null}
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}
