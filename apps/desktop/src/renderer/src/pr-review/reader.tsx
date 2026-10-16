// The reader: one pull request, whole. Every file's diff stacked in one scroll with a tree to jump by, a Viewed
// mark on each file that folds it, and the forge's threads, his pending words and Armada's findings hung on
// their lines. Conversation, Checks and Commits are tabs of the same pane, the pull request panel's own lists.
// **Reviewing and replying are one loop**: the box to answer a thread opens under the line it is about.

import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Check, ExternalLink, GitPullRequest, MessageSquare, SquareTerminal } from "lucide-react";
import { Badge, Button, Checkbox, DiffSize, PrChecks, PrCommits, PrConversation, PrMark, Tabs, Tooltip, UnifiedDiff, agoOf } from "@armada/components";
import type { DiffFile } from "@armada/components";
import type { PullRequestEntry } from "@armada/protocol";

import { useSendFindings } from "./acts";
import { DraftCard, FindingCard, LineComposer, ThreadCard } from "./lines";
import { ChecksMark, ReviewMark } from "./marks";
import type { Anchor, ReviewFile, ReviewPr } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";

/** Asks the reader to bring a finding into view, from the column beside it. */
export const SHOW_FINDING = "pr-review:show-finding";

const TABS = [
  { id: "files", label: "Files" },
  { id: "conversation", label: "Conversation" },
  { id: "checks", label: "Checks" },
  { id: "commits", label: "Commits" },
];

const STATE = {
  open: { status: "completed-success", said: "Open" },
  draft: { status: "not-started", said: "Draft" },
} as const;

const VERDICT = { comment: "commented", approve: "approved", request_changes: "changes_requested" } as const;

type ReaderHosts = {
  onOpenLink: (address: string) => void;
  /** Open the Session that holds the pull request. */
  onOpenSession: (id: string) => void;
};

/** The pane. With nothing open it is the head alone: an empty slot stays empty. */
export function ReviewReader(hosts: ReaderHosts) {
  const { prs, open } = useReviews();
  const pr = prs.find((one) => one.key === open);
  if (pr === undefined) {
    return (
      <section className="deck-pane pr-reader" aria-label="Pull request">
        <header className="deck-band deck-pane__head">
          <Tooltip label="Pull request">
            <span className="deck-pane__kind" role="img" aria-label="Pull request">
              <GitPullRequest size={16} aria-hidden="true" />
            </span>
          </Tooltip>
        </header>
      </section>
    );
  }
  return <Reader key={pr.key} pr={pr} {...hosts} />;
}

