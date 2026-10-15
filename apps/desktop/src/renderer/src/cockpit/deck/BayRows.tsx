// The bays as the flight deck lists them: two lines a row, in the list's own row system. Line one is the bay, the state it is in as
// a glyph, and the kind of whoever holds it; line two is the facts behind the state, in muted sans, each with its tooltip. Nothing is
// worded beyond the facts and nothing is counted beside the list. The states are the pool's own (`bays.ts`). #2301.

import { ArrowUpFromLine, Box, FilePenLine, Flame, HardDrive, Mail, Snowflake, SquareTerminal, Unplug } from "lucide-react";
import type { ReactNode } from "react";
import { Button, Tooltip } from "@armada/components";

import { PanelKey } from "./keys";
import { clockOf, shortOf, spanOf, type BayView, type Holder } from "./bays";

export const kindGlyph = (kind: Holder["kind"], size = 12) => (kind === "job" ? <Box size={size} aria-hidden /> : <SquareTerminal size={size} aria-hidden />);

/** A glyph with its tooltip, which is also its name. */
function Mark({ said, children }: { said: string; children: ReactNode }) {
  return (
    <Tooltip label={said}>
      <span className="deck-bay__mark" role="img" aria-label={said}>
        {children}
      </span>
    </Tooltip>
  );
}

/** A piece of text that can be cut short, whole in its tooltip. */
function Cut({ full, className, children }: { full: string; className: string; children: ReactNode }) {
  return (
    <Tooltip label={full}>
      <span className={className}>{children}</span>
    </Tooltip>
  );
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** How long a leased bay has been held, or how long since a free one was given back: the last figure of line one. */
function timeOf(view: BayView): ReactNode {
  const [ms, said] = view.state === "leased" ? [view.heldFor, "Held for"] : view.state === "free" || view.state === "closed" ? [view.freed, "Given back"] : [undefined, ""];
  if (ms === undefined) return null;
  return (
    <Cut full={view.state === "leased" ? `${said} ${spanOf(ms)}` : `${said} ${spanOf(ms)} ago`} className="deck-bay__time">
      {spanOf(ms)}
    </Cut>
  );
}

/** Line two: what stands behind the state. */
export function BayFacts({ view }: { view: BayView }) {
  if (view.state === "leased" && view.holder !== undefined) {
    const who = view.holder.title ?? shortOf(view.holder.id);
    return (
      <>
        <Cut full={view.holder.title ?? view.holder.id} className="deck-bay__who">
          {who}
        </Cut>
        {view.branch === undefined ? null : (
          <Cut full={view.branch} className="deck-bay__branch">
            {view.branch}
          </Cut>
        )}
      </>
    );
  }
  if (view.state === "stranded") {
    const last = view.last;
    return (
      <>
        {last === undefined ? (
          <span className="deck-bay__who" data-gone>
            holder gone
          </span>
        ) : (
          <Cut full={last.title ?? last.id} className="deck-bay__who">
            {last.title ?? shortOf(last.id)}
          </Cut>
        )}
        <span className="deck-bay__marks">
          {(view.dirty ?? 0) > 0 ? (
            <Mark said={`${plural(view.dirty!, "file", "files")} uncommitted`}>
              <FilePenLine size={12} aria-hidden />
            </Mark>
          ) : null}
          {(view.unpushed ?? 0) > 0 ? (
            <Mark said={`${plural(view.unpushed!, "commit", "commits")} not pushed`}>
              <ArrowUpFromLine size={12} aria-hidden />
            </Mark>
          ) : null}
          {view.told === undefined ? null : (
            <Mark said={`Holder told at ${clockOf(view.told)}`}>
              <Mail size={12} aria-hidden />
            </Mark>
          )}
          {last === undefined || last.open ? null : (
            <Mark said={last.kind === "job" ? "Job ended" : "Session closed"}>
              <Unplug size={12} aria-hidden />
            </Mark>
          )}
        </span>
      </>
    );
  }
  if (view.state === "free" || view.state === "closed") {
    const Warmth = view.warm ? Flame : Snowflake;
    return (
      <>
        <Mark said={view.warm ? "Warm" : "Cold"}>
          <Warmth size={12} aria-hidden />
        </Mark>
      </>
    );
  }
  return null;
}

/** The bays panel: every slot of the pool, a row each. A press selects the slot. */
export function Bays({ views, repoName, working, selected, onSelect, onOpenSlots }: { views: readonly BayView[]; repoName: (manifest: string) => string; working: ReadonlySet<string>; selected: string | undefined; onSelect: (key: string) => void; onOpenSlots?: (() => void) | undefined }) {
  return (
    <section className="deck-panel deck-bays" aria-label="Bays" tabIndex={-1}>
      <header className="deck-band">
        Bays
        {onOpenSlots === undefined ? null : (
          <Tooltip label="Worktree Slots" asChild>
            <Button variant="ghost" size="sm" iconOnly aria-label="Worktree Slots" onClick={onOpenSlots}>
              <HardDrive size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        )}
        <PanelKey id="bays" />
      </header>
      <ul className="deck-bays__rows">
        {views.map((view) => {
          const repo = repoName(view.slot.manifest_id);
          const live = view.holder !== undefined && working.has(`${view.holder.kind}:${view.holder.id}`);
          return (
            <li key={view.key}>
              <button
                type="button"
                className="deck-bay"
                data-bay={view.state}
                data-live={live || undefined}
                data-selected={selected === view.key || undefined}
                aria-label={`slot-${view.slot.slot}, ${repo}, ${view.word}${view.holder === undefined ? "" : `, ${view.holder.title ?? view.holder.id}`}`}
                onClick={() => onSelect(view.key)}
              >
                <span className="deck-bay__head">
                  <Tooltip label={view.said ?? view.word}>
                    <span className="deck-bay__state" role="img" aria-label={view.word}>
                      <view.Glyph size={16} aria-hidden />
                    </span>
                  </Tooltip>
                  <span className="deck-bay__name">slot-{view.slot.slot}</span>
                  {view.state === "leased" && view.holder !== undefined ? (
                    <Mark said={view.holder.kind === "job" ? "Job" : "Session"}>{kindGlyph(view.holder.kind)}</Mark>
                  ) : null}
                  <span className="deck-bay__repo">{repo}</span>
                  {timeOf(view)}
                </span>
                <span className="deck-bay__facts">
                  <BayFacts view={view} />
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
