// The Job's Lanes, as a layer over its stage: the workflow as the top lane and a lane under it for each ticket, pull request and deploy the Job holds (`LaneCanvas`),
// with the comments on the pull request under the canvas while it is in review. It takes the stage and not the spine, so the strip stays on show and the same press or
// Esc closes it, as the Field large does. **A mock's view**: the lanes are fixtures, and Send to Job marks a comment sent through the draft and reaches no agent.
// Keys: the canvas's own (arrows step between nodes, R and Shift R go round the rules, Esc lets go of the pick); a second Esc is Back. Tab leaves the canvas for the comments.

import { useEffect, useMemo, useRef, useState } from "react";
import { Check, ExternalLink, MessageSquareDiff, MessageSquareOff, MessageSquareReply, Send, X, type LucideIcon } from "lucide-react";
import { Button, LaneCanvas, Tooltip, jobLanes, type CommentRead, type LanePick, type SessionLanes, type TriagedComment } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { useSessionsDraft } from "../sessions-draft";
import { closeLayer } from "./layers";
import "./lanes.css";

/** What the agent made of a comment, as a glyph and the words its tooltip reads. */
const READ: Record<CommentRead, { Glyph: LucideIcon; said: string }> = {
  change: { Glyph: MessageSquareDiff, said: "Needs a change" },
  answer: { Glyph: MessageSquareReply, said: "Answer only" },
  skip: { Glyph: MessageSquareOff, said: "Not worth acting on" },
};

/**
 * One human comment: the agent's read, who wrote it and where, the words, and the two acts in columns of their own at the trailing edge. A comment that was sent keeps its row
 * and its place, and a mark in a column of its own says so. Open in PR leaves the row as it is: the mock has no pull request to go to.
 */
function CommentRow({ comment, send }: { comment: TriagedComment; send: (() => void) | undefined }) {
  const { Glyph, said } = READ[comment.read];
  const who = `${comment.author} on ${comment.where}`;
  return (
    <li className="sv-lanes__comment" data-read={comment.read} data-sent={comment.sent || undefined}>
      <Tooltip asChild label={said}>
        <span className="sv-lanes__read" role="img" aria-label={said}>
          <Glyph size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <div className="sv-lanes__words">
        <div className="sv-lanes__who">
          <span className="sv-lanes__author">{comment.author}</span>
          <code className="sv-lanes__where">{comment.where}</code>
        </div>
        <p className="sv-lanes__text">{comment.text}</p>
      </div>
      <span className="sv-lanes__sent">
        {comment.sent === true ? (
          <Tooltip asChild label="Sent">
            <span role="img" aria-label="Sent">
              <Check size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : null}
      </span>
      <Tooltip label="Open in PR">
        <Button type="button" variant="ghost" size="sm" iconOnly aria-label={`Open in PR: ${who}`}>
          <ExternalLink size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
      {send === undefined ? (
        <span />
      ) : (
        <Tooltip label="Send to Job">
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={`Send to Job: ${who}`} onClick={comment.sent === true ? undefined : send}>
            <Send size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
    </li>
  );
}

export function LanesLayer({ session, lanes }: { session: Session; lanes: SessionLanes }) {
  const body = useRef<HTMLDivElement>(null);
  const send = useSessionsDraft()?.sendComment;
  const [picked, setPicked] = useState<LanePick | undefined>();
  const drawn = useMemo(() => jobLanes(lanes.job, lanes), [lanes]);
  const { triage } = lanes.job;
  const held = triage === undefined ? undefined : (lanes.job.instances.find((one) => one.id === triage.instance)?.label ?? triage.instance);

  // Esc is Back, wherever focus is, unless a sheet or a menu is open over the layer, or the canvas took it to let go of a pick.
  useEffect(() => {
    const listen = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null || popupIsOpen()) return;
      event.preventDefault();
      closeLayer();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);

  // Focus goes to the canvas's current node. React Flow places its nodes a frame or two after the layer mounts, so the press waits for one it can reach.
  useEffect(() => {
    let frame = 0;
    let tries = 0;
    const enter = () => {
      const node = body.current?.querySelector<HTMLElement>('[data-node][tabindex="0"]');
      node?.focus({ preventScroll: true });
      if (node !== null && node !== undefined && document.activeElement === node) return;
      tries += 1;
      if (tries < 30) frame = window.requestAnimationFrame(enter);
    };
    enter();
    return () => window.cancelAnimationFrame(frame);
  }, []);

  return (
    <section className="sv-layer" aria-label="Lanes" data-keeps-escape="" data-owns-keys="r">
      <header className="sv-layer__head">
        <h3 className="sv-layer__title">Lanes</h3>
        <Tooltip label="Back" shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={closeLayer}>
            <X size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      </header>
      <div ref={body} className="sv-layer__body sv-lanes__body">
        <LaneCanvas label="Lanes" lanes={drawn.lanes} rules={drawn.rules} picked={picked} onPick={setPicked} />
        {triage === undefined ? null : (
          <section className="sv-lanes__triage" aria-label={`Comments on ${held}`}>
            <h4 className="sv-lanes__triage-head">{`Comments on ${held}`}</h4>
            <ul className="sv-lanes__comments">
              {triage.comments.map((comment) => (
                <CommentRow key={comment.id} comment={comment} send={send === undefined ? undefined : () => send(session.id, comment.id)} />
              ))}
            </ul>
          </section>
        )}
      </div>
    </section>
  );
}
