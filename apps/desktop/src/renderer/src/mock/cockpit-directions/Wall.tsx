// Direction B, the mission wall: a fixed wall of six stations, Sessions, Jobs, Merge line, Main, Bays
// and Autopilot. Each is a calm gray tile that condenses to one mark, Go, Hold or No-go, and shows only
// its exceptions. A station in trouble grows into the large tile and draws its subjects with their acts
// in place; the others shrink to the edge. A flight director line at the top names the station that needs
// him next, and `]` jumps to it. Mock only. proto/cockpit-three-directions.

import { useEffect, useRef, useState } from "react";
import { Hammer, Rows3, SquareTerminal, Box } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";

import type { CockpitProps } from "../../cockpit/body-seam";
import { useCalls } from "./calls";
import { ActBar, FixAct, KEYS, KeySheet, MarkGlyph, PeekPop, PrChip, ReplyBox, STATION_GLYPH, StationMark, StepsBar, stationLines, useShipActs, useShipKeys, useShipUi, type Acts, type ShipUi } from "./shared";
import { stationsOf, useShip, type Bay, type Main, type Ship, type Station, type StationId, type Subject, type Vote } from "./ship";
import "./wall.css";

const RANK: Record<Vote, number> = { go: 0, off: 0, hold: 1, nogo: 2 };
/** Where a tie goes first: main, then the merge line, then the work. */
const PRIORITY: readonly StationId[] = ["main", "merge", "sessions", "jobs", "checks", "bays", "autopilot"];

const rankSubject = (one: Subject) => (one.alarm ? 0 : one.needs ? 1 : 2);

/** The subjects a station draws: all of them, what is wrong first. */
export function subjectsOf(station: StationId, ship: Ship): Subject[] {
  const all =
    station === "sessions" ? ship.subjects.filter((one) => one.kind === "session") : station === "jobs" ? ship.subjects.filter((one) => one.kind === "job") : station === "merge" ? ship.subjects.filter((one) => one.pr !== undefined) : station === "checks" ? stationsOf(ship).find((one) => one.id === "checks")?.exceptions ?? [] : [];
  const landed = (one: Subject) => (one.pr?.stage === "landed" ? 1 : 0);
  return [...all].sort((a, b) => landed(a) - landed(b) || rankSubject(a) - rankSubject(b));
}

/** The line of a subject on the large tile: its mark, title, chips and acts. */
function Row({ subject, selected, ui, acts }: { subject: Subject; selected: boolean; ui: ShipUi; acts: Acts }) {
  const tone = subject.alarm ? "bad" : subject.needs ? "ask" : "calm";
  return (
    <li
      className="proto-row"
      data-subject={subject.key}
      data-tone={tone}
      data-selected={selected || undefined}
      aria-label={subject.title}
      tabIndex={0}
      onFocus={() => ui.select(subject.key)}
      onClick={() => ui.select(subject.key)}
    >
      <div className="proto-row__line">
        <MarkGlyph mark={subject.mark} />
        <span className="proto-row__title">{subject.title}</span>
        <span className="proto-row__facts">
          <span className="proto-row__repo">{subject.repo}</span>
          {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => acts.open({ ...subject, kind: "pr" })} />}
          {subject.step === undefined ? null : <StepsBar step={subject.step} tone={subject.mark.tone} />}
          {subject.slot === undefined ? null : (
            <Tooltip label={`Bay ${subject.slot}`}>
              <span className="proto-slot" role="img" aria-label={`Bay ${subject.slot}`}>
                <Rows3 size={12} aria-hidden />
              </span>
            </Tooltip>
          )}
        </span>
        <ActBar subject={subject} acts={acts} ui={ui} />
      </div>
      {ui.replying === subject.key ? <ReplyBox subject={subject} acts={acts} ui={ui} /> : null}
      {ui.peeking === subject.key ? <PeekPop subject={subject} ui={ui} acts={acts} /> : null}
    </li>
  );
}

