// The pull requests waiting on him, in the column: those someone asked his review of, and those one of his
// agents opened. **Fetched when the view opens and when Refresh is pressed, and never polled.** Armada does
// not watch other people's pull requests, so the glyph on Refresh says when this list was read.

import { useEffect } from "react";
import { RotateCw } from "lucide-react";
import { Button, Tabs, Tooltip, agoOf } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { ChecksMark, ReviewMark } from "./marks";
import type { ReviewPr, ReviewTab } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";

const TABS = [
  { id: "requested", label: "Review requested" },
  { id: "mine", label: "Mine" },
];

/** The repository picked narrows the list, as it narrows every other list in the window. */
export const inRepository = (pr: ReviewPr, picked: RepositorySummary | null): boolean => picked === null || picked.manifest === undefined || pr.manifestId === picked.manifest.id;

export function ReviewList({ picked }: { picked: RepositorySummary | null }) {
  const store = useReviewStore()!;
  const { prs, tab, open, fetchedAt, fetching, work } = useReviews();
  // The list is read when the view is opened, the first time and no more.
  useEffect(() => {
    if (fetchedAt === undefined) store.fetch();
  }, [store, fetchedAt]);
  const shown = prs.filter((one) => one.tab === tab && inRepository(one, picked));
  const fetched = fetchedAt === undefined ? "Not fetched" : `Fetched ${agoOf(new Date(fetchedAt).toISOString())}`;
  return (
    <section className="deck-panel deck-list pr-list" aria-label="Pull requests" tabIndex={-1}>
      <header className="deck-band">
        Pull requests
        <span className="pr-list__refresh">
          <Tooltip label={fetching ? "Fetching" : fetched}>
            <Button variant="ghost" size="sm" iconOnly aria-label="Refresh" disabled={fetching} onClick={() => store.fetch()}>
              <RotateCw size={16} strokeWidth={2} aria-hidden data-fetching={fetching || undefined} className="pr-list__spin" />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-list__tabs">
        <Tabs items={TABS} value={tab} onChange={(id) => store.setTab(id as ReviewTab)} />
      </div>
      <ul className="deck-list__rows pr-list__rows">
        {shown.map((pr) => (
          <li key={pr.key} className="deck-list__item">
            <button type="button" className="pr-row" data-selected={open === pr.key || undefined} aria-current={open === pr.key || undefined} aria-label={`#${pr.number} ${pr.title}`} onClick={() => store.open(pr.key)}>
              <span className="pr-row__title">{pr.title}</span>
              <ReviewMark review={work[pr.key]?.review ?? NO_WORK.review} />
              <ChecksMark checks={pr.checks} />
              <span className="pr-row__facts">
                <span className="pr-row__number mono">{`#${pr.number}`}</span>
                <span className="pr-row__author">{pr.page.author}</span>
                <span className="pr-row__repository">{pr.repository}</span>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
