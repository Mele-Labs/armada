import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { LoadoutItem, LoadoutKind, LoadoutTurn, SessionLoadout } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { KIND_SAID, KNOWN, TOLD, USE_SAID, loadoutKeyOf, loadoutRefOf, countSaid, sourceSaid, sourcesOf, useSaid, type LoadoutRef } from "../../loadout";
import { timeSaid, useTimeFormat } from "../../time-format";
import { LoadoutItemPanel, type LoadoutOpened } from "./LoadoutItemPanel";
import { McpServers, useRevealed, type McpGroup, type Opens } from "./LoadoutMcp";
import { SkillActs, skillActsOf, type SkillGroup } from "./LoadoutSkill";
import { StateMark } from "./LoadoutStateMark";

export type { LoadoutItemRead, LoadoutOpened } from "./LoadoutItemPanel";
export { mcpActOf, type McpGroup, type McpServerRow } from "./LoadoutMcp";
export type { LoadoutSkillAct, LoadoutSkillActs, SkillGroup } from "./LoadoutSkill";

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
  /** The item open in the panel to the left, and what it says as the host read it. Absent draws no such panel. */
  opened?: LoadoutOpened;
  /** A row pressed, or `null` for the item's panel closed. Absent leaves the rows as text. */
  onOpenItem?: (item: LoadoutRef | null) => void;
  /** A hosted Job's MCP servers, live, each with its act. Absent draws them from the agent's own report, as a terminal Job's are. */
  mcp?: McpGroup;
  /** The group the panel was opened on, scrolled into view: typing `/mcp` opens it on its MCP servers. */
  at?: "mcp";
  /** Kit's acts on the skills it holds, as the host matched them against Kit's inventory. Absent draws none. */
  skills?: SkillGroup;
};

/**
 * One row: a button that opens its item where the host opens items, marked while it is the one open. `acts` sit at its
 * trailing edge, beside the press rather than inside it.
 */
function Row({ item, opens, children, acts }: { item: LoadoutRef; opens: Opens; children: ReactNode; acts?: ReactNode }) {
  const acted = acts !== undefined && acts !== null;
  if (opens === undefined) {
    return acted ? (
      <li className="armada-loadout__item armada-loadout__acted">
        <span className="armada-loadout__row">{children}</span>
        {acts}
      </li>
    ) : (
      <li className="armada-loadout__row">{children}</li>
    );
  }
  const key = loadoutKeyOf(item);
  return (
    <li className={acted ? "armada-loadout__item armada-loadout__acted" : "armada-loadout__item"}>
      <button type="button" className="armada-loadout__row armada-loadout__press" aria-current={opens.open === key ? "true" : undefined} onClick={() => opens.onOpen(loadoutRefOf(item))}>
        {children}
      </button>
      {acts}
    </li>
  );
}

/** One kind's items, bounded, with the rest a press away. */
function KindList({ kind, items, opens, reveal = false, note, skills }: { kind: LoadoutKind; items: readonly LoadoutItem[]; opens: Opens; reveal?: boolean; note?: string | undefined; skills?: SkillGroup | undefined }) {
  const [all, setAll] = useState(false);
  const ref = useRevealed(reveal);
  if (items.length === 0) return null;
  const shown = all ? items : items.slice(0, BOUND);
  const heading = KIND_SAID[kind];
  return (
    <section ref={ref} className="armada-loadout__group" aria-label={heading}>
      <h4 className="armada-loadout__eyebrow">{heading}</h4>
      {note === undefined ? null : <p className="armada-loadout__none">{note}</p>}
      <ul className="armada-loadout__rows">
        {shown.map((one) => {
          const acts = skillActsOf(one, skills);
          return (
            <Row key={loadoutKeyOf(one)} item={one} opens={opens} acts={acts === undefined || skills === undefined ? undefined : <SkillActs item={loadoutRefOf(one)} acts={acts} onAct={skills.onAct} />}>
              <span className="armada-loadout__name">{one.name}</span>
              <StateMark state={one.state} />
              <span className="armada-loadout__source">{sourceSaid(one)}</span>
            </Row>
          );
        })}
      </ul>
      {shown.length === items.length ? null : (
        <Button variant="ghost" size="sm" onClick={() => setAll(true)}>
          {`Show ${items.length - shown.length} more`}
        </Button>
      )}
    </section>
  );
}

/** The live MCP group where the host reads one, else the agent's report of each kind. */
type Live = { mcp: McpGroup | undefined; all: readonly LoadoutItem[]; shown: (said: string) => boolean; at: "mcp" | undefined; skills: SkillGroup | undefined };

