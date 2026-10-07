// **The badge is static here.** Job detail has a workflow rail, and the rail's
// current step names *which* step is working where this badge only names the
// Job's state — so the rail carries the pulse and this badge does not.
//
// **The title is the title, since #1484.** It was the last segment of a
// two-segment trail — `Overview ›` and then the Job — and the first segment
// was never wired to anything: every caller passed `from` and none passed
// `onLeave`, so it drew as grey text that looked like a way back and was not.
// The owner found it by pressing it. The trail went with it rather than being
// wired, because one segment naming the screen Bridge opens on repeats what
// Navigation already says.
//
// **The facts are a labelled run, not a table**, on screen without a click.
// They wrap first, and past this file's measured floor the least urgent
// gives way — #1093 dropped the rule that none ever did.
//
// **The run changes with the state, the block does not.** The facts are the
// caller's; their type, spacing and order are not.

// **Hedge by source.** Elapsed is measured and speaks flatly; spend is
// estimated and is marked approximate. Rendering the two alike would destroy
// trust in both.
//
// **A fact may point somewhere, and the run is where that belongs.** A pull
// request is read in the same breath as the branch it was opened from, and a
// control at the trailing edge would put going to read something among the
// acts that end a Job. So `href` makes a fact a link in place, beside the
// facts it is about, and nothing moves to the other end of the header.

// **No fact here ever navigates.** The anchor's default is cancelled and the
// address goes back to the caller, because a surface that let a click load a
// page would be a surface that could be steered off the app by a value that
// arrived on a wire. Where a caller sets `href` and no `onFollowed`, the fact
// draws as a link and the click does nothing — a caller wiring only half of
// it, and the one shape to look for when a link goes dead. **Nothing here has
// an exception**, since the trail went: every control this block draws either
// ends something or hands an address back.

// **What the header offers changes with the state; how it is arranged does
// not.** The set is the caller's, and which state carries what is written where
// that decision is made — `Acts` in Bridge's `JobDetail.tsx`. The arrangement
// is this file's: ghost first, the acts that end something as one outlined
// destructive group, and the one primary a Job detail ever carries last, at the
// trailing edge where the shell head puts its own.

import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { Fragment, useCallback } from "react";
import { ConceptLabel } from "../../concepts";
import { Badge } from "../../primitives/Badge/Badge";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Job detail header — what this Job is, and whatever you can do to it from
 * here. One component for every state job detail reaches: running, failed and
 * finished all render through this, so the block cannot drift between them.
 */
export type JobDetailField = {
  /** The label, where the fact needs one. `Dispatched by you` does not. */
  label?: ReactNode;
  value?: ReactNode;
  /**
   * Machine-derived: a step count, a branch, a duration, a cost. Mono is the
   * signal that Armada reported this rather than wrote it.
   */
  mono?: boolean;
  /**
   * The exact string this fact puts on the clipboard. Setting it makes the
   * value copy on click and go to `--accent` on hover, with no `copy` glyph.
   */
  copyValue?: string;
  /**
   * Where this fact goes, for a fact that names something outside Armada.
   * Setting it draws the value in `--accent` — the token the design contract
   * gives links — and hands the address to `onFollowed` when it is clicked.
   *
   * **The address is never the value.** A forge address is long, a person
   * reads one number out of it, and a fact run is a line of short readings;
   * what this carries is where the fact points, and the caller decides what it
   * spells. The full address is on the element's `title`, which is the only
   * place a person can still get at the whole of it.
   */
  href?: string;
  /**
   * The Job this fact names, for a fact that points *inside* Armada.
   *
   * **`href`'s twin, and not `href`.** A fact pointing outward carries an
   * address, draws as an anchor and leaves the app through `onFollowed`; this
   * one carries an id, draws as a button and stays. Spelling an in-app
   * destination as an address would make `onFollowed` answer two questions,
   * and a caller that reads its argument as a forge address would open the
   * wrong thing.
   *
   * Drawn in `--accent` the same way, because the affordance is the same:
   * press the value and land on what it names. **Without `onOpenJob` it draws
   * as plain text** — no control where there is nowhere to go, the rule the
   * replaced-job callout already keeps.
   */
  opensJob?: string;
  /**
   * A destination inside Armada that is not another Job, pressed in place —
   * the Studio a Job was dispatched from, #1674.
   *
   * **A bound press, not an id.** A Studio is opened on one of its nodes, so
   * the destination is two ids and the host's own navigation; `opensJob`'s
   * one-id shape would grow a callback per kind of place. `label` is the
   * tooltip and names where the press lands, because the value is the
   * registry's sentence and has no slot for the name.
   *
   * Drawn as `opensJob` is. **Absent where nothing opens**, so a fact with
   * nowhere to go stays plain text.
   */
  opens?: { label: string; onOpen: () => void };
  /**
   * The words after the value, where the fact reads as a sentence around it —
   * `All 4 of 4 steps advanced`. Sans, and never part of the mono run.
   */
  suffix?: ReactNode;
  /**
   * This fact continues the one before it, joined by a comma inside the same
   * span instead of separated by the gap between facts. `Stopped at Run tests,
   * step 3 of 4` is one fact — where the job stopped — carrying two values of
   * different type; the gap would read it as two unrelated ones and force
   * `step` to be re-cased as a label of its own.
   */
  continues?: boolean;
  /**
   * The state the thing this fact names is in, drawn as a badge after the
   * value: `Pull request #1750 [Merged]`. `Badge`'s own two fields, so a pull
   * request's state reads the same here as on the Land board's row for it.
   */
  badge?: { status: string; icon: LucideIcon; label: ReactNode };
};

