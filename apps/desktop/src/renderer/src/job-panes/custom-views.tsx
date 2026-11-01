// The views a built Custom tab draws, by the mod that made them. A tab mod is code Bridge loads as part of itself and hands the whole Fleet API,
// so each reads Fleet through `window.armada` and draws from Bridge's own components. **The mock's**: these stand for what a Mod Job would write.

import { useEffect, useState, type ComponentType } from "react";
import { Copy, ExternalLink, GitPullRequest } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { PullReviewRow } from "@armada/protocol";

import { openLink } from "../commands";
import { ChecksMark, ReviewMark } from "../pr-review/marks";
import { LedgerRow, Glyphed } from "../session-view/LedgerRow";
import { openAddressInPanel } from "../shared-panel/store";
import { useRevision } from "./custom";

/** How long ago a time was, in its largest unit. */
function waited(at: string): string {
  const minutes = Math.max(1, Math.round((Date.now() - Date.parse(at)) / 60_000));
  return minutes < 60 ? `${minutes}m` : minutes < 1440 ? `${Math.round(minutes / 60)}h` : `${Math.round(minutes / 1440)}d`;
}

/** Pull requests across the repositories that wait on a review of his, the newest change first, under the repository each is in. */
function ReviewQueue() {
  // The Mod Job's changes land one at a time: first the oldest lead, then how long each has waited, then the heading says so.
  const step = useRevision("review-queue");
  const [rows, setRows] = useState<readonly PullReviewRow[]>([]);
  useEffect(() => {
    let live = true;
    void window.armada.getReviewLists().then((read) => live && setRows(read.ok ? read.value.requested : []));
    return () => {
      live = false;
    };
  }, []);
  const repositories = [...new Set(rows.map((one) => one.repository))];
  return (
    <div className="job-pane__scroll">
      {repositories.map((repository) => (
        <section key={repository} className="sv-layer__group" aria-label={repository}>
          <h4 className="sv-layer__group-label">{step >= 3 ? `${repository} · oldest first` : repository}</h4>
          <ul className="sv-layer__rows">
            {rows
              .filter((one) => one.repository === repository)
              .sort((a, b) => (step >= 1 ? a.updated_at.localeCompare(b.updated_at) : b.updated_at.localeCompare(a.updated_at)))
              .map((pr) => (
                <LedgerRow
                  key={`${pr.manifest_id}:${pr.number}`}
                  tone={pr.draft ? "draft" : pr.checks === "failed" ? "failed" : "open"}
                  lead={<Glyphed Glyph={GitPullRequest} said={pr.draft ? "Draft" : "Open"} state={pr.draft ? "draft" : "open"} />}
                  num={`#${pr.number}`}
                  title={pr.title}
                  label={`Pull request #${pr.number} ${pr.title}`}
                  onOpen={() => void (openAddressInPanel(pr.address) || openLink(pr.address))}
                  act={
                    <>
                      <Tooltip label="Copy link">
                        <Button variant="ghost" size="sm" iconOnly aria-label={`Copy link to #${pr.number}`} onClick={() => void navigator.clipboard.writeText(pr.address)}>
                          <Copy size={16} aria-hidden="true" />
                        </Button>
                      </Tooltip>
                      <Tooltip label="Open on the forge">
                        <Button variant="ghost" size="sm" iconOnly aria-label={`Open #${pr.number} on the forge`} onClick={() => void openLink(pr.address)}>
                          <ExternalLink size={16} aria-hidden="true" />
                        </Button>
                      </Tooltip>
                    </>
                  }
                  meta={
                    <>
                      <code className="sv-ledger__sub">{pr.branch}</code>
                      <span className="sv-ledger__word">{pr.author}</span>
                      {step >= 2 ? <span className="sv-ledger__word">{waited(pr.updated_at)}</span> : null}
                      <ReviewMark review={pr.review} />
                      <ChecksMark checks={pr.checks} />
                    </>
                  }
                />
              ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** The view of each tab mod the mock knows, by the mod's folder name. A mod it has none for draws an empty body. */
export const VIEWS: Readonly<Record<string, ComponentType>> = { "review-queue": ReviewQueue };
