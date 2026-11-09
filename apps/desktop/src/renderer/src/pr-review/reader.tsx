// The reader: one pull request, its head and its facts over its panes (`panes.tsx`), the split-pane tabs a Job's are drawn in.
// Overview is the description first. Each explanation (`explain.tsx`) is a tab of its own: how it works (its changes grouped into
// themes by what they make happen, each in the order it was made with why), what it changes for him, how to test it, or why it was
// needed. An explanation names no list of files: a path is a caption on a change, and a hunk is its evidence, folded until opened.
// The Files tab is that list, every changed file's diff as the pull request panel draws them. Review, Checks, Commits and Ask are
// tabs too, and each tool's result is a tab in the tools pane.

import { useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Compass,
  ExternalLink,
  GitPullRequest,
  SquareTerminal,
} from "lucide-react";
import { Badge, Button, DiffSize, Kbd, Prose, Tooltip, agoOf } from "@armada/components";

import type { useSendFindings } from "./acts";
import { AskedOf, Explained } from "./explain";
import { REVIEW_KEYS, capOf } from "./key-table";
import { ChecksMark, ReviewMark } from "./marks";
import { Moved } from "./moved";
import type { ReviewPr } from "./model";
import { PrPanes } from "./panes";
import { NO_WORK, useReviewStore, useReviews } from "./store";

const STATE = {
  open: {
    status: "completed-success",
    said: "Open",
  },
  draft: {
    status: "not-started",
    said: "Draft",
  },
} as const;

type ReaderHosts = {
  onOpenLink: (address: string) => void;
  /** Open the Session that holds the pull request. */
  onOpenSession: (id: string) => void;
};

/** The pane. With nothing open it is the frame alone: an empty slot stays empty. */
export function ReviewReader(hosts: ReaderHosts) {
  const { prs, open } = useReviews();
  const pr = prs.find((one) => one.key === open);
  if (pr === undefined) {
    return <section className="deck-pane pr-reader" aria-label="Pull request" tabIndex={-1} />;
  }
  return <Reader key={pr.key} pr={pr} {...hosts} />;
}

function Reader({
  pr,
  onOpenLink,
  onOpenSession,
}: ReaderHosts & {
  pr: ReviewPr;
}) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const state = STATE[pr.draft ? "draft" : "open"];
  const opened = agoOf(pr.page.created_at);
  // Where the panes' Layout control stands: the head, once it is drawn.
  const [layout, setLayout] = useState<HTMLSpanElement | null>(null);

  return (
    <section
      className="deck-pane pr-reader"
      aria-label={`Pull request #${pr.number}`}
      tabIndex={-1}
    >
      <header className="deck-band deck-pane__head">
        <Tooltip label="Pull request">
          <span className="deck-pane__kind" role="img" aria-label="Pull request">
            <GitPullRequest size={16} aria-hidden="true" />
          </span>
        </Tooltip>
        <h2 className="deck-pane__title">{`#${pr.number} ${pr.title}`}</h2>
        <span className="deck-pane__where">{pr.repository}</span>
        <span className="deck-pane__acts">
          <span className="pr-reader__layout" ref={setLayout} />
          <Tooltip label="Go to the pull request" shortcut={REVIEW_KEYS.reader}>
            <Kbd>{REVIEW_KEYS.reader}</Kbd>
          </Tooltip>
          <Tooltip label="Where changes run" shortcut={capOf(REVIEW_KEYS.where)}>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Where changes run"
              onClick={() => store.showPlaces()}
            >
              <Compass size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          {pr.holder === undefined ? null : (
            <Tooltip label="Held by this Job">
              <Button
                variant="ghost"
                size="sm"
                aria-label={`Job: ${pr.holder.title}`}
                onClick={() => onOpenSession(pr.holder!.id)}
              >
                <SquareTerminal size={16} strokeWidth={2} aria-hidden />
                {pr.holder.title}
              </Button>
            </Tooltip>
          )}
          <Tooltip label="Open on the forge">
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Open on the forge"
              onClick={() => onOpenLink(pr.address)}
            >
              <ExternalLink size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-reader__meta">
        <Badge status={state.status} icon={GitPullRequest}>
          {state.said}
        </Badge>
        <Tooltip label="Branch and the branch it merges into">
          <span className="pr-reader__branch mono">
            {pr.page.base === "" ? pr.branch : `${pr.branch} → ${pr.page.base}`}
          </span>
        </Tooltip>
        <Tooltip label="Opened by">
          <span className="pr-reader__who">{pr.page.author}</span>
        </Tooltip>
        {opened === undefined ? null : (
          <Tooltip label="Opened">
            <span className="pr-reader__who">{opened}</span>
          </Tooltip>
        )}
        <DiffSize additions={pr.page.additions} deletions={pr.page.deletions} />
        <ReviewMark review={work.review} />
        <ChecksMark checks={pr.checks} />
      </div>
      {work.page.state === "said" ? (
        <p className="pr-reader__said" role="alert">
          {work.page.said}
        </p>
      ) : null}
      <Moved pr={pr} />
      <div className="pr-reader__panes">
        <PrPanes pr={pr} onOpenLink={onOpenLink} controls={layout} />
      </div>
    </section>
  );
}

/** `explanations` false leaves the explanations to tabs of their own: what is drawn under the description is an answer to his question alone. */
export function Overview({
  pr,
  send,
  onOpenLink,
  explanations = true,
}: {
  pr: ReviewPr;
  send: ReturnType<typeof useSendFindings>;
  onOpenLink: (address: string) => void;
  explanations?: boolean;
}) {
  const work = useReviews().work[pr.key] ?? NO_WORK;
  // **The description tucks away once an explanation or an answer is shown**, into one row that opens it again. With nothing chosen it is open.
  const shown = explanations
    ? work.kind !== undefined
    : work.asks.some((one) => one.id === work.kind);
  const [chosen, setChosen] = useState<boolean | undefined>(undefined);
  const open = chosen ?? !shown;
  return (
    <div className="pr-explain">
      <AskedOf pr={pr} />
      <section className="pr-what pr-desc" aria-label="Description" data-open={open || undefined}>
        <header className="pr-what__head">
          <Tooltip
            label={open ? "Hide the description" : "Show the description"}
            shortcut={capOf(REVIEW_KEYS.description)}
          >
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label={open ? "Hide the description" : "Show the description"}
              aria-expanded={open}
              onClick={() => setChosen(!open)}
            >
              {open ? (
                <ChevronDown size={16} strokeWidth={2} aria-hidden />
              ) : (
                <ChevronRight size={16} strokeWidth={2} aria-hidden />
              )}
            </Button>
          </Tooltip>
          <h3 className="pr-what__title">Description</h3>
        </header>
        {open ? <Prose text={pr.body} /> : null}
      </section>
      <Explained pr={pr} send={send} onOpenLink={onOpenLink} explanations={explanations} />
    </div>
  );
}
