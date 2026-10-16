// The bays as the flight deck lists them, each drawn as a bay: the state it is in as a glyph with its tooltip, the kind of whoever
// holds it, and the facts behind the state, each with its tooltip. Four looks are on trial, `?bayTake=a|b|c|d` in the mock. Nothing
// is worded beyond the facts and nothing is counted beside the list. The states are the pool's own (`bays.ts`). #2301.

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

/** Whoever holds the bay, or held it last. */
function Who({ view }: { view: BayView }) {
  if (view.state === "leased" && view.holder !== undefined) {
    return (
      <Cut full={view.holder.title ?? view.holder.id} className="deck-bay__who">
        {view.holder.title ?? shortOf(view.holder.id)}
      </Cut>
    );
  }
  if (view.state !== "stranded") return null;
  const last = view.last;
  return last === undefined ? (
    <span className="deck-bay__who" data-gone>
      holder gone
    </span>
  ) : (
    <Cut full={last.title ?? last.id} className="deck-bay__who">
      {last.title ?? shortOf(last.id)}
    </Cut>
  );
}

/** The branch the bay is on now, where there is one to name. */
function Branch({ view }: { view: BayView }) {
  if (view.state !== "leased" || view.holder === undefined || view.branch === undefined) return null;
  return (
    <Cut full={view.branch} className="deck-bay__branch">
      {view.branch}
    </Cut>
  );
}

/** What is left on a stranded bay, and whether its holder was told or is gone. */
function Marks({ view }: { view: BayView }) {
  if (view.state !== "stranded") return null;
  const last = view.last;
  return (
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
  );
}

/** A free or closed bay: warm or cold. */
function Warmth({ view }: { view: BayView }) {
  if (view.state !== "free" && view.state !== "closed") return null;
  const Glyph = view.warm ? Flame : Snowflake;
  return (
    <Mark said={view.warm ? "Warm" : "Cold"}>
      <Glyph size={12} aria-hidden />
    </Mark>
  );
}

/** The kind of whoever holds a leased bay. */
function Kind({ view }: { view: BayView }) {
  return view.state === "leased" && view.holder !== undefined ? <Mark said={view.holder.kind === "job" ? "Job" : "Session"}>{kindGlyph(view.holder.kind)}</Mark> : null;
}

/** The state as a glyph, its word in the tooltip. */
function State({ view, size }: { view: BayView; size: number }) {
  return (
    <Tooltip label={view.said ?? view.word}>
      <span className="deck-bay__state" role="img" aria-label={view.word}>
        <view.Glyph size={size} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The bay's number, painted: `slot-` small, the figure large where a take sets it so. */
const Name = ({ view }: { view: BayView }) => (
  <span className="deck-bay__name">
    <span className="deck-bay__prefix">slot-</span>
    {view.slot.slot}
  </span>
);

/** The bay's headline: the holder and its kind, where a bay has one. */
function Lead({ view }: { view: BayView }) {
  if (view.state !== "leased" && view.state !== "stranded") return null;
  return (
    <span className="deck-bay__line">
      <Kind view={view} />
      <Who view={view} />
    </span>
  );
}

type Take = "a" | "b" | "c" | "d";

/** Mock only: which of four looks the bays are drawn in, `?bayTake=a|b|c|d`. No param is the first. */
const takeOf = (): Take => {
  const asked = new URLSearchParams(window.location.search).get("bayTake");
  return asked === "b" || asked === "c" || asked === "d" ? asked : "a";
};

/** Takes A and D: a small card, a band naming the bay and its state over a dark body. A tints the band, D leaves it bare. */
function Card({ view, repo }: { view: BayView; repo: string }) {
  return (
    <>
      <span className="deck-bay__band">
        <State view={view} size={16} />
        <Name view={view} />
        <span className="deck-bay__repo">{repo}</span>
        <span className="deck-bay__word">{view.word}</span>
      </span>
      <span className="deck-bay__body">
        <span className="deck-bay__line">
          <Kind view={view} />
          <Who view={view} />
          <Marks view={view} />
          <Warmth view={view} />
          {timeOf(view)}
        </span>
        <Branch view={view} />
      </span>
    </>
  );
}

/** Take B: a berth, a door in the state's hue, the number painted beside it like a floor marking. */
function Berth({ view, repo }: { view: BayView; repo: string }) {
  return (
    <>
      <span className="deck-bay__door">
        <State view={view} size={16} />
      </span>
      <span className="deck-bay__floor">
        <Name view={view} />
        <span className="deck-bay__repo">{repo}</span>
      </span>
      <span className="deck-bay__body">
        <Lead view={view} />
        <Branch view={view} />
        <span className="deck-bay__foot">
          <Warmth view={view} />
          <Marks view={view} />
          {timeOf(view)}
        </span>
      </span>
    </>
  );
}

/** Take C: a marked bay seen from above, its number big, a lane painted in the state's hue, who is parked in it inside. */
function Tile({ view, repo }: { view: BayView; repo: string }) {
  return (
    <>
      <span className="deck-bay__top">
        <Name view={view} />
        <State view={view} size={16} />
      </span>
      <Lead view={view} />
      <Branch view={view} />
      <span className="deck-bay__foot">
        <Warmth view={view} />
        <Marks view={view} />
        <span className="deck-bay__repo">{repo}</span>
        {timeOf(view)}
      </span>
    </>
  );
}

/** The bays panel: every slot of the pool, a row each. A press selects the slot. */
export function Bays({ views, repoName, working, selected, onSelect, onOpenSlots }: { views: readonly BayView[]; repoName: (manifest: string) => string; working: ReadonlySet<string>; selected: string | undefined; onSelect: (key: string) => void; onOpenSlots?: (() => void) | undefined }) {
  const take = takeOf();
  const Cell = take === "a" || take === "d" ? Card : take === "b" ? Berth : Tile;
  return (
    <section className="deck-panel deck-bays" aria-label="Bays" data-take={take} tabIndex={-1}>
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
                data-kept={view.kept !== undefined || undefined}
                data-live={live || undefined}
                data-selected={selected === view.key || undefined}
                aria-label={`slot-${view.slot.slot}, ${repo}, ${view.word}${view.holder === undefined ? "" : `, ${view.holder.title ?? view.holder.id}`}`}
                onClick={() => onSelect(view.key)}
              >
                <Cell view={view} repo={repo} />
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