function Reader({ pr, onOpenLink, onOpenSession }: ReaderHosts & { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const { work: all } = useReviews();
  const work = all[pr.key] ?? NO_WORK;
  const [tab, setTab] = useState("files");
  const [composing, setComposing] = useState<(Anchor & { thread?: string }) | undefined>(undefined);
  const scroller = useRef<HTMLDivElement>(null);
  const send = useSendFindings(pr);
  const state = STATE[pr.draft ? "draft" : "open"];
  const opened = agoOf(pr.page.created_at);

  // A finding pressed in the column beside is brought into view, on the Files tab.
  useEffect(() => {
    const show = (event: Event) => {
      const id = (event as CustomEvent<string>).detail;
      setTab("files");
      window.requestAnimationFrame(() => scroller.current?.querySelector(`[data-finding="${CSS.escape(id)}"]`)?.scrollIntoView?.({ block: "center" }));
    };
    window.addEventListener(SHOW_FINDING, show);
    return () => window.removeEventListener(SHOW_FINDING, show);
  }, []);

  const jump = (path: string) => scroller.current?.querySelector(`[data-diff-path="${CSS.escape(path)}"]`)?.scrollIntoView?.({ block: "start" });

  const files: DiffFile[] = pr.files.map((file) => ({
    path: file.path,
    lines: file.lines,
    meta: <DiffSize additions={file.additions} deletions={file.deletions} />,
    folded: work.viewed.includes(file.path),
    actions: (
      <Checkbox checked={work.viewed.includes(file.path)} aria-label={`Viewed ${file.path}`} onChange={() => store.toggleViewed(pr.key, file.path)}>
        Viewed
      </Checkbox>
    ),
  }));

  const under = (file: DiffFile, index: number): ReactNode => {
    const here = (one: Anchor) => one.path === file.path && one.line === index;
    const threads = work.threads.filter(here);
    const drafts = work.drafts.filter((one) => here(one) && one.thread === undefined);
    const findings = work.findings.filter((one) => here(one) && !one.dismissed);
    const writing = composing !== undefined && here(composing) && composing.thread === undefined;
    if (threads.length + drafts.length + findings.length === 0 && !writing) return null;
    return (
      <div className="pr-under">
        {threads.map((thread) => (
          <ThreadCard
            key={thread.id}
            thread={thread}
            drafts={work.drafts.filter((one) => one.thread === thread.id)}
            replying={composing?.thread === thread.id}
            onReply={() => setComposing({ path: thread.path, line: thread.line, thread: thread.id })}
            onResolve={(resolved) => store.resolve(pr.key, thread.id, resolved)}
            onRemove={(id) => store.removeDraft(pr.key, id)}
            composer={
              <LineComposer
                keep={`pr-review:${pr.key}:reply:${thread.id}`}
                label="Reply"
                onAdd={(body) => {
                  store.addDraft(pr.key, thread, body, thread.id);
                  setComposing(undefined);
                }}
                onCancel={() => setComposing(undefined)}
              />
            }
          />
        ))}
        {drafts.map((draft) => (
          <DraftCard key={draft.id} draft={draft} onRemove={() => store.removeDraft(pr.key, draft.id)} />
        ))}
        {findings.map((finding) => (
          <FindingCard key={finding.id} finding={finding} session={send !== undefined} onDismiss={() => store.dismiss(pr.key, finding.id)} onSend={() => send?.([finding])} />
        ))}
        {writing ? (
          <LineComposer
            keep={`pr-review:${pr.key}:${file.path}:${index}`}
            label="Comment"
            onAdd={(body) => {
              store.addDraft(pr.key, { path: file.path, line: index }, body);
              setComposing(undefined);
            }}
            onCancel={() => setComposing(undefined)}
          />
        ) : null}
      </div>
    );
  };

  const conversation: PullRequestEntry[] = useMemo(
    () => [
      ...pr.page.conversation,
      ...work.submissions.map((one): PullRequestEntry => ({ kind: "review", author: "nick", at: one.at, verdict: VERDICT[one.verdict], body: one.body })),
    ],
    [pr.page.conversation, work.submissions],
  );

  return (
    <section className="deck-pane pr-reader" aria-label={`Pull request #${pr.number}`}>
      <header className="deck-band deck-pane__head">
        <Tooltip label="Pull request">
          <span className="deck-pane__kind" role="img" aria-label="Pull request">
            <GitPullRequest size={16} aria-hidden="true" />
          </span>
        </Tooltip>
        <h2 className="deck-pane__title">{`#${pr.number} ${pr.title}`}</h2>
        <span className="deck-pane__where">{pr.repository}</span>
        <span className="deck-pane__acts">
          {pr.holder === undefined ? null : (
            <Tooltip label="Held by this Session">
              <Button variant="ghost" size="sm" aria-label={`Session: ${pr.holder.title}`} onClick={() => onOpenSession(pr.holder!.id)}>
                <SquareTerminal size={16} strokeWidth={2} aria-hidden />
                {pr.holder.title}
              </Button>
            </Tooltip>
          )}
          <Tooltip label="Open on the forge">
            <Button variant="ghost" size="sm" iconOnly aria-label="Open on the forge" onClick={() => onOpenLink(pr.address)}>
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
          <span className="pr-reader__branch mono">{`${pr.branch} → ${pr.page.base}`}</span>
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
      <div className="pr-reader__tabs">
        <Tabs items={TABS} value={tab} onChange={setTab} />
      </div>
      {tab === "files" ? (
        <div className="pr-reader__files">
          <nav className="pr-tree" aria-label="File tree">
            <ul className="pr-tree__rows">
              {pr.files.map((file) => (
                <TreeRow key={file.path} file={file} viewed={work.viewed.includes(file.path)} marked={work.threads.some((one) => one.path === file.path && !one.resolved) || work.findings.some((one) => one.path === file.path && !one.dismissed)} onJump={() => jump(file.path)} />
              ))}
            </ul>
          </nav>
          <div className="pr-reader__scroll" ref={scroller}>
            <UnifiedDiff
              files={files}
              under={under}
              onLine={{
                label: "Comment on this line",
                name: (file, index) => `Comment on ${file.lines[index]?.text.trim() ?? "this line"}`,
                press: (file, index) => setComposing({ path: file.path, line: index }),
              }}
            />
          </div>
        </div>
      ) : (
        <div className="pr-reader__page">
          {tab === "conversation" ? <PrConversation page={{ ...pr.page, conversation }} description={pr.body} /> : null}
          {tab === "checks" ? <PrChecks rows={pr.page.checks} onOpenLink={onOpenLink} /> : null}
          {tab === "commits" ? <PrCommits rows={pr.page.commits} /> : null}
        </div>
      )}
    </section>
  );
}

function TreeRow({ file, viewed, marked, onJump }: { file: ReviewFile; viewed: boolean; marked: boolean; onJump: () => void }) {
  const name = file.path.split("/").pop() ?? file.path;
  const dir = file.path.slice(0, file.path.length - name.length);
  return (
    <li>
      <Tooltip label={file.path} asChild>
        <button type="button" className="pr-tree__row" data-viewed={viewed || undefined} aria-label={file.path} onClick={onJump}>
          <span className="pr-tree__viewed">{viewed ? <PrMark Glyph={Check} said="Viewed" tone="success" /> : null}</span>
          <span className="pr-tree__name">
            <span className="pr-tree__file">{name}</span>
            <span className="pr-tree__dir">{dir}</span>
          </span>
          <span className="pr-tree__open">{marked ? <PrMark Glyph={MessageSquare} said="Open threads or findings" /> : null}</span>
        </button>
      </Tooltip>
    </li>
  );
}
