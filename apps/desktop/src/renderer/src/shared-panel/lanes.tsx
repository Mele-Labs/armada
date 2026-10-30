// A Job's Workflow view: the workflow and the tickets, pull request and deploy it holds (`LaneCanvas`), with the comments on the pull request under the canvas as a monitor.
// **The body is one**: `LanesBody` is drawn in the app's side panel on a window narrower than `WIDE_WINDOW_PX`, and as a layer in the Job's spine on a wider one (`session-view/LanesLayer.tsx`), the same press or Esc closing either. The panel is the pull request panel's width. The lanes are read from what Fleet serves of the Job; the comments under them are a mock's.
// A comment here shows that it came in and what the agent made of it; pressing it opens the pull request at that comment, where the acts on it are. A rule waiting on the person opens
// what it needs (`rule.tsx`). **Above the canvas `+` adds to this Job's own copy, and under it the changes made to it are listed** (`lanes-copy.tsx`).

import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Check, ChevronLeft } from "lucide-react";
import { Button, COMMENT_READ, LaneCanvas, Tooltip, jobLanes } from "@armada/components";
import type { LanePick, LaneView, TriagedComment } from "@armada/components";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";
import { instanceKey } from "@armada/screens/src/session-lanes";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { teachHereOf, useTeachFrom } from "../teach";
import type { TeachHere } from "../teach";
import { SidePanel } from "./side";
import { useBack } from "./panels";
import { usePanelReads } from "./reads";
import { AddToJob, JobChanges } from "./lanes-copy";
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

/**
 * The lanes and the comments under them, for the panel and the layer to frame. Nothing when the Job has no lanes. **The head of a lane for a Job it started opens that Job's own
 * lanes in place of these**, under a Back and the Job's title. These stay drawn behind, hidden, so Back returns to them as they were: where they were panned, what was picked.
 */
export function LanesBody({ sessionId }: { sessionId: string }) {
  // The Job whose lanes are open over these, and the head that opened them.
  const [into, setInto] = useState<{ job: string; lane: string } | undefined>();
  const host = useRef<HTMLDivElement>(null);
  const sessions = useSessions();
  const title = sessions.find((one) => one.id === sessionId)?.title ?? "Workflow";
  const child = into === undefined ? undefined : sessions.find((one) => one.id === into.job);
  const back = () => {
    const from = into?.lane;
    setInto(undefined);
    // Focus goes back to the head that opened it.
    window.requestAnimationFrame(() => host.current?.querySelector<HTMLElement>(`[data-lane="${CSS.escape(from ?? "")}"]`)?.focus({ preventScroll: true }));
  };
  // **While a Job's own Workflow is open, Esc is Back wherever focus is.** The panel defers to `data-holds-esc` and the layer to a press taken, and this runs ahead of the layer's own listener.
  // Only the deepest open one answers, and a menu, the palette or a confirmation over it keeps its own Esc. The canvas has had its press first: a pick let go is not Back.
  const latest = useRef(back);
  latest.current = back;
  const open = child !== undefined;
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || popupIsOpen() || document.querySelector('.armada-palette, .armada-dialog, [role="alertdialog"]') !== null) return;
      if (host.current?.querySelectorAll(".sv-lanes__into").length !== 1) return;
      event.preventDefault();
      event.stopPropagation();
      latest.current();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);
  return (
    <div ref={host} className="sv-lanes__host">
      <OwnLanes sessionId={sessionId} away={child !== undefined} onOpen={(lane) => setInto({ job: instanceKey(lane.id), lane: lane.id })} />
      {child === undefined ? null : (
        <Into back={{ label: `Back to ${title}`, onBack: back }} title={child.title ?? "Workflow"}>
          <LanesBody sessionId={child.id} />
        </Into>
      )}
    </div>
  );
}

/**
 * A Job's own Workflow opened from a lane on its parent's: the way back and its title over its lanes. **Focus goes to Back**, so Enter, Tab and Esc work from the first moment: the canvas draws its
 * nodes a frame later, and the head that was pressed is hidden. `data-holds-esc` tells the panel that Esc is this one's (`LanesBody`).
 */
