import { useState } from "react";
import { ChevronDown, ChevronRight, Clock, Unplug, type LucideIcon } from "lucide-react";
import type { LoadoutItem, LoadoutKind, LoadoutState, LoadoutTurn, SessionLoadout } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { KIND_SAID, KNOWN, TOLD, USE_SAID, sourceSaid, sourcesOf, turnSaid, useSaid } from "../../loadout";

/** What a kind's list draws before it is asked for the rest. */
const BOUND = 40;

/** The whole Loadout once read, the latest turn alone while it is read, or the read refused. */
export type LoadoutRead = { state: "read"; loadout: SessionLoadout } | { state: "reading"; latest?: LoadoutTurn } | { state: "failed"; latest?: LoadoutTurn };

export type LoadoutSheetProps = {
  open: boolean;
  read: LoadoutRead;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose: () => void;
};

/** Only the states that are not the usual one draw a mark: a server that failed or is still connecting. */
const STATE_MARK: Partial<Record<LoadoutState, { Glyph: LucideIcon; said: string }>> = {
  failed: { Glyph: Unplug, said: "Failed" },
  pending: { Glyph: Clock, said: "Connecting" },
};

function StateMark({ state }: { state: LoadoutState | undefined }) {
  const mark = state === undefined ? undefined : STATE_MARK[state];
  if (mark === undefined) return null;
  return (
    <Tooltip label={mark.said}>
      <span className="armada-loadout__state" role="img" aria-label={mark.said} data-state={state}>
        <mark.Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** One kind's items, bounded, with the rest a press away. */
function KindList({ kind, items }: { kind: LoadoutKind; items: readonly LoadoutItem[] }) {
  const [all, setAll] = useState(false);
  if (items.length === 0) return null;
  const shown = all ? items : items.slice(0, BOUND);
  const heading = KIND_SAID[kind];
  return (
    <section className="armada-loadout__group" aria-label={heading}>
      <h4 className="armada-loadout__eyebrow">{heading}</h4>
      <ul className="armada-loadout__rows">
        {shown.map((one) => (
          <li key={`${one.source}:${one.from ?? ""}:${one.name}`} className="armada-loadout__row">
            <span className="armada-loadout__name">{one.name}</span>
            <StateMark state={one.state} />
            <span className="armada-loadout__source">{sourceSaid(one)}</span>
          </li>
        ))}
      </ul>
      {shown.length === items.length ? null : (
        <Button variant="ghost" size="sm" onClick={() => setAll(true)}>
          {`Show ${items.length - shown.length} more`}
        </Button>
      )}
    </section>
  );
}

function Items({ heading, kinds, items }: { heading: string; kinds: readonly LoadoutKind[]; items: readonly LoadoutItem[] }) {
  const held = kinds.filter((kind) => items.some((one) => one.kind === kind));
  return (
    <section className="armada-loadout__section" aria-label={heading}>
      <h3 className="armada-loadout__heading">{heading}</h3>
      {held.length === 0 ? <p className="armada-loadout__none">None reported</p> : held.map((kind) => <KindList key={kind} kind={kind} items={items.filter((one) => one.kind === kind)} />)}
    </section>
  );
}

/** The turns, latest first. **One is open at a time**, and it lists what it used. */
function Turns({ turns, shown }: { turns: readonly LoadoutTurn[]; shown: (source: string) => boolean }) {
  const latest = turns[turns.length - 1]?.started_at;
  const [open, setOpen] = useState<string | undefined>(latest);
  // A turn that began since the panel opened takes the open place only where the last latest still held it.
  const [lastLatest, setLastLatest] = useState(latest);
  if (latest !== lastLatest) {
    setLastLatest(latest);
    if (open === lastLatest) setOpen(latest);
  }
  return (
    <section className="armada-loadout__section" aria-label="Used by turn">
      <h3 className="armada-loadout__heading">Used by turn</h3>
      {turns.length === 0 ? (
        <p className="armada-loadout__none">None reported</p>
      ) : (
        <ul className="armada-loadout__turns">
          {[...turns].reverse().map((turn) => {
            const used = turn.used.filter((one) => shown(sourceSaid(one)));
            const expanded = open === turn.started_at;
            const Chevron = expanded ? ChevronDown : ChevronRight;
            return (
              <li key={turn.started_at} className="armada-loadout__turn">
                <button
                  type="button"
                  className="armada-loadout__turn-head"
                  aria-expanded={expanded}
                  onClick={() => setOpen(expanded ? undefined : turn.started_at)}
                >
                  <Chevron size={12} strokeWidth={2} aria-hidden />
                  {`Turn at ${turnSaid(turn.started_at)}`}
                </button>
                {!expanded ? null : used.length === 0 ? (
                  <p className="armada-loadout__none">None from this source</p>
                ) : (
                  <ul className="armada-loadout__rows" aria-label={`Used in the turn at ${turnSaid(turn.started_at)}`}>
                    {used.map((one) => (
                      <li key={`${one.kind}:${one.source}:${one.from ?? ""}:${one.name}:${one.by ?? ""}`} className="armada-loadout__row">
                        <span className="armada-loadout__kind">{USE_SAID[one.kind]}</span>
                        <span className="armada-loadout__name">{one.name}</span>
                        <span className="armada-loadout__source">{one.by === undefined ? useSaid(one) : `${useSaid(one)} · by ${one.by}`}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

const ALL = "";

/**
 * A Session's whole Loadout, in the panel over the work area that a Check's log opens in: what its agent knows, what
 * it was told, and what each turn used, every item with where it came from. **One filter narrows all three to a
 * source.** The turns are listed latest first, the latest open, and a press opens another in its place.
 */
export function LoadoutSheet({ open, read, floor = false, onClose }: LoadoutSheetProps) {
  const [source, setSource] = useState(ALL);
  const items = read.state === "read" ? read.loadout.items : [];
  const turns = read.state === "read" ? read.loadout.turns : read.latest === undefined ? [] : [read.latest];
  const sources = sourcesOf(items, turns.flatMap((one) => one.used));
  const picked = source !== ALL && !sources.includes(source) ? ALL : source;
  const shown = (said: string) => picked === ALL || said === picked;
  const kept = items.filter((one) => shown(sourceSaid(one)));
  return (
    <Sheet
      kind="session-loadout"
      open={open}
      floating
      floor={floor}
      title="Loadout"
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      controls={
        <Select aria-label="Source" value={picked} onChange={(event) => setSource(event.target.value)}>
          <option value={ALL}>All sources</option>
          {sources.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
      }
    >
      <div className="armada-loadout">
        {read.state === "failed" ? <p className="armada-loadout__none">Not read. Close the panel and open it again.</p> : null}
        {read.state === "read" ? (
          <>
            <Items heading="Knows" kinds={KNOWN} items={kept} />
            <Items heading="Told" kinds={TOLD} items={kept} />
          </>
        ) : null}
        <Turns turns={turns} shown={shown} />
      </div>
    </Sheet>
  );
}