/** Main's tile: what failed, what broke it, and Fix. */
export function MainBody({ main, acts }: { main: Main; acts: Acts }) {
  return (
    <div className="proto-main" data-state={main.state}>
      {main.state === "red" ? (
        <dl className="proto-main__facts">
          <div>
            <dt>check</dt>
            <dd>{main.check}</dd>
          </div>
          {main.test === undefined ? null : (
            <div>
              <dt>test</dt>
              <dd>{main.test}</dd>
            </div>
          )}
          {main.broke === undefined ? null : (
            <div>
              <dt>broke</dt>
              <dd>
                #{main.broke.number} {main.broke.title}
              </dd>
            </div>
          )}
          {main.by === undefined ? null : (
            <div>
              <dt>by</dt>
              <dd>
                {main.by.kind === "session" ? <SquareTerminal size={12} aria-hidden /> : <Box size={12} aria-hidden />} {main.by.title}
              </dd>
            </div>
          )}
        </dl>
      ) : null}
      <div className="proto-main__acts">
        {main.fixing ? <MarkGlyph mark={{ Glyph: Hammer, said: "Fixing main", tone: "calm" }} size={20} /> : <FixAct main={main} acts={acts} />}
      </div>
    </div>
  );
}

/** The bays, as the pool draws them: a held one under its holder, a free one open. */
export function BaysBody({ bays }: { bays: readonly Bay[] }) {
  return (
    <ul className="proto-bays" aria-label="Worktree slots">
      {bays.map((bay) => (
        <li key={`${bay.root}:${bay.slot}`} className="proto-bay" data-bay={bay.state}>
          <span className="proto-bay__band">
            <Rows3 size={12} aria-hidden />
            <span>{bay.repo}</span>
          </span>
          <span className="proto-bay__body">{bay.holder === undefined ? null : bay.holder.title}</span>
        </li>
      ))}
    </ul>
  );
}

/** What a station on the edge shows of itself: its mark and only what is wrong. */
export function EdgeTile({ station, ship, onPress }: { station: Station; ship: Ship; onPress: () => void }) {
  const Glyph = STATION_GLYPH[station.id];
  const lines =
    station.id === "main"
      ? ship.mains.filter((one) => one.state === "red").map((one) => ({ key: one.root, mark: undefined, text: one.check ?? "" }))
      : station.exceptions.slice(0, 3).map((one) => ({ key: one.key, mark: one.mark, text: one.title }));
  return (
    <button type="button" className="proto-tile" data-vote={station.vote} data-station={station.id} aria-label={stationLines(station).join("; ")} onClick={onPress}>
      <span className="proto-tile__band">
        <Glyph size={16} aria-hidden />
        <span>{station.name}</span>
        <StationMark station={station} />
      </span>
      <span className="proto-tile__body">
        {lines.map((line) => (
          <span key={line.key} className="proto-tile__line">
            {line.mark === undefined ? null : <MarkGlyph mark={line.mark} size={12} />}
            <span>{line.text}</span>
          </span>
        ))}
      </span>
    </button>
  );
}

