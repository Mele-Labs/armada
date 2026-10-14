// Direction A, the strips board: the air traffic controller's board. Columns are the phases of one
// pipeline, Working, Needs you, Checking, In queue, Landed, and the last three are the merge line,
// running on to a main marker. Every Job and Session is one strip in the column it is in; its pull
// request is a chip on it and decides the column. A roll-call row along the top polls main, the merge
// line, the bays and Autopilot as marks. Each strip carries its acts in place. Mock only.
// proto/cockpit-three-directions.

import { Fragment } from "react";
import { ChevronRight, Hammer, Rows3 } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";

import type { CockpitProps } from "../../cockpit/body-seam";
import { useCalls } from "./calls";
import { ActBar, FixAct, KEYS, KeySheet, MarkGlyph, PeekPop, PrChip, ReplyBox, StationMark, useShipActs, useShipKeys, useShipUi, type Acts, type ShipUi } from "./shared";
import { PHASES, stationsOf, useShip, type Main, type Phase, type Subject } from "./ship";
import "./strips.css";

const COLUMN: Record<Phase, string> = { working: "Working", needs: "Needs you", checking: "Checking", queue: "In queue", landed: "Landed" };

/** Steps as pips, a Job's place in its workflow without a figure beside it. */
function Pips({ at, of, label }: { at: number; of: number; label: string }) {
  return (
    <Tooltip label={`Step ${at} of ${of}: ${label}`}>
      <span className="proto-pips" role="img" aria-label={`Step ${at} of ${of}`}>
        {Array.from({ length: of }, (_, index) => (
          <span key={index} className="proto-pips__pip" data-state={index + 1 < at ? "done" : index + 1 === at ? "now" : "next"} />
        ))}
      </span>
    </Tooltip>
  );
}

function Strip({ subject, selected, ui, acts }: { subject: Subject; selected: boolean; ui: ShipUi; acts: Acts }) {
  const tone = subject.alarm ? "bad" : subject.needs ? "ask" : "calm";
  return (
    <li
      className="proto-strip"
      data-subject={subject.key}
      data-tone={tone}
      data-kind={subject.kind}
      data-selected={selected || undefined}
      aria-label={subject.title}
      tabIndex={0}
      onFocus={() => ui.select(subject.key)}
      onClick={() => ui.select(subject.key)}
    >
      <div className="proto-strip__band">
        <MarkGlyph mark={subject.mark} size={16} />
        <span className="proto-strip__title">{subject.title}</span>
      </div>
      <div className="proto-strip__body">
        <span className="proto-strip__repo">{subject.repo}</span>
        {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => acts.open({ ...subject, kind: "pr" })} />}
        {subject.step === undefined ? null : <Pips at={subject.step.at} of={subject.step.of} label={subject.step.label} />}
        {subject.slot === undefined ? null : (
          <Tooltip label={`Bay ${subject.slot}`}>
            <span className="proto-slot" role="img" aria-label={`Bay ${subject.slot}`}>
              <Rows3 size={12} aria-hidden />
            </span>
          </Tooltip>
        )}
        <span className="proto-strip__acts">
          <ActBar subject={subject} acts={acts} ui={ui} />
        </span>
      </div>
      {ui.replying === subject.key ? <ReplyBox subject={subject} acts={acts} ui={ui} /> : null}
      {ui.peeking === subject.key ? <PeekPop subject={subject} ui={ui} /> : null}
    </li>
  );
}

