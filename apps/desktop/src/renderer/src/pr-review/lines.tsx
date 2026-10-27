// What hangs under a line of the diff: a thread the forge holds, his words not yet posted, and a finding
// Armada drew from an agent review. **Three things, told apart by their frame**: the forge's is a solid card,
// his own pending words take his hue and a dashed edge, and a finding is hatched, because it is Armada's and
// goes nowhere. A state is a glyph with a name; none is a phrase.

import { useState } from "react";
import type { ReactNode } from "react";
import { Check, SquareDashed, X } from "lucide-react";
import { Button, Prose, PrMark, Textarea, Tooltip, agoOf, forgetKept, sendsOn } from "@armada/components";

import type { Draft, Finding, Thread, ToolMark } from "./model";
import { ArmadaMark, SeverityMark, TOOL_MARK } from "./marks";

// ---- The box -------------------------------------------------------------------------------------------

/**
 * The box a comment or a reply is written in. **It adds to the pending review**; nothing is posted from here.
 * Cancel leaves the words where they are, kept under `keep`, so a draft survives putting the box away.
 */
export function LineComposer({ keep, label, onAdd, onCancel, action = "Add to review" }: { keep: string; label: string; onAdd: (body: string) => void; onCancel: () => void; action?: string }) {
  const [text, setText] = useState("");
  const ready = text.trim() !== "";
  const add = () => {
    if (!ready) return;
    onAdd(text.trim());
    forgetKept(keep);
    setText("");
  };
  return (
    <form
      className="pr-compose"
      aria-label={label}
      onSubmit={(event) => {
        event.preventDefault();
        add();
      }}
    >
      <Textarea
        rows={3}
        keep={keep}
        aria-label={label}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (sendsOn(event) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            add();
          }
        }}
      />
      <div className="pr-compose__bar">
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button variant="secondary" size="sm" type="submit" disabled={!ready}>
          {action}
        </Button>
      </div>
    </form>
  );
}

// ---- His words, not yet posted ---------------------------------------------------------------------------

export function DraftCard({ draft, onRemove }: { draft: Draft; onRemove: () => void }) {
  return (
    <article className="pr-draft" aria-label="Pending comment">
      <header className="pr-card__head">
        <PrMark Glyph={SquareDashed} said="Pending, posted with the review" />
        <b className="pr-card__who">You</b>
        <span className="pr-card__act">
          <Tooltip label="Remove from the review">
            <Button variant="ghost" size="sm" iconOnly aria-label="Remove from the review" onClick={onRemove}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-card__body">
        <Prose text={draft.body} />
      </div>
    </article>
  );
}

// ---- A thread the forge holds ----------------------------------------------------------------------------

export function ThreadCard({
  thread,
  where,
  drafts,
  replying,
  onReply,
  onResolve,
  onRemove,
  composer,
}: {
  thread: Thread;
  /** The change it is on, by title. */
  where: string;
  drafts: readonly Draft[];
  replying: boolean;
  onReply: () => void;
  onResolve: (resolved: boolean) => void;
  onRemove: (id: string) => void;
  composer: ReactNode;
}) {
  const first = thread.comments[0];
  return (
    <article className="pr-thread" aria-label={`Thread on ${where} by ${first?.author ?? "reviewer"}`} data-resolved={thread.resolved || undefined}>
      <header className="pr-card__head">
        {thread.resolved ? <PrMark Glyph={Check} said="Resolved" tone="success" /> : null}
        <b className="pr-card__who">{first?.author}</b>
        <span className="pr-card__act">
          <Button variant="ghost" size="sm" onClick={() => onResolve(!thread.resolved)}>
            {thread.resolved ? "Unresolve" : "Resolve"}
          </Button>
        </span>
      </header>
      {thread.resolved ? null : (
        <>
          <ul className="pr-thread__comments">
            {thread.comments.map((one) => (
              <li key={one.id} className="pr-thread__comment">
                <div className="pr-thread__by">
                  <b className="pr-card__who">{one.author}</b>
                  <span className="pr-card__ago">{agoOf(one.at)}</span>
                </div>
                <Prose text={one.body} />
              </li>
            ))}
          </ul>
          {drafts.map((one) => (
            <DraftCard key={one.id} draft={one} onRemove={() => onRemove(one.id)} />
          ))}
          {replying ? (
            composer
          ) : (
            <div className="pr-thread__foot">
              <Button variant="secondary" size="sm" onClick={onReply}>
                Reply
              </Button>
            </div>
          )}
        </>
      )}
    </article>
  );
}

// ---- Armada's own note -----------------------------------------------------------------------------------

/**
 * A finding: pinned to a change, hatched, and the agent's alone. **It is not on the forge**, which the glyph
 * says. Dismiss puts it away; Send to Session is there only where one of his Sessions holds the pull request.
 */
export function FindingCard({ finding, mark, session, onDismiss, onSend }: { finding: Finding; mark: ToolMark; session: boolean; onDismiss: () => void; onSend: () => void }) {
  const kind = TOOL_MARK[mark];
  return (
    <article className="pr-finding" aria-label={`Finding: ${finding.title}`} data-finding={finding.id}>
      <header className="pr-card__head">
        <ArmadaMark />
        <span className="pr-hued" data-tool={mark}>
          <PrMark Glyph={kind.Glyph} said={kind.said} />
        </span>
        <b className="pr-card__who">{finding.title}</b>
        <SeverityMark severity={finding.severity} />
        <span className="pr-card__act">
          <Tooltip label="Dismiss">
            <Button variant="ghost" size="sm" iconOnly aria-label="Dismiss finding" onClick={onDismiss}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-card__body">
        <Prose text={finding.body} />
      </div>
      {session ? (
        <div className="pr-thread__foot">
          {finding.sent ? (
            <PrMark Glyph={Check} said="Sent to the Job" tone="success" />
          ) : (
            <Button variant="secondary" size="sm" aria-label={`Send to Job: ${finding.title}`} onClick={onSend}>
              Send to Job
            </Button>
          )}
        </div>
      ) : null}
    </article>
  );
}
