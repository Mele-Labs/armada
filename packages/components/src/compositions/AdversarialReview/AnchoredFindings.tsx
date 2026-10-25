import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { ScanSearch } from "lucide-react";

import { SLOT_NAMES, isPressed, pressedSlot } from "../../keymap";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { TaskMark } from "../TaskMark/TaskMark";
import { FindingRow, SettledRow, findingsOf } from "./AdversarialReview";
import type { AdversarialReviewState, Finding } from "./AdversarialReview";

/**
 * Findings pinned to the plan they are about, where the host draws the plan itself. A finding's `about` names a task or a group; one
 * with none, or one the plan no longer holds, is about the plan as a whole and stands under its summary. A group holding findings
 * opens in place to show the task each names, by its words, with the finding directly under it. The plan card and the view of the plan
 * in full draw the same claim, the same acts and the same keys, so the two offer the same things.
 */
export type Anchors = { whole: readonly Finding[]; at: (id: string) => readonly Finding[] };

/** `review`'s findings by what they are about. `known` is every group and task id of the plan drawn. */
export function anchorsOf(review: AdversarialReviewState, known: readonly string[]): Anchors {
  const found = findingsOf(review);
  const held = new Set(known);
  return {
    whole: found.filter((one) => one.about === undefined || !held.has(one.about)),
    at: (id) => found.filter((one) => one.about === id),
  };
}

export type FindingActs = { onSend: (id: string) => void; onDismiss: (id: string) => void };

/**
 * The keys of findings drawn in several places under one element, as the registry binds them (`finding_move`, `finding_send`,
 * `finding_dismiss`): one moves between those not yet acted on, in the order on screen; one sends the one in focus, one dismisses it,
 * and focus passes to the next. `bind` goes on the element that holds them all. A press on one of a finding's buttons keeps its own key.
 */
export function useFindingKeys(review: AdversarialReviewState | undefined, acts: FindingActs) {
  const scope = useRef<HTMLElement | null>(null);
  const after = useRef<string | undefined>(undefined);
  const live = () => [...(scope.current?.querySelectorAll<HTMLElement>("[data-finding]") ?? [])];
  useEffect(() => {
    const id = after.current;
    after.current = undefined;
    if (id !== undefined) live().find((one) => one.dataset.finding === id)?.focus();
  }, [review]);
  const act = (id: string, run: (id: string) => void) => {
    const all = live();
    const at = all.findIndex((one) => one.dataset.finding === id);
    after.current = (all[at + 1] ?? all[at - 1])?.dataset.finding;
    run(id);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const target = (event.target as HTMLElement).closest<HTMLElement>("[data-finding]");
    if (target === null || event.target !== target) return;
    const all = live();
    const at = all.indexOf(target);
    const moved = pressedSlot("finding_move", event);
    if (moved !== -1) {
      event.preventDefault();
      event.stopPropagation();
      const down = SLOT_NAMES.finding_move?.[moved] === "Down";
      all[Math.min(all.length - 1, Math.max(0, at + (down ? 1 : -1)))]?.focus();
    } else if (isPressed("finding_send", event)) {
      event.preventDefault();
      event.stopPropagation();
      act(target.dataset.finding!, acts.onSend);
    } else if (isPressed("finding_dismiss", event)) {
      event.preventDefault();
      event.stopPropagation();
      act(target.dataset.finding!, acts.onDismiss);
    }
  };
  return {
    bind: { ref: (el: HTMLElement | null) => void (scope.current = el), onKeyDown },
    send: (id: string) => act(id, acts.onSend),
    dismiss: (id: string) => act(id, acts.onDismiss),
  };
}

/** The findings pinned to one thing: each a claim with its acts, or, acted on, one muted line. A hairline down their edge ties them to it. */
export function FindingNotes({ findings, label, onSend, onDismiss }: { findings: readonly Finding[]; label: string } & FindingActs) {
  if (findings.length === 0) return null;
  return (
    <ul className="armada-adversarial__notes" aria-label={label}>
      {findings.map((one) =>
        one.settled === undefined ? (
          <FindingRow key={one.id} finding={one} onSend={() => onSend(one.id)} onDismiss={() => onDismiss(one.id)} />
        ) : (
          <SettledRow key={one.id} finding={one} />
        ),
      )}
    </ul>
  );
}

type Grouped = { id: string; title: string; tasks: readonly { id: string; text: string }[] };

/** What a group holds: findings about the group itself, then each task a finding names, in the plan's order, by its words. */
function held(group: Grouped, anchors: Anchors) {
  const own = anchors.at(group.id);
  const tasks = group.tasks.map((task) => ({ task, found: anchors.at(task.id) })).filter((one) => one.found.length > 0);
  return { own, tasks, all: [...own, ...tasks.flatMap((one) => one.found)] };
}

/** The open part of a group: its own findings directly under its line, then the task each names with its findings under it. */
export function GroupFindings({ group, anchors, id, ...acts }: { group: Grouped; anchors: Anchors; id?: string } & FindingActs) {
  const { own, tasks } = held(group, anchors);
  return (
    <div id={id} className="armada-adversarial__anchored">
      <FindingNotes findings={own} label={`Findings on ${group.title}`} {...acts} />
      {tasks.map(({ task, found }) => (
        <div key={task.id} className="armada-adversarial__task">
          <div className="armada-adversarial__task-line">
            <TaskMark state="open" />
            <span>{task.text}</span>
          </div>
          <FindingNotes findings={found} label={`Findings on ${task.text}`} {...acts} />
        </div>
      ))}
    </div>
  );
}

/**
 * A line of a plan's plain view, what a group gives, and the findings it holds. A group with none is its line alone. One with some
 * has a mark in a fixed column at the line's end, and opens in place under it; the mark shuts it. Once every finding in it is acted on
 * the mark goes and the muted lines stay.
 */
export function GivesRow({ text, group, anchors, ...acts }: { text: string; group: Grouped; anchors: Anchors } & FindingActs) {
  const [open, setOpen] = useState(true);
  const region = useId();
  const { all } = held(group, anchors);
  if (all.length === 0) return <li>{text}</li>;
  const live = all.some((one) => one.settled === undefined);
  return (
    <li className="armada-adversarial__given">
      <div className="armada-adversarial__given-row">
        <span>{text}</span>
        {live ? (
          <Tooltip label="Adversarial findings">
            <Button size="sm" variant="ghost" iconOnly aria-label={`Findings on ${text}`} aria-expanded={open} aria-controls={region} onClick={() => setOpen(!open)}>
              <ScanSearch size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        ) : null}
      </div>
      {open || !live ? <GroupFindings id={region} group={group} anchors={anchors} {...acts} /> : null}
    </li>
  );
}
