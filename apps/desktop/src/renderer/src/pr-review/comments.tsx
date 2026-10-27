// The Comments tool: the forge's conversation, every change with the threads and line comments on it, and his
// pending review with Submit. **It runs no agent**, since all of it is the forge's or his. Words he writes join the
// pending review, which is kept here and leaves only with Submit: Comment, Approve or Request changes, posted as one
// review with its line comments. A refusal keeps every draft and says the forge's words.

import { useState } from "react";
import { Ban, Check, ScrollText, SquareTerminal, X } from "lucide-react";
import { Button, PrConversation, PrMark, Tabs, Textarea, Tooltip, agoOf, forgetKept, mayWrite } from "@armada/components";

import type { useSendFindings } from "./acts";
import { useSessionsDraft } from "../sessions-draft";
import { changesOf, ownerOf, placeOfDraft } from "./changes";
import type { Draft, ReviewPr, Verdict } from "./model";
import { indexOfPlace } from "./patch";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { BareChanges, ThemeBlock } from "./themes";
import { isExplained } from "./tools";

const KINDS = [
  { id: "comment", label: "Comment" },
  { id: "approve", label: "Approve" },
  { id: "request_changes", label: "Request changes" },
];

const VERDICT = {
  comment: { Glyph: ScrollText, said: "Commented" },
  approve: { Glyph: Check, said: "Approved", tone: "success" },
  request_changes: { Glyph: Ban, said: "Changes requested", tone: "failed" },
} as const;

export function CommentsView({ pr, send }: { pr: ReviewPr; send: ReturnType<typeof useSendFindings> }) {
  const { work: all } = useReviews();
  const work = all[pr.key] ?? NO_WORK;
  return (
    <div className="pr-comments">
      <YourReview pr={pr} />
      <PrConversation page={pr.page} description={pr.body} />
      {isExplained(work) ? (
        pr.explanation.themes.map((theme, at) => <ThemeBlock key={theme.id} pr={pr} theme={theme} number={at + 1} mode="comments" send={send} />)
      ) : (
        <BareChanges pr={pr} mode="comments" send={send} />
      )}
    </div>
  );
}

/**
 * One comment waiting on Submit: the file and line it is on, with that line's code, and his words. **A press goes to it in the
 * diff**, opening the change again where its diff was put away.
 */
function PendingComment({ pr, draft }: { pr: ReviewPr; draft: Draft }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const place = placeOfDraft(draft);
  const where = place === undefined ? draft.path : `${draft.path}:${place.line}`;
  const patch = work.patches[draft.path];
  const lines = patch?.state === "read" ? patch.lines : [];
  const code = place === undefined ? undefined : lines[indexOfPlace(lines, place)]?.text;
  const card = () => document.querySelector<HTMLElement>(`.pr-reader [data-draft="${draft.id}"]`);
  const goTo = () => {
    if (card() === null) {
      const owner = ownerOf(changesOf(pr, work), draft.path, place);
      if (owner !== undefined) store.toggleOpen(pr.key, owner.id);
    }
    window.requestAnimationFrame(() => {
      card()?.scrollIntoView({ block: "center" });
      card()?.focus();
    });
  };
  return (
    <button type="button" className="pr-yours__go" aria-label={`Go to your comment on ${where}`} onClick={goTo}>
      <span className="pr-yours__where">
        <span className="pr-yours__path">{where}</span>
        {code === undefined ? null : <code className="pr-yours__code">{code}</code>}
      </span>
      <span className="pr-yours__text">{draft.body}</span>
    </button>
  );
}

function YourReview({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const { work: all } = useReviews();
  const work = all[pr.key] ?? NO_WORK;
  const [kind, setKind] = useState<Verdict>("comment");
  const [text, setText] = useState("");
  const ready = work.drafts.length > 0 || mayWrite(kind, text);
  const keep = `pr-review:${pr.key}:summary`;
  const draft = useSessionsDraft();
  const holder = pr.holder;
  const submit = () => {
    if (!ready || work.submitting) return;
    // **Request changes on a pull request one of his Sessions holds goes to that Session too**, with the review, so the Drone has it.
    const told = kind === "request_changes" && holder !== undefined && draft !== undefined;
    const sent = work.drafts.map((one) => `${one.path}: ${one.body}`);
    void store.submit(pr.key, kind, text, told ? holder.title : undefined).then((took) => {
      if (!took) return;
      if (told) void draft.send(holder.id, { text: [`Changes requested on #${pr.number}`, text.trim(), ...sent].filter((one) => one !== "").join("\n\n"), files: [], sketches: [], tags: [] });
      forgetKept(keep);
      setText("");
      setKind("comment");
    });
  };
  return (
    <section className="pr-yours" aria-label="Your review">
      <h3 className="pr-yours__head">Your review</h3>
      {work.drafts.length === 0 ? null : (
        <ul className="pr-yours__drafts" aria-label="Pending comments">
          {work.drafts.map((one) => (
            <li key={one.id} className="pr-yours__draft">
              <PendingComment pr={pr} draft={one} />
              <Tooltip label="Remove from the review">
                <Button variant="ghost" size="sm" iconOnly aria-label="Remove from the review" onClick={() => store.removeDraft(pr.key, one.id)}>
                  <X size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </li>
          ))}
        </ul>
      )}
      <form
        className="pr-yours__form"
        aria-label="Review box"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <Textarea rows={3} keep={keep} aria-label="Summary" value={text} onChange={(event) => setText(event.target.value)} />
        <div className="pr-yours__bar">
          <Tabs items={KINDS} value={kind} onChange={(id) => setKind(id as Verdict)} />
          <Button variant="primary" type="submit" disabled={!ready || work.submitting}>
            Submit review
          </Button>
        </div>
      </form>
      {work.refused === undefined ? null : (
        <p className="pr-yours__refused" role="alert">
          {work.refused}
        </p>
      )}
      {work.submissions.length === 0 ? null : (
        <ul className="pr-yours__sent" aria-label="Submitted reviews">
          {work.submissions.map((one) => {
            const verdict = VERDICT[one.verdict];
            return (
              <li key={one.at} className="pr-yours__done">
                <PrMark Glyph={verdict.Glyph} said={verdict.said} {...("tone" in verdict ? { tone: verdict.tone } : {})} />
                <Tooltip label="Submitted">
                  <span className="pr-yours__ago">{agoOf(one.at)}</span>
                </Tooltip>
                {one.told === undefined ? null : (
                  <span className="pr-yours__told" role="note" aria-label={`Sent to ${one.told}`}>
                    <PrMark Glyph={SquareTerminal} said="Sent to the Job" />
                    {`Sent to ${one.told}`}
                  </span>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