export type JobDetailHeaderActionsProps = {
  /** The status token stem, e.g. `running`. Drives the badge's hue and tint. */
  status: string;
  /** The glyph, from `packages/icons/icons/`, group `Job state`. */
  statusIcon: LucideIcon;
  /** The verb, from the enum→verb map. Never written by hand where it ships. */
  statusLabel: ReactNode;
  /** Stands beside the badge and never in place of it: a paused Job's mark. */
  mark?: ReactNode;
  /** The Job's title, in the person's own words. Sans. Truncates in the
   *  trail; the untruncated title is the element's own `title`. */
  headline: ReactNode;
  /**
   * The job id, in mono and set back, behind whatever word names it —
   * `jobIdLabel`. It identifies, it does not describe.
   */
  jobId?: ReactNode;
  /**
   * The whole of the identifier, where what is drawn is short for it.
   *
   * **The reading is never the value, on `href`'s own rule.** A Job's handle
   * is its number and a slug of its title, and the owner read
   * `3-show-what-s-running-in-the-drones-stat` as a string that had come out
   * wrong. `Job 3` is what he is asked to read; this is what a click puts on
   * the clipboard and what the hover carries whole, because the handle is the
   * branch name and the worktree directory and it has to stay reachable.
   */
  jobIdWhole?: string;
  /**
   * What the id is called. **Drawn, not optional in practice**: the run is a
   * line of bare values and the first of them was the one nobody could name.
   * `undefined` draws the id alone, for a caller whose id needs no word.
   */
  jobIdLabel?: ReactNode;
  /** The facts, in the order the drawing runs them. */
  fields: JobDetailField[];
  /**
   * The controls at the trailing edge. **A terminal Job still carries one
   * where Fleet offers redispatch** — #1093 dropped the rule that this was
   * always empty there.
   *
   * **A running Job's settings open from here too**, left of the acts — they
   * had a line of their own under the facts, which read as the screen's main
   * button with nothing saying what pressing it did. Every setting is on one
   * panel now, and the way into it is a quiet control beside the others.
   */
  actions?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
  /**
   * A fact with an `href` was clicked. **The host opens it, not this** — a
   * component that reached for a browser could not be rendered in Storybook,
   * and the process with the shell is the one that should decide what leaving
   * the app means.
   */
  onFollowed?: (href: string) => void;
  /**
   * A fact with an `opensJob` was pressed. **The host navigates, not this** —
   * the same division `onFollowed` keeps, one screen in rather than one app
   * out.
   */
  onOpenJob?: (jobId: string) => void;
};

