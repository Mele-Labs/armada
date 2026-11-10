// The Comments pane. **What sits under no line comes first**, as Conversation: the forge's general comments and reviews, and the
// threads on a whole file. Then every thread on a line, by its file and line, open ones first and the resolved folded under their
// count. **A press on a thread goes to it in Files**, at its line with the thread under it; its chevron opens it here, to read, reply
// and resolve without leaving the pane. A reply joins his pending review, as one written under the line does.

import { useState } from "react";
import { Check, ChevronDown, ChevronRight, MessageSquare } from "lucide-react";
import { Button, PrConversation, PrMark, Tooltip, readKept, useKept } from "@armada/components";

import { LineComposer, ThreadCard } from "./lines";
import type { ReviewPr, Thread } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";

/** Where a thread is: its file and line, or its file alone where it is on the file as a whole. */
export const whereOf = (thread: Thread): string =>
  thread.line === undefined ? thread.path : `${thread.path}:${thread.line}`;

/** A thread the forge holds, with Reply and Resolve, wherever it is drawn. */
export function ThreadHere({ pr, thread }: { pr: ReviewPr; thread: Thread }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const keep = `pr-review:${pr.key}:reply:${thread.id}`;
  // A reply he began and left is still in its box when he comes back.
  const [replying, setReplying] = useState(() => (readKept<string>(keep) ?? "") !== "");
  return (
    <ThreadCard
      thread={thread}
      where={whereOf(thread)}
      drafts={work.drafts.filter((one) => one.thread === thread.id)}
      replying={replying}
      onReply={() => setReplying(true)}
      onResolve={(resolved) => void store.resolve(pr.key, thread.id, resolved)}
      onRemove={(id) => store.removeDraft(pr.key, id)}
      composer={
        <LineComposer
          keep={keep}
          label="Reply"
          onAdd={(body) => {
            store.addDraft(pr.key, {
              path: thread.path,
              body,
              thread: thread.id,
            });
            setReplying(false);
          }}
          onCancel={() => setReplying(false)}
        />
      }
    />
  );
}

function ThreadRow({
  pr,
  thread,
  onGo,
  open,
  onToggle,
}: {
  pr: ReviewPr;
  thread: Thread;
  onGo: (thread: Thread) => void;
  open: boolean;
  onToggle: () => void;
}) {
  const first = thread.comments[0];
  const where = whereOf(thread);
  return (
    <li className="pr-threads__item" data-resolved={thread.resolved || undefined}>
      <div className="pr-threads__row">
        <Tooltip label={open ? "Fold the thread" : "Open the thread here"}>
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label={open ? `Fold the thread on ${where}` : `Open the thread on ${where}`}
            aria-expanded={open}
            onClick={onToggle}
          >
            {open ? (
              <ChevronDown size={16} strokeWidth={2} aria-hidden />
            ) : (
              <ChevronRight size={16} strokeWidth={2} aria-hidden />
            )}
          </Button>
        </Tooltip>
        <span className="pr-threads__mark">
          {thread.resolved ? (
            <PrMark Glyph={Check} said="Resolved" tone="success" />
          ) : (
            <PrMark Glyph={MessageSquare} said="Open" />
          )}
        </span>
        <button
          type="button"
          className="pr-threads__go"
          aria-label={`Go to ${where} in Files`}
          onClick={() => onGo(thread)}
        >
          <span className="pr-threads__where mono">{where}</span>
          {first === undefined ? null : (
            <span className="pr-threads__said">
              <b className="pr-card__who">{first.author}</b>
              <span className="pr-threads__body">{first.body}</span>
            </span>
          )}
        </button>
      </div>
      {open ? (
        <div className="pr-threads__card">
          <ThreadHere pr={pr} thread={thread} />
        </div>
      ) : null}
    </li>
  );
}

const NONE_OPEN: string[] = [];

/** The pane. `onGo` brings Files forward at a thread's line. */
export function CommentsPane({ pr, onGo }: { pr: ReviewPr; onGo: (thread: Thread) => void }) {
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const [folded, setFolded] = useState(true);
  // Which threads are open here, kept per pull request on this machine, so they are open again when he comes back.
  const [opened, setOpened] = useKept<string[]>(`pr-review:${pr.key}:threads-open`, NONE_OPEN);
  const row = (one: Thread) => (
    <ThreadRow
      key={one.id}
      pr={pr}
      thread={one}
      onGo={onGo}
      open={opened.includes(one.id)}
      onToggle={() =>
        setOpened((was) =>
          was.includes(one.id)
            ? was.filter((id) => id !== one.id)
            : [
                ...was,
                one.id,
              ],
        )
      }
    />
  );
  const onFile = work.threads.filter((one) => one.line === undefined);
  const onLine = work.threads.filter((one) => one.line !== undefined);
  const open = onLine.filter((one) => !one.resolved);
  const resolved = onLine.filter((one) => one.resolved);
  const general = pr.page.conversation.length > 0 || onFile.length > 0;
  return (
    <section className="pr-threads" aria-label="Comments">
      {!general ? null : (
        <section className="pr-threads__general" aria-label="Conversation">
          <h3 className="pr-threads__head">Conversation</h3>
          <PrConversation page={pr.page} description="" />
          {onFile.length === 0 ? null : (
            <ul className="pr-threads__list" aria-label="Threads on whole files">
              {onFile.map(row)}
            </ul>
          )}
        </section>
      )}
      {general && onLine.length > 0 ? <h3 className="pr-threads__head">On lines</h3> : null}
      <ul className="pr-threads__list" aria-label="Open threads">
        {open.map(row)}
      </ul>
      {resolved.length === 0 ? null : (
        <>
          <div className="pr-threads__fold">
            <Button
              variant="ghost"
              size="sm"
              aria-expanded={!folded}
              onClick={() => setFolded(!folded)}
            >
              {folded ? (
                <ChevronRight size={16} strokeWidth={2} aria-hidden />
              ) : (
                <ChevronDown size={16} strokeWidth={2} aria-hidden />
              )}
              Resolved
              <span className="mono">{resolved.length}</span>
            </Button>
          </div>
          {folded ? null : (
            <ul className="pr-threads__list" aria-label="Resolved threads">
              {resolved.map(row)}
            </ul>
          )}
        </>
      )}
    </section>
  );
}