function Items({ heading, kinds, items, opens, live }: { heading: string; kinds: readonly LoadoutKind[]; items: readonly LoadoutItem[]; opens: Opens; live: Live }) {
  const servers = live.mcp?.servers;
  // A live server the agent also reported keeps that report's source for the filter; one it did not is under All sources only.
  const kept = servers?.filter((row) => {
    const reported = live.all.find((one) => one.kind === "mcp_server" && one.name === row.name);
    return reported === undefined ? live.shown("") : live.shown(sourceSaid(reported));
  });
  const held = kinds.filter((kind) => (kind === "mcp_server" && kept !== undefined && kept.length > 0) || items.some((one) => one.kind === kind));
  return (
    <section className="armada-loadout__section" aria-label={heading}>
      <h3 className="armada-loadout__heading">{heading}</h3>
      {held.length === 0 ? (
        <p className="armada-loadout__none">None reported</p>
      ) : (
        held.map((kind) =>
          kind === "mcp_server" && live.mcp !== undefined && kept !== undefined ? (
            <McpServers key={kind} servers={kept} items={live.all} said={live.mcp.said} opens={opens} onAct={live.mcp.onAct} reveal={live.at === "mcp"} />
          ) : (
            <KindList key={kind} kind={kind} items={items.filter((one) => one.kind === kind)} opens={opens} reveal={kind === "mcp_server" && live.at === "mcp"} note={kind === "mcp_server" ? live.mcp?.said : undefined} skills={kind === "skill" ? live.skills : undefined} />
          ),
        )
      )}
    </section>
  );
}

/** The turns, latest first. **One is open at a time**, and it lists what it used. */
function Turns({ turns, shown, opens }: { turns: readonly LoadoutTurn[]; shown: (source: string) => boolean; opens: Opens }) {
  const latest = turns[turns.length - 1]?.started_at;
  const format = useTimeFormat();
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
                  Turn at <Stamp still at={turn.started_at} />
                </button>
                {!expanded ? null : used.length === 0 ? (
                  <p className="armada-loadout__none">None from this source</p>
                ) : (
                  <ul className="armada-loadout__rows" aria-label={`Used in the turn at ${timeSaid(turn.started_at, format)}`}>
                    {used.map((one) => (
                      <Row key={`${loadoutKeyOf(one)}:${one.by ?? ""}`} item={one} opens={opens}>
                        <span className="armada-loadout__kind">{USE_SAID[one.kind]}</span>
                        <span className="armada-loadout__name">{one.name}</span>
                        {countSaid(one) === undefined ? null : <span className="armada-loadout__count">{countSaid(one)}</span>}
                        <span className="armada-loadout__source">{one.by === undefined ? useSaid(one) : `${useSaid(one)} · by ${one.by}`}</span>
                      </Row>
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
 *
 * **Every row is a button** where the host opens items: a press opens what that item says in a panel to the left, as
 * the pull request's file opens to the left of its panel, and a press on another row replaces it. `Esc` closes that
 * panel first, and the Loadout stays.
 */
export function LoadoutSheet({ open, read, floor = false, onClose, opened, onOpenItem, mcp, at, skills }: LoadoutSheetProps) {
  const [source, setSource] = useState(ALL);
  const items = read.state === "read" ? read.loadout.items : [];
  const turns = read.state === "read" ? read.loadout.turns : read.latest === undefined ? [] : [read.latest];
  const sources = sourcesOf(items, turns.flatMap((one) => one.used));
  const picked = source !== ALL && !sources.includes(source) ? ALL : source;
  const shown = (said: string) => picked === ALL || said === picked;
  const kept = items.filter((one) => shown(sourceSaid(one)));
  const live: Live = { mcp, all: items, shown, at, skills };
  const liveState = opened?.item.kind === "mcp_server" ? mcp?.servers?.find((one) => one.name === opened.item.name)?.state : undefined;
  const opens: Opens = onOpenItem === undefined ? undefined : { open: opened === undefined ? undefined : loadoutKeyOf(opened.item), onOpen: onOpenItem };
  return (
    <>
    <Sheet
      kind="session-loadout"
      open={open}
      floating
      floor={floor}
      title="Loadout"
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      under={open && opened !== undefined}
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
            <Items heading="Knows" kinds={KNOWN} items={kept} opens={opens} live={live} />
            <Items heading="Told" kinds={TOLD} items={kept} opens={opens} live={live} />
          </>
        ) : mcp?.servers === undefined ? null : (
          <Items heading="Knows" kinds={["mcp_server"]} items={[]} opens={opens} live={live} />
        )}
        <Turns turns={turns} shown={shown} opens={opens} />
      </div>
    </Sheet>
    {!open || opened === undefined ? null : <LoadoutItemPanel opened={opened} items={items} {...(liveState === undefined ? {} : { state: liveState })} floor={floor} onClose={() => onOpenItem?.(null)} />}
    </>
  );
}