function Into({ back, title, children }: { back: { label: string; onBack: () => void }; title: string; children: ReactNode }) {
  const way = useRef<HTMLButtonElement>(null);
  useEffect(() => way.current?.focus({ preventScroll: true }), []);
  return (
    <div className="sv-lanes__into" data-holds-esc="">
      <header className="sv-lanes__into-head">
        <Button ref={way} variant="ghost" size="sm" title={back.label} onClick={back.onBack}>
          <ChevronLeft size={12} strokeWidth={2} aria-hidden="true" />
          {back.label}
        </Button>
        <h4 className="sv-lanes__into-title">{title}</h4>
      </header>
      {children}
    </div>
  );
}

/** One Job's lanes and the comments under them. `away` is a Job's own Workflow drawn over it: it stays mounted and is hidden, and `onOpen` opens one. */
function OwnLanes({ sessionId, away, onOpen }: { sessionId: string; away: boolean; onOpen: (lane: LaneView) => void }) {
  const sessions = useSessions();
  const session = sessions.find((one) => one.id === sessionId);
  const copy = useSessionsDraft()?.copy;
  const [picked, setPicked] = useState<LanePick | undefined>();
  // The panel is drawn outside the Job, so Teach here is told which Job and repository it is pressed in.
  const listed = usePanelReads()?.repositories?.().listed;
  const here = useMemo((): TeachHere => (session === undefined ? { sessionId, jobTitle: sessionId, repository: "" } : teachHereOf(session, listed ?? [])), [session, sessionId, listed]);
  const teach = useTeachFrom(here);
  const lanes = session?.lanes;
  // A lane for a Job this one started opens it where that Job has lanes of its own, and nowhere else. The ids are a string so the lanes are drawn again only when that changes.
  const withLanes = sessions.flatMap((one) => (one.lanes === undefined ? [] : [one.id])).join("\n");
  const drawn = useMemo(() => {
    if (lanes === undefined) return undefined;
    const made = jobLanes(lanes.job, lanes);
    const opens = (lane: LaneView): boolean => {
      const held = lanes.job.instances.find((one) => one.id === lane.id);
      return held !== undefined && lanes.job.copy.lifecycles.find((one) => one.id === held.lifecycle)?.kind === "job" && withLanes.split("\n").includes(instanceKey(held.id));
    };
    return { ...made, lanes: made.lanes.map((lane) => (opens(lane) ? { ...lane, opens: true as const } : lane)) };
  }, [lanes, withLanes]);
  if (session === undefined || lanes === undefined || drawn === undefined) return null;
  const { triage } = lanes.job;
  const held = triage === undefined ? undefined : (lanes.job.instances.find((one) => one.id === triage.instance)?.label ?? triage.instance);
  const pull = session.attachments.find((one) => one.kind === "pull_request");
  const pick = (next: LanePick | undefined) => {
    setPicked(next);
    // A rule waiting on the person opens what it needs; one with acts Fleet kept opens what each is.
    const needs = next?.kind === "rule" ? drawn.rules.find((one) => one.rule === next.id && one.needs !== undefined)?.needs : undefined;
    if (next !== undefined && needs !== undefined) openInPanel({ kind: "rule", sessionId: session.id, rule: next.id, row: needs });
    else if (next?.kind === "rule" && lanes.job.fired.some((one) => one.rule === next.id && (one.acts?.length ?? 0) > 0)) openInPanel({ kind: "rule", sessionId: session.id, rule: next.id });
  };
  return (
    <div className="sv-lanes__body" data-away={away || undefined}>
      {copy === undefined ? null : <AddToJob sessionId={session.id} lanes={lanes} acts={copy} />}
      <LaneCanvas
        label="Workflow"
        lanes={drawn.lanes}
        rules={drawn.rules}
        picked={picked}
        onPick={pick}
        onOpen={onOpen}
        {...(teach === undefined ? {} : { onTeach: (lane) => teach("ticket", lane.note === undefined ? lane.title : `${lane.title} ${lane.note}`, undefined, [instanceKey(lane.id)]) })}
      />
      <JobChanges sessionId={session.id} lanes={lanes} acts={copy} />
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
  );
}

export function LanesPanel({ item }: { item: Extract<PanelItem, { kind: "lanes" }> }) {
  const session = useSessions().find((one) => one.id === item.sessionId);
  const back = useBack();
  if (session?.lanes === undefined) return null;
  return (
    <SidePanel title="Workflow" heading={session.title ?? "Workflow"} through back={back} onClose={closePanel}>
      <LanesBody sessionId={session.id} />
    </SidePanel>
  );
}