export function Wall(props: CockpitProps) {
  const ship = useShip(props);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const calls = useCalls(props, ship);
  const stations = stationsOf(ship);
  const [pin, setPin] = useState<StationId>();
  const troubled = stations
    .filter((one) => RANK[one.vote] > 0)
    .sort((a, b) => RANK[b.vote] - RANK[a.vote] || PRIORITY.indexOf(a.id) - PRIORITY.indexOf(b.id));
  // A station that gets worse takes the large tile, whatever was chosen.
  const was = useRef<Partial<Record<StationId, number>>>({});
  const signature = stations.map((one) => `${one.id}:${RANK[one.vote]}`).join(" ");
  useEffect(() => {
    const worse = stations.some((one) => was.current[one.id] !== undefined && RANK[one.vote] > was.current[one.id]!);
    if (worse) setPin(undefined);
    was.current = Object.fromEntries(stations.map((one) => [one.id, RANK[one.vote]]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);
  const lead = stations.find((one) => one.id === pin) ?? troubled[0];
  const edge = stations.filter((one) => one !== lead);
  const order = lead === undefined ? [] : subjectsOf(lead.id, ship);
  const nextTrouble = (from: Station | undefined) => {
    if (troubled.length === 0) return undefined;
    const at = from === undefined ? -1 : troubled.findIndex((one) => one.id === from.id);
    return troubled[(at + 1) % troubled.length];
  };
  const next = nextTrouble(lead);
  const nextSubject = next === undefined ? undefined : next.id === "main" ? undefined : next.exceptions[0];
  const nextMain = next?.id === "main" ? ship.mains.find((one) => one.state === "red") : undefined;
  const current = useShipKeys({
    ship,
    order,
    ui,
    acts,
    cardOpen: calls.open,
    openCall: calls.openFront,
    extra: (event) => {
      if (event.key !== "]") return false;
      if (next !== undefined) setPin(next.id);
      return true;
    },
  });
  const alarm = troubled.some((one) => one.vote === "nogo");
  const waiting = troubled.length > 0 || calls.front !== undefined;
  const LeadGlyph = lead === undefined ? undefined : STATION_GLYPH[lead.id];
  const mainOf = ship.main;

  return (
    <div className="armada-cockpit proto" data-direction="wall">
      <section className="proto-view" aria-label="Dashboard" data-alarm={alarm || undefined} data-waiting={waiting || undefined}>
        <header className="proto-band" aria-label="Flight director">
          {next === undefined ? null : (
            <Tooltip label="Next station that needs you">
              <button type="button" className="proto-director" data-vote={next.vote} aria-label={`Next: ${next.name}`} onClick={() => setPin(next.id)}>
                <StationMark station={next} />
                <span className="proto-director__title">{nextSubject?.title ?? nextMain?.check ?? next.why[0] ?? ""}</span>
                <Kbd>]</Kbd>
              </button>
            </Tooltip>
          )}
          <span className="proto-band__keys">
            <Tooltip label="Open the call that arrived">
              <Kbd>{KEYS.call}</Kbd>
            </Tooltip>
            <Tooltip label="Move">
              <span>
                <Kbd>j</Kbd> <Kbd>k</Kbd>
              </span>
            </Tooltip>
            <Tooltip label="Keys">
              <Kbd>?</Kbd>
            </Tooltip>
          </span>
        </header>
        <div className="proto-stage">
          <div className="proto-wall" data-calm={lead === undefined || undefined}>
            {lead === undefined || LeadGlyph === undefined ? null : (
              <section className="proto-lead" data-vote={lead.vote} aria-label={lead.name}>
                <header className="proto-lead__band">
                  <LeadGlyph size={16} aria-hidden />
                  <span>{lead.name}</span>
                  <StationMark station={lead} />
                </header>
                <div className="proto-lead__body">
                  {lead.id === "main" && mainOf !== undefined ? (
                    <MainBody main={mainOf} acts={acts} />
                  ) : lead.id === "bays" ? (
                    <BaysBody bays={ship.bays} />
                  ) : order.length === 0 ? null : (
                    <ul className="proto-rows" aria-label={`${lead.name} subjects`}>
                      {order.map((one) => (
                        <Row key={one.key} subject={one} selected={current?.key === one.key} ui={ui} acts={acts} />
                      ))}
                    </ul>
                  )}
                </div>
              </section>
            )}
            <div className="proto-edge" data-calm={lead === undefined || undefined}>
              {edge.map((one) => (
                <EdgeTile key={one.id} station={one} ship={ship} onPress={() => setPin(one.id)} />
              ))}
            </div>
          </div>
          {calls.tray}
          {calls.card}
          <KeySheet ui={ui} more={[[["]"], "Next station in trouble"]]} />
        </div>
      </section>
    </div>
  );
}
