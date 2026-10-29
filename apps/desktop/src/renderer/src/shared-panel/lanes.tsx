// A Job's Lanes, in the side panel: the workflow and the tickets, pull request and deploy it holds (`LaneCanvas`), with the comments on the pull request under the canvas as a monitor.
// **The panel is the app's one side panel**, so the Job's spine stays on show beside it and the same press or Esc closes it. **A mock's view**: the lanes are fixtures.
// A comment here shows that it came in and what the agent made of it; pressing it opens the pull request at that comment, where the acts on it are. A rule waiting on the person opens
// what it needs (`rule.tsx`).

import { useMemo, useState } from "react";
import { Check } from "lucide-react";
import { COMMENT_READ, LaneCanvas, Tooltip, jobLanes } from "@armada/components";
import type { LanePick, TriagedComment } from "@armada/components";

import { useSessions } from "../sessions-draft";
import { SidePanel } from "./side";
import { useBack } from "./panels";
import { closePanel, openInPanel } from "./store";
import type { PanelItem } from "./store";
import "../session-view/lanes.css";

/** One human comment, as a monitor row: what the agent made of it as a glyph and its words, who wrote it and where, the words, and a mark once it went to the Job. The row opens the comment in the pull request. */
function CommentRow({ comment, onOpen }: { comment: TriagedComment; onOpen: () => void }) {
  const { Glyph, said } = COMMENT_READ[comment.read];
  return (
    <li className="sv-lanes__item">
      <button type="button" className="sv-lanes__comment" data-read={comment.read} data-sent={comment.sent || undefined} aria-label={`Open in the pull request: ${comment.author} on ${comment.where}`} onClick={onOpen}>
        <span className="sv-lanes__read">
          <Glyph size={16} strokeWidth={2} aria-hidden />
          <span>{said}</span>
        </span>
        <span className="sv-lanes__words">
          <span className="sv-lanes__who">
            <span className="sv-lanes__author">{comment.author}</span>
            <code className="sv-lanes__where">{comment.where}</code>
          </span>
          <span className="sv-lanes__text">{comment.text}</span>
        </span>
        <span className="sv-lanes__sent">
          {comment.sent === true ? (
            <Tooltip asChild label="Sent">
              <span role="img" aria-label="Sent">
                <Check size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : null}
        </span>
      </button>
    </li>
  );
}

export function LanesPanel({ item }: { item: Extract<PanelItem, { kind: "lanes" }> }) {
  const session = useSessions().find((one) => one.id === item.sessionId);
  const back = useBack();
  const [picked, setPicked] = useState<LanePick | undefined>();
  const lanes = session?.lanes;
  const drawn = useMemo(() => (lanes === undefined ? undefined : jobLanes(lanes.job, lanes)), [lanes]);
  if (session === undefined || lanes === undefined || drawn === undefined) return null;
  const { triage } = lanes.job;
  const held = triage === undefined ? undefined : (lanes.job.instances.find((one) => one.id === triage.instance)?.label ?? triage.instance);
  const pull = session.attachments.find((one) => one.kind === "pull_request");
  const pick = (next: LanePick | undefined) => {
    setPicked(next);
    // A rule waiting on the person opens what it needs.
    if (next?.kind === "rule" && drawn.rules.some((one) => one.rule === next.id && one.needs !== undefined)) openInPanel({ kind: "rule", sessionId: session.id, rule: next.id });
  };
  return (
    <SidePanel title="Lanes" heading={session.title ?? "Lanes"} back={back} onClose={closePanel}>
      <div className="sv-lanes__body">
        <LaneCanvas label="Lanes" lanes={drawn.lanes} rules={drawn.rules} picked={picked} onPick={pick} />
        {triage === undefined ? null : (
          <section className="sv-lanes__triage" aria-label={`Comments on ${held}`}>
            <h4 className="sv-lanes__triage-head">{`Comments on ${held}`}</h4>
            <ul className="sv-lanes__comments">
              {triage.comments.map((comment) => (
                <CommentRow
                  key={comment.id}
                  comment={comment}
                  onOpen={() => pull !== undefined && openInPanel({ kind: "pr", number: pull.number, address: pull.address ?? "", title: pull.title, branch: pull.branch, tab: "conversation", comment: { author: comment.author, body: comment.text } })}
                />
              ))}
            </ul>
          </section>
        )}
      </div>
    </SidePanel>
  );
}