/** Main red, as a strip of its own at the head of Needs you: what failed, what broke it, and Fix. */
function MainStrip({ main, acts }: { main: Main; acts: Acts }) {
  const mark = { Glyph: Hammer, said: "Fixing", tone: "calm" as const };
  return (
    <li className="proto-strip" data-tone={main.fixing ? "calm" : "bad"} data-kind="main" aria-label={`Main is red in ${main.repo}`}>
      <div className="proto-strip__band">
        <StationMark station={{ id: "main", name: "Main", vote: "nogo" }} />
        <span className="proto-strip__title">{main.check}</span>
      </div>
      <div className="proto-strip__body">
        <span className="proto-strip__repo">{main.repo}</span>
        {main.broke === undefined ? null : <span className="proto-strip__fact">#{main.broke.number}</span>}
        {main.test === undefined ? null : <span className="proto-strip__fact proto-strip__fact--wide">{main.test}</span>}
        <span className="proto-strip__acts">{main.fixing ? <MarkGlyph mark={mark} /> : <FixAct main={main} acts={acts} />}</span>
      </div>
    </li>
  );
}

const rank = (one: Subject) => (one.alarm ? 0 : one.needs ? 1 : 2);

export function Strips(props: CockpitProps) {
  const ship = useShip(props);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const calls = useCalls(props, ship);
  const stations = stationsOf(ship);
  const columns = PHASES.map((phase) => ({ phase, items: ship.subjects.filter((one) => one.phase === phase).sort((a, b) => rank(a) - rank(b)) }));
  const order = columns.flatMap((column) => column.items);
  const current = useShipKeys({
    ship,
    order,
    ui,
    acts,
    cardOpen: calls.open,
    openCall: calls.openFront,
    // Left and right step to the strip at the same height in the next column.
    extra: (event, subject) => {
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return false;
      if (subject === undefined) return false;
      const here = columns.findIndex((column) => column.items.includes(subject));
      const row = columns[here]!.items.indexOf(subject);
      const step = event.key === "ArrowRight" ? 1 : -1;
      for (let at = here + step; at >= 0 && at < columns.length; at += step) {
        const items = columns[at]!.items;
        if (items.length > 0) {
          ui.select(items[Math.min(row, items.length - 1)]!.key);
          return true;
        }
      }
      return true;
    },
  });
  const red = ship.mains.filter((one) => one.state === "red");
  const alarm = red.length > 0 || ship.subjects.some((one) => one.alarm);
  const waiting = ship.subjects.some((one) => one.needs) || calls.front !== undefined;
  const main = ship.main;
  return (
    <div className="armada-cockpit proto" data-direction="strips">
      <section className="proto-view" aria-label="Dashboard" data-alarm={alarm || undefined} data-waiting={waiting || undefined}>
        <header className="proto-band" aria-label="Roll call">
          <span className="proto-band__marks">
            {stations
              .filter((one) => one.id === "main" || one.id === "merge" || one.id === "bays" || one.id === "autopilot")
              .map((one) => (
                <StationMark key={one.id} station={one} />
              ))}
          </span>
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
          <div className="proto-strips">
            {columns.map(({ phase, items }, at) => (
              <section key={phase} className="proto-col" data-phase={phase} data-line={at >= 2 || undefined} aria-label={COLUMN[phase]}>
                <header className="proto-col__band">
                  <span>{COLUMN[phase]}</span>
                  {phase === "landed" ? (
                    <span className="proto-col__main">
                      {main === undefined ? null : main.fixing ? <MarkGlyph mark={{ Glyph: Hammer, said: "Fixing main", tone: "calm" }} /> : <FixAct main={main} acts={acts} labeled={false} />}
                      <StationMark station={stations.find((one) => one.id === "main")!} />
                    </span>
                  ) : at >= 2 ? (
                    <ChevronRight size={14} aria-hidden className="proto-col__toward" />
                  ) : null}
                </header>
                <ul className="proto-col__list" aria-label={COLUMN[phase]}>
                  {phase === "needs" ? red.map((one) => <MainStrip key={one.root} main={one} acts={acts} />) : null}
                  {items.map((one) => (
                    <Fragment key={one.key}>
                      <Strip subject={one} selected={current?.key === one.key} ui={ui} acts={acts} />
                    </Fragment>
                  ))}
                </ul>
              </section>
            ))}
          </div>
          {calls.tray}
          {calls.card}
          <KeySheet ui={ui} more={[[["←", "→"], "Next column"]]} />
        </div>
      </section>
    </div>
  );
}

