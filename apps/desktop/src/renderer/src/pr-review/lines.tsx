// What hangs under a line of the diff: a thread the forge holds, his words not yet posted, and a finding
// Armada drew from an agent review. **Three things, told apart by their frame**: the forge's is a solid card,
// his own pending words take his hue and a dashed edge, and a finding is hatched, because it is Armada's and
// goes nowhere. A state is a glyph with a name; none is a phrase.

import { useState } from "react";
import type { ReactNode } from "react";
import { Check, SquareDashed, X } from "lucide-react";
import {
  Button,
  Prose,
  PrMark,
  Textarea,
  Tooltip,
  UnifiedDiff,
  agoOf,
  forgetKept,
  sendsOn,
} from "@armada/components";

import { rangeOfDraft } from "./changes";
import type { AgentRun, Draft, Finding, Thread, ToolMark } from "./model";
import { ArmadaMark, RunFailed, SeverityMark, TOOL_MARK } from "./marks";
import { runState } from "./opening-run";
import { linesOfFix, type FixWritten } from "./runs";

// ---- The box -------------------------------------------------------------------------------------------

/**
 * The box a comment or a reply is written in. **It adds to the pending review**; nothing is posted from here.
 * Cancel leaves the words where they are, kept under `keep`, so a draft survives putting the box away.
 */
export function LineComposer({
  keep,
  label,
  onAdd,
  onCancel,
  action = "Add to review",
  initial = "",
  also,
}: {
  keep: string;
  label: string;
  onAdd: (body: string) => void;
  onCancel: () => void;
  action?: string;
  initial?: string;
  /** A second way to send the words, beside `action`: a comment posted on its own rather than kept for the review. */
  also?: {
    label: string;
    press: (body: string) => void;
  };
}) {
  const [text, setText] = useState(initial);
  const ready = text.trim() !== "";
  const take = (to: (body: string) => void) => {
    if (!ready) return;
    to(text.trim());
    forgetKept(keep);
    setText("");
  };
  const add = () => take(onAdd);
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
        {also === undefined ? null : (
          <Button variant="secondary" size="sm" disabled={!ready} onClick={() => take(also.press)}>
            {also.label}
          </Button>
        )}
        <Button variant="secondary" size="sm" type="submit" disabled={!ready}>
          {action}
        </Button>
      </div>
    </form>
  );
}

// ---- His words, not yet posted ---------------------------------------------------------------------------

export function DraftCard({ draft, onRemove }: { draft: Draft; onRemove: () => void }) {
  const range = rangeOfDraft(draft);
  return (
    <article className="pr-draft" aria-label="Pending comment" data-draft={draft.id} tabIndex={-1}>
      <header className="pr-card__head">
        <PrMark Glyph={SquareDashed} said="Pending, posted with the review" />
        <b className="pr-card__who">You</b>
        {range === undefined ? null : <span className="pr-card__lines">{range}</span>}
        <span className="pr-card__act">
          <Tooltip label="Remove from the review">
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Remove from the review"
              onClick={onRemove}
            >
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
    <article
      className="pr-thread"
      aria-label={`Thread on ${where} by ${first?.author ?? "reviewer"}`}
      data-resolved={thread.resolved || undefined}
    >
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
export function FindingCard({
  finding,
  mark,
  session,
  onDismiss,
  onSend,
  children,
  fix,
}: {
  finding: Finding;
  mark: ToolMark;
  session: boolean;
  /** Absent draws no Dismiss, for a host whose own marks put a finding away. */
  onDismiss?: () => void;
  onSend: () => void;
  /** What the host adds under the words: the Review tab's marks. */
  children?: ReactNode;
  /** Ask for a fix, and the fix once written. Absent draws none, for a finding on the base's lines. */
  fix?: FixAsk;
}) {
  const kind = TOOL_MARK[mark];
  return (
    <article
      className="pr-finding"
      aria-label={`Finding: ${finding.title}`}
      data-finding={finding.id}
    >
      <header className="pr-card__head">
        <ArmadaMark />
        <span className="pr-hued" data-tool={mark}>
          <PrMark Glyph={kind.Glyph} said={kind.said} />
        </span>
        <b className="pr-card__who">{finding.title}</b>
        <SeverityMark severity={finding.severity} />
        {onDismiss === undefined ? null : (
          <span className="pr-card__act">
            <Tooltip label="Dismiss">
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label="Dismiss finding"
                onClick={onDismiss}
              >
                <X size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        )}
      </header>
      <div className="pr-card__body">
        <Prose text={finding.body} />
      </div>
      {children}
      {fix === undefined ? null : (
        <FixSection title={finding.title} path={finding.at.path} fix={fix} />
      )}
      {session ? (
        <div className="pr-thread__foot">
          {finding.sent ? (
            <PrMark Glyph={Check} said="Sent to the Job" tone="success" />
          ) : (
            <Button
              variant="secondary"
              size="sm"
              aria-label={`Send to Job: ${finding.title}`}
              onClick={onSend}
            >
              Send to Job
            </Button>
          )}
        </div>
      ) : null}
    </article>
  );
}

// ---- A fix an agent wrote --------------------------------------------------------------------------------

/** A finding's fix as the card holds it: the run, what it wrote, and the presses on it. */
export type FixAsk = {
  run: AgentRun | undefined;
  written: FixWritten | undefined;
  /** The pull request's head, which says whether a fix is on the commit he reads. */
  head: string;
  onAsk: (again: boolean) => void;
  /** Opens the comment box on the finding's lines holding the fix. Absent where the lines are not on screen. */
  onSuggest?: (after: string) => void;
};

/**
 * Ask for a fix, and what came back: **the lines before in the diff's deletion hue and after in its addition hue**, why
 * on one line above them, and Add as suggestion under. Going is the tool buttons' glyph; failed is Fleet's reason.
 */
function FixSection({ title, path, fix }: { title: string; path: string; fix: FixAsk }) {
  const { run, Glyph } = runState(fix.run, fix.head);
  const written = run === "done" ? fix.written : undefined;
  return (
    <section className="pr-fix" aria-label={`Fix for ${title}`}>
      {run === "failed" && fix.run?.state === "failed" ? <RunFailed said={fix.run.said} /> : null}
      {written === undefined ? null : (
        <>
          <p className="pr-fix__why">{written.why}</p>
          <UnifiedDiff
            bare
            files={[
              {
                path,
                lines: linesOfFix(written),
              },
            ]}
          />
        </>
      )}
      <div className="pr-fix__bar">
        {written !== undefined ? (
          fix.onSuggest === undefined ? null : (
            <Button
              variant="secondary"
              size="sm"
              aria-label={`Add as suggestion: ${title}`}
              onClick={() => fix.onSuggest?.(written.after)}
            >
              Add as suggestion
            </Button>
          )
        ) : run === "running" ? (
          <Tooltip label="Running">
            <Button
              variant="secondary"
              size="sm"
              data-run="running"
              aria-label={`Ask for a fix, running: ${title}`}
              aria-disabled
            >
              <Glyph size={16} strokeWidth={2} aria-hidden />
              Ask for a fix
            </Button>
          </Tooltip>
        ) : run === "failed" ? (
          <Button
            variant="secondary"
            size="sm"
            aria-label={`Ask again for a fix: ${title}`}
            onClick={() => fix.onAsk(true)}
          >
            Ask again
          </Button>
        ) : (
          <Button
            variant="secondary"
            size="sm"
            aria-label={`Ask for a fix: ${title}`}
            onClick={() => fix.onAsk(false)}
          >
            Ask for a fix
          </Button>
        )}
      </div>
    </section>
  );
}
