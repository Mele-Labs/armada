// The Comments pane, one kind of thing under each heading. **Conversation** is the forge's general comments and review notes, a
// card each, and the box that posts a new one at once: the forge holds no thread on them, so nothing there takes a reply. **On
// files** is the threads on a whole file, and **On lines** every thread on a line, open ones first and the resolved folded under
// their count. A press on a thread goes to it in Files; its chevron opens it here to read, and Reply opens it with its box. A reply
// joins his pending review, as one written under the line does.

import { useState } from "react";
import { Check, ChevronDown, ChevronRight, MessageSquare } from "lucide-react";
import {
  Button,
  PrConversation,
  PrMark,
  Textarea,
  Tooltip,
  forgetKept,
  readKept,
  sendsOn,
  useKept,
} from "@armada/components";

import { LineComposer, ThreadCard } from "./lines";
import type { ReviewPr, Thread } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";

/** Where a thread is: its file and line, or its file alone where it is on the file as a whole. */
export const whereOf = (thread: Thread): string =>
  thread.line === undefined ? thread.path : `${thread.path}:${thread.line}`;

const replyKeep = (pr: ReviewPr, thread: Thread) => `pr-review:${pr.key}:reply:${thread.id}`;

/** A reply he began and left is still in its box when he comes back. */
const replyBegun = (pr: ReviewPr, thread: Thread) =>
  (readKept<string>(replyKeep(pr, thread)) ?? "") !== "";

/**
 * A thread the forge holds, with Reply and Resolve, wherever it is drawn. `replying` and `onReplying` hand whether its box is
 * open to the host, so a Reply outside the card can open it.
 */
export function ThreadHere({
  pr,
  thread,
  replying: held,
  onReplying,
}: {
  pr: ReviewPr;
  thread: Thread;
  replying?: boolean;
  onReplying?: (on: boolean) => void;
}) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const keep = replyKeep(pr, thread);
  const [own, setOwn] = useState(() => replyBegun(pr, thread));
  const replying = held ?? own;
  const setReplying = onReplying ?? setOwn;
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
  const [replying, setReplying] = useState(() => replyBegun(pr, thread));
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
        {/* A resolved thread takes no reply, and keeps the column empty so the rows still line up. */}
        <span className="pr-threads__reply">
          {thread.resolved ? null : (
            <Button
              variant="secondary"
              size="sm"
              aria-label={`Reply on ${where}`}
              onClick={() => {
                setReplying(true);
                if (!open) onToggle();
              }}
            >
              Reply
            </Button>
          )}
        </span>
      </div>
      {open ? (
        <div className="pr-threads__card">
          <ThreadHere pr={pr} thread={thread} replying={replying} onReplying={setReplying} />
        </div>
      ) : null}
    </li>
  );
}

/**
 * The box for a general comment. **Comment posts it at once**, apart from his pending review; the words stay in the box until the
 * forge takes them, and a refusal says the forge's sentence under it.
 */
function CommentBox({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const keep = `pr-review:${pr.key}:comment`;
  const [text, setText] = useState("");
  const [posting, setPosting] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const ready = text.trim() !== "" && !posting;
  const post = async () => {
    if (!ready) return;
    setPosting(true);
    setRefused(undefined);
    const answer = await store.comment(pr.key, text);
    setPosting(false);
    if (!answer.ok) {
      setRefused(answer.said);
      return;
    }
    forgetKept(keep);
    setText("");
  };
  return (
    <form
      className="pr-threads__compose"
      aria-label="Comment"
      onSubmit={(event) => {
        event.preventDefault();
        void post();
      }}
    >
      <Textarea
        rows={3}
        keep={keep}
        aria-label="Comment"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (sendsOn(event) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            void post();
          }
        }}
      />
      {refused === undefined ? null : (
        <p className="pr-threads__refused" role="alert">
          {refused}
        </p>
      )}
      <div className="pr-compose__bar">
        <Button variant="secondary" size="sm" type="submit" disabled={!ready}>
          Comment
        </Button>
      </div>
    </form>
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
  return (
    <section className="pr-threads" aria-label="Comments">
      <section className="pr-threads__general" aria-label="Conversation">
        <h3 className="pr-threads__head">Conversation</h3>
        <PrConversation page={pr.page} description="" />
        <CommentBox pr={pr} />
      </section>
      {onFile.length === 0 ? null : (
        <section className="pr-threads__general" aria-label="On files">
          <h3 className="pr-threads__head">On files</h3>
          <ul className="pr-threads__list" aria-label="Threads on whole files">
            {onFile.map(row)}
          </ul>
        </section>
      )}
      {onLine.length === 0 ? null : (
        <section className="pr-threads__general" aria-label="On lines">
          <h3 className="pr-threads__head">On lines</h3>
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
      )}
    </section>
  );
}
