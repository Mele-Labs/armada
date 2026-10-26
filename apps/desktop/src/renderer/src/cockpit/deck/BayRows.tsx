// The bays as the flight deck lists them, each drawn as a bay: its number in the hue of the state it is in, the state's word its tooltip, whoever
// holds it, and the facts behind the state, each with its tooltip, grouped under a heading for each repository. A bay and the row of
// the Session that holds it light each other under the pointer. Nothing is worded beyond the facts and nothing is counted
// beside the list. The states are the pool's own (`bays.ts`). #2301.

import { ArrowUpFromLine, FilePenLine, Flame, FolderPlus, HardDrive, Mail, Snowflake, Unplug } from "lucide-react";
import { Standby } from "./Standby";
import { useState, type ReactNode } from "react";
import { Button, Inspect, Tooltip, timeSaid, useTimeFormat } from "@armada/components";
import type { Outcome } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { BayCard } from "./inspect-cards";

import { PanelKey } from "./keys";
import { holderKeyOf, shortOf, type BayView } from "./bays";

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
  const format = useTimeFormat();
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
        <Mark said={`Holder told at ${timeSaid(view.told, format)}`}>
          <Mail size={12} aria-hidden />
        </Mark>
      )}
      {last === undefined || last.open ? null : (
        <Mark said="Job closed">
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

/** The bay's number, painted large in the door. */
const Name = ({ view }: { view: BayView }) => (
  <span className="deck-bay__name">
    <span className="deck-bay__prefix">slot-</span>
    {view.slot.slot}
  </span>
);

/** The bay's headline: the holder, where a bay has one. */
function Lead({ view }: { view: BayView }) {
  if (view.state !== "leased" && view.state !== "stranded") return null;
  return (
    <span className="deck-bay__line">
      <Who view={view} />
    </span>
  );
}

/** A berth: a door in the state's hue carrying the bay's number, and to its right the holder and its branch. `foot` adds marks of a host's own. */
export function Berth({ view, foot }: { view: BayView; foot?: ReactNode }) {
  return (
    <>
      <span className="deck-bay__door">
        {/* The door is the number alone, in the state's hue, and the state's word is the number's tooltip. */}
        <Tooltip label={view.said ?? view.word}>
          <Name view={view} />
        </Tooltip>
      </span>
      <span className="deck-bay__body">
        <Lead view={view} />
        <Branch view={view} />
        <span className="deck-bay__foot">
          <Warmth view={view} />
          <Marks view={view} />
          {foot}
        </span>
      </span>
    </>
  );
}

/**
 * One more slot for a repository's pool, on the heading of its bays: the act and the glyph of Cleanup's add tile, sent the same way.
 * A second press waits for the first. Fleet makes the worktree before it answers, so the wait is on the button (`pending`) and said under the heading; the answer is said there until the next press.
 */
function AddSlot({ repo, manifest, onAdd }: { repo: string; manifest: string; onAdd: (manifest: string) => Promise<Outcome> }) {
  const [adding, setAdding] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  const [added, setAdded] = useState(false);
  const press = async () => {
    if (adding) return;
    setAdding(true);
    setRefused(undefined);
    setAdded(false);
    const outcome = await onAdd(manifest);
    if (outcome.ok) setAdded(true);
    else setRefused(`Slot not added: ${refusalWords(outcome)}`);
    setAdding(false);
  };
  return (
    <>
      <Tooltip label="Add a slot" asChild>
        <Button variant="ghost" size="sm" iconOnly className="deck-bays__add" aria-label={`Add a slot to ${repo}`} pending={adding} onClick={() => void press()}>
          <FolderPlus size={14} aria-hidden="true" />
        </Button>
      </Tooltip>
      {adding || added ? (
        <p className="deck-bays__refused" role="status">
          {adding ? "Adding a slot…" : "Slot added"}
        </p>
      ) : null}
      {refused === undefined ? null : (
        <p className="deck-bays__refused" role="alert">
          {refused}
        </p>
      )}
    </>
  );
}

/** The bays panel: every slot of the pool, a bay each, under a heading for its repository. A press selects the slot. */
export function Bays({ views, repoName, working, selected, hovered, onHover, onSelect, onOpenSlots, onAddSlot }: { views: readonly BayView[]; repoName: (manifest: string) => string; working: ReadonlySet<string>; selected: string | undefined; hovered: string | undefined; onHover: (holder: string | undefined) => void; onSelect: (key: string) => void; onOpenSlots?: (() => void) | undefined; onAddSlot?: ((manifest: string) => Promise<Outcome>) | undefined }) {
  const groups = [...new Set(views.map((view) => view.slot.manifest_id))].map((manifest) => ({ manifest, views: views.filter((view) => view.slot.manifest_id === manifest) }));
  return (
    <section className="deck-panel deck-bays" aria-label="Bays" tabIndex={-1}>
      <Standby kind="bays" />
      <header className="deck-band">
        <span className="deck-band__name">Bays</span>
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
        {groups.map((group) => {
          const repo = repoName(group.manifest);
          return (
            <li key={group.manifest} className="deck-list__band">
              <div className="deck-bays__head">
                <h3 className="deck-list__repository">{repo}</h3>
                {onAddSlot === undefined ? null : <AddSlot repo={repo} manifest={group.manifest} onAdd={onAddSlot} />}
              </div>
              <ul className="deck-list__group">
                {group.views.map((view) => {
                  const live = view.holder !== undefined && working.has(`${view.holder.kind}:${view.holder.id}`);
                  const holder = holderKeyOf(view);
                  return (
                    <li key={view.key}>
                      <Inspect card={<BayCard view={view} repo={repo} />} name={`slot-${view.slot.slot}`} side="beside" pin={false}>
                      <button
                        type="button"
                        className="deck-bay"
                        data-bay={view.state}
                        data-live={live || undefined}
                        data-hover={(holder !== undefined && holder === hovered) || undefined}
                        data-selected={selected === view.key || undefined}
                        aria-label={`slot-${view.slot.slot}, ${repo}, ${view.word}${view.holder === undefined ? "" : `, ${view.holder.title ?? view.holder.id}`}`}
                        onClick={() => onSelect(view.key)}
                        onMouseEnter={() => onHover(holder)}
                        onMouseLeave={() => onHover(undefined)}
                        onFocus={() => onHover(holder)}
                        onBlur={() => onHover(undefined)}
                      >
                        <Berth view={view} />
                      </button>
                      </Inspect>
                    </li>
                  );
                })}
              </ul>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
