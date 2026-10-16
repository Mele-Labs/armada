// The right column of a review: what an agent found, and what he has written so far. **An agent review runs
// only when he presses for one**, because it is expensive, and its findings stay in Armada: they are pinned in
// the reader, offered here, and go nowhere else unless he sends them to the Session that holds the pull request.
// His own words wait here as a pending review, and Submit posts them together.

import { useState } from "react";
import { Ban, Check, ScrollText, X } from "lucide-react";
import { Button, Checkbox, PrMark, Tabs, Textarea, Tooltip, agoOf, forgetKept, mayWrite } from "@armada/components";

import { useSendFindings } from "./acts";
import { FINDING } from "./marks";
import type { ReviewPr, Verdict } from "./model";
import { SHOW_FINDING } from "./reader";
import { NO_WORK, useReviewStore, useReviews } from "./store";

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

const nameOf = (path: string) => path.split("/").pop() ?? path;

export function ReviewSide() {
  const { prs, open } = useReviews();
  const pr = prs.find((one) => one.key === open);
  return (
    <>
      <AgentPanel pr={pr} />
      <YourReview key={pr?.key} pr={pr} />
    </>
  );
}

function AgentPanel({ pr }: { pr: ReviewPr | undefined }) {
  const store = useReviewStore()!;
  const { work: all } = useReviews();
  const work = pr === undefined ? NO_WORK : (all[pr.key] ?? NO_WORK);
  const send = useSendFindings(pr);
  const [picked, setPicked] = useState<ReadonlySet<string>>(new Set());
  const running = work.run.state === "running";
  const live = work.findings.filter((one) => !one.dismissed);
  const toSend = live.filter((one) => picked.has(one.id) && !one.sent);
  return (
    <section className="deck-panel pr-agent" aria-label="Agent review" tabIndex={-1}>
      <header className="deck-band">Agent review</header>
      {pr === undefined ? null : (
        <div className="pr-agent__body">
          <Tooltip label="Findings stay in Armada. Nothing is posted" asChild>
            <Button variant="secondary" pending={running} disabled={running} onClick={() => store.askAgent(pr.key)}>
              {running ? "Reviewing" : "Review with an agent"}
            </Button>
          </Tooltip>
          {live.length === 0 ? null : (
            <ul className="pr-agent__findings" aria-label="Findings">
              {live.map((one) => {
                const kind = FINDING[one.kind];
                return (
                  <li key={one.id} className="pr-find">
                    <div className="pr-find__head">
                      {send === undefined ? null : (
                        <Checkbox checked={picked.has(one.id)} disabled={one.sent} aria-label={`Pick ${one.title}`} onChange={() => setPicked((was) => (was.has(one.id) ? new Set([...was].filter((id) => id !== one.id)) : new Set(was).add(one.id)))}>
                          {null}
                        </Checkbox>
                      )}
                      <PrMark Glyph={kind.Glyph} said={kind.said} />
                      <button type="button" className="pr-find__title" onClick={() => window.dispatchEvent(new CustomEvent(SHOW_FINDING, { detail: one.id }))}>
                        {one.title}
                      </button>
                      <Tooltip label="Dismiss">
                        <Button variant="ghost" size="sm" iconOnly aria-label={`Dismiss ${one.title}`} onClick={() => store.dismiss(pr.key, one.id)}>
                          <X size={16} strokeWidth={2} aria-hidden />
                        </Button>
                      </Tooltip>
                    </div>
                    <div className="pr-find__foot">
                      <span className="pr-find__where mono">{nameOf(one.path)}</span>
                      {send === undefined ? null : one.sent ? (
                        <PrMark Glyph={Check} said="Sent to the Session" tone="success" />
                      ) : (
                        <Button variant="ghost" size="sm" aria-label={`Send to Session: ${one.title}`} onClick={() => send([one])}>
                          Send to Session
                        </Button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
          {toSend.length === 0 || send === undefined ? null : (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => {
                send(toSend);
                setPicked(new Set());
              }}
            >
              Send picked to Session
            </Button>
          )}
        </div>
      )}
    </section>
  );
}

function YourReview({ pr }: { pr: ReviewPr | undefined }) {
  const store = useReviewStore()!;
  const { work: all } = useReviews();
  const work = pr === undefined ? NO_WORK : (all[pr.key] ?? NO_WORK);
  const [kind, setKind] = useState<Verdict>("comment");
  const [text, setText] = useState("");
  const ready = pr !== undefined && (work.drafts.length > 0 || mayWrite(kind, text));
  const keep = pr === undefined ? false : `pr-review:${pr.key}:summary`;
  const submit = () => {
    if (pr === undefined || !ready) return;
    store.submit(pr.key, kind, text);
    if (keep !== false) forgetKept(keep);
    setText("");
    setKind("comment");
  };
  return (
    <section className="deck-panel pr-yours" aria-label="Your review" tabIndex={-1}>
      <header className="deck-band">Your review</header>
      {pr === undefined ? null : (
        <div className="pr-yours__body">
          {work.drafts.length === 0 ? null : (
            <ul className="pr-yours__drafts" aria-label="Pending comments">
              {work.drafts.map((one) => (
                <li key={one.id} className="pr-yours__draft">
                  <span className="pr-yours__path mono">{nameOf(one.path)}</span>
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
            <Tabs items={KINDS} value={kind} onChange={(id) => setKind(id as Verdict)} />
            <Button variant="primary" type="submit" disabled={!ready}>
              Submit review
            </Button>
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
        </div>
      )}
    </section>
  );
}