export function JobDetailHeaderActions({
  status,
  statusIcon,
  statusLabel,
  mark,
  headline,
  jobId,
  jobIdWhole,
  jobIdLabel,
  fields,
  actions,
  onCopied,
  onFollowed,
  onOpenJob,
}: JobDetailHeaderActionsProps) {
  // The anchor is a real one so it reads as a link and carries the address on
  // hover, and its default is cancelled so nothing here can navigate. See
  // `href`.
  const follow = useCallback(
    (event: MouseEvent<HTMLAnchorElement>, href: string) => {
      event.preventDefault();
      event.stopPropagation();
      onFollowed?.(href);
    },
    [onFollowed],
  );

  const copy = useCallback(
    (event: MouseEvent<HTMLSpanElement>, value: string) => {
      event.stopPropagation();
      void navigator.clipboard.writeText(value).then(
        () => onCopied?.(value),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way.
        () => onCopied?.(value),
      );
    },
    [onCopied],
  );

  // Facts that continue one another fold into a single span, so a comma-joined
  // fact wraps and truncates as the one thought it is.
  const runs: JobDetailField[][] = [];
  for (const field of fields) {
    const previous = runs[runs.length - 1];
    if (field.continues && previous) previous.push(field);
    else runs.push([field]);
  }

  return (
    <div className="armada-job-head">
      <div className="armada-job-head__top">
        <div className="armada-job-head__lead">
          {/* The Job's name, and the whole of it on hover where the column is
              too narrow to hold it. */}
          <h1 className="armada-job-head__here" title={typeof headline === "string" ? headline : undefined}>
            {headline}
          </h1>
          {/* No `pulsing`. The rail's current step is the running mark, so
              it carries the loop and this badge stays still. */}
          <Badge status={status} icon={statusIcon}>
            {statusLabel}
          </Badge>
          {mark}
        </div>
        {actions ? <div className="armada-job-head__actions">{actions}</div> : null}
      </div>
      <div className="armada-job-head__facts">
        {jobId ? (
          <span className="armada-job-head__fact armada-job-head__id">
            {jobIdLabel ? (
              <>
                <FactLabel>{jobIdLabel}</FactLabel>{" "}
              </>
            ) : null}
            {/* The whole identifier on the hover and on the clipboard, the
                short one on screen. A tooltip carrying the value behind an
                abbreviated one is the first of the three things the contract
                lets a tooltip hold. */}
            {jobIdWhole === undefined ? (
              <span data-mono>{jobId}</span>
            ) : (
              // A control, because it copies: the short form read aloud is a
              // number with nothing round it, so the whole handle is the name.
              <Tooltip asChild label={jobIdWhole}>
                <button
                  type="button"
                  data-mono
                  data-copies=""
                  aria-label={jobIdWhole}
                  onClick={(e) => copy(e, jobIdWhole)}
                >
                  {jobId}
                </button>
              </Tooltip>
            )}
          </span>
        ) : null}
        {runs.map((run, i) => (
          <span className="armada-job-head__fact" key={i}>
            {run.map((field, j) => (
              <Fragment key={j}>
                {j > 0 ? ", " : null}
                {field.label ? (
                  <>
                    <FactLabel>{field.label}</FactLabel>
                    {field.value !== undefined ? " " : null}
                  </>
                ) : null}
                {field.value === undefined ? null : field.opens !== undefined ? (
                  <Tooltip asChild label={field.opens.label}>
                    <button
                      type="button"
                      className="armada-job-head__value"
                      data-mono={field.mono || undefined}
                      data-opens=""
                      onClick={field.opens.onOpen}
                    >
                      {field.value}
                    </button>
                  </Tooltip>
                ) : field.opensJob !== undefined && onOpenJob !== undefined ? (
                  <button
                    type="button"
                    className="armada-job-head__value"
                    data-mono={field.mono || undefined}
                    data-opens=""
                    onClick={() => onOpenJob(field.opensJob as string)}
                  >
                    {field.value}
                  </button>
                ) : field.href !== undefined ? (
                  <a
                    className="armada-job-head__value"
                    data-mono={field.mono || undefined}
                    data-opens=""
                    href={field.href}
                    title={field.href}
                    onClick={(e) => follow(e, field.href as string)}
                  >
                    {field.value}
                  </a>
                ) : (
                  <span
                    className="armada-job-head__value"
                    data-mono={field.mono || undefined}
                    data-copies={field.copyValue !== undefined || undefined}
                    onClick={
                      field.copyValue !== undefined
                        ? (e) => copy(e, field.copyValue as string)
                        : undefined
                    }
                  >
                    {field.value}
                  </span>
                )}
                {field.suffix ? <> {field.suffix}</> : null}
                {field.badge === undefined ? null : (
                  <>
                    {field.label !== undefined || field.value !== undefined ? " " : null}
                    <Badge status={field.badge.status} icon={field.badge.icon}>
                      {field.badge.label}
                    </Badge>
                  </>
                )}
              </Fragment>
            ))}
          </span>
        ))}
      </div>
    </div>
  );
}

/**
 * A fact's label, and what it means where the label is an Armada word.
 *
 * **The sentence is looked up, never written here.** `Workflow`, `Branch` and
 * `Spend, estimated` are the vocabulary rather than this header's copy, so one
 * explanation serves the header, the run tree and *Where things are* alike —
 * the same word explained twice is two things that can disagree.
 *
 * It was this file's own copy of `ConceptLabel` until the proposal wanted a
 * third; the shared one lives beside the sentences, in `concepts.tsx`.
 */
const FactLabel = ConceptLabel;
