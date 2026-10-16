// The Comments tool: the forge's conversation, every change with the threads and line comments on it, and his
// pending review with Submit. **It runs no agent**, since all of it is the forge's or his. Words he writes join the
// pending review and leave only with Submit, which records Comment, Approve or Request changes. Nothing is posted in the mock.

import { useMemo, useState } from "react";
import { Ban, Check, ScrollText, X } from "lucide-react";
import { Button, PrConversation, PrMark, Tabs, Textarea, Tooltip, agoOf, forgetKept, mayWrite } from "@armada/components";
import type { PullRequestEntry } from "@armada/protocol";

import type { useSendFindings } from "./acts";
import type { ReviewPr, Verdict } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { ThemeBlock } from "./themes";

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

const WIRE = { comment: "commented", approve: "approved", request_changes: "changes_requested" } as const;

export function CommentsView({ pr, send }: { pr: ReviewPr; send: ReturnType<typeof useSendFindings> }) {
  const { work: all } = useReviews();
  const work = all[pr.key] ?? NO_WORK;
  const conversation: PullRequestEntry[] = useMemo(
    () => [...pr.page.conversation, ...work.submissions.map((one): PullRequestEntry => ({ kind: "review", author: "nick", at: one.at, verdict: WIRE[one.verdict], body: one.body }))],
    [pr.page.conversation, work.submissions],
  );
  return (
    <div className="pr-comments">
      <YourReview pr={pr} />
      <PrConversation page={{ ...pr.page, conversation }} description={pr.body} />
      {pr.explanation.themes.map((theme, at) => (
        <ThemeBlock key={theme.id} pr={pr} theme={theme} number={at + 1} mode="comments" send={send} />
      ))}
    </div>
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
  const where = (change: string) => pr.explanation.themes.flatMap((one) => one.changes).find((one) => one.id === change)?.title ?? "";
  const submit = () => {
    if (!ready) return;
    store.submit(pr.key, kind, text);
    forgetKept(keep);
    setText("");
    setKind("comment");
  };
  return (
    <section className="pr-yours" aria-label="Your review">
      <h3 className="pr-yours__head">Your review</h3>
      {work.drafts.length === 0 ? null : (
        <ul className="pr-yours__drafts" aria-label="Pending comments">
          {work.drafts.map((one) => (
            <li key={one.id} className="pr-yours__draft">
              <span className="pr-yours__path">{where(one.change)}</span>
              <span className="pr-yours__text">{one.body}</span>
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
          <Button variant="primary" type="submit" disabled={!ready}>
            Submit review
          </Button>
        </div>
      </form>
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
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
