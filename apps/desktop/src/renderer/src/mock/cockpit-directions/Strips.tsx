// Direction A, the strips board: the air traffic controller's board. Every Job and Session is one strip in
// the one column its standing puts it in, the words the timeline's groups use: Needs you, Working, Stuck or
// held, Done. A Job's workflow is its step bar inside the strip, never a column. The merge line is the one real
// pipeline and is only about pull requests: a rail along the foot, Open, Checking, In queue, then the main
// marker, each pull request a unit joined to its owner's strip by a chip and by hover or selection. A roll
// call row along the top polls main, the merge line, the bays and Autopilot as marks. Mock only.
// proto/cockpit-three-directions.

import { useState } from "react";
import type { ReactNode } from "react";
import { ChevronRight, Hammer, Rows3 } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import type { CockpitProps } from "../../cockpit/body-seam";
import { useCalls } from "./calls";
import { ActBar, FixAct, KEYS, KeySheet, MarkGlyph, PeekPop, PrChip, ReplyBox, StationMark, StepsBar, useShipActs, useShipKeys, useShipUi, type Acts, type ShipUi } from "./shared";
import { PHASES, stationsOf, useShip, type Main, type Phase, type Pr, type Station, type Subject } from "./ship";
import "./strips.css";

const COLUMN: Record<Phase, string> = { needs: "Needs you", working: "Working", held: "Stuck or held", done: "Done" };

/** The stage of the one pipeline a pull request is at. */
type Stage = "open" | "checking" | "queue";
const STAGES: readonly { id: Stage; said: string }[] = [
  { id: "open", said: "Open" },
  { id: "checking", said: "Checking" },
  { id: "queue", said: "In queue" },
];
const stageOf = (pr: Pr): Stage | "main" => (pr.stage === "landed" ? "main" : pr.stage === "queued" || pr.stage === "ready" ? "queue" : pr.ci === "waiting_on_main" ? "open" : "checking");

type Link = { setHover: (key: string | undefined) => void };

function Strip({ subject, selected, linked, ui, acts, link }: { subject: Subject; selected: boolean; linked: boolean; ui: ShipUi; acts: Acts; link: Link }) {
  const tone = subject.alarm ? "bad" : subject.needs ? "ask" : "calm";
  return (
    <li
      className="proto-strip"
      data-subject={subject.key}
      data-tone={tone}
      data-kind={subject.kind}
      data-selected={selected || undefined}
      data-linked={linked || undefined}
      aria-label={subject.title}
      tabIndex={0}
      onFocus={() => ui.select(subject.key)}
      onClick={() => ui.select(subject.key)}
      onPointerEnter={() => link.setHover(subject.key)}
      onPointerLeave={() => link.setHover(undefined)}
    >
      <div className="proto-strip__band">
        <MarkGlyph mark={subject.mark} size={16} />
        <span className="proto-strip__title">{subject.title}</span>
      </div>
      <div className="proto-strip__body">
        <span className="proto-strip__repo">{subject.repo}</span>
        {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => ui.select(subject.key)} />}
        {subject.step === undefined ? null : <StepsBar step={subject.step} tone={subject.mark.tone} />}
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
      {ui.peeking === subject.key ? <PeekPop subject={subject} ui={ui} acts={acts} /> : null}
    </li>
  );
}

/** A pull request on the rail. Joined to its owner: hover or select either and both light. Merge acts here. */
function Unit({ subject, pr, linked, ui, acts, link }: { subject: Subject; pr: Pr; linked: boolean; ui: ShipUi; acts: Acts; link: Link }) {
  return (
    <li
      className="proto-unit"
      data-subject={subject.key}
      data-stage={pr.stage}
      data-linked={linked || undefined}
      aria-label={`#${pr.number} ${pr.title}`}
      tabIndex={0}
      onFocus={() => ui.select(subject.key)}
      onClick={() => ui.select(subject.key)}
      onPointerEnter={() => link.setHover(subject.key)}
      onPointerLeave={() => link.setHover(undefined)}
    >
      <PrChip pr={pr} />
      <span className="proto-unit__title">{pr.owner?.title ?? pr.title}</span>
      {subject.mergeable ? (
        <Button variant="secondary" size="sm" aria-label={`Merge #${pr.number}`} onClick={() => acts.merge(subject)}>
          Merge
          <Kbd>{KEYS.merge}</Kbd>
        </Button>
      ) : null}
    </li>
  );
}

/** Main, the end of the rail. Red, it is the alarm, and Fix is on it. */
function MainMarker({ main, station, acts, children }: { main: Main | undefined; station: Station; acts: Acts; children: ReactNode }) {
  return (
    <div className="proto-rail__main" data-state={main?.state ?? "unknown"}>
      <div className="proto-rail__mainhead">
        <StationMark station={station} />
        {main === undefined || main.state !== "red" ? null : (
          <>
            <span className="proto-rail__check">{main.check}</span>
            {main.fixing ? <MarkGlyph mark={{ Glyph: Hammer, said: "Fixing main", tone: "calm" }} /> : <FixAct main={main} acts={acts} />}
          </>
        )}
      </div>
      <ul className="proto-rail__landed" aria-label="Landed">
        {children}
      </ul>
    </div>
  );
}

const rank = (one: Subject) => (one.alarm ? 0 : one.needs ? 1 : 2);

export function Strips(props: CockpitProps) {
  const ship = useShip(props);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const calls = useCalls(props, ship);
  const [hover, setHover] = useState<string | undefined>();
  const link: Link = { setHover };
  const stations = stationsOf(ship);
  const strips = ship.subjects.filter((one) => one.kind !== "pr");
  const columns = PHASES.map((phase) => ({ phase, items: strips.filter((one) => one.phase === phase).sort((a, b) => rank(a) - rank(b)) }));
  // A pull request nobody here owns has only its unit on the rail: the keys reach it after the strips.
  const bare = ship.subjects.filter((one) => one.kind === "pr" && one.pr?.stage !== "landed");
  const order = [...columns.flatMap((column) => column.items), ...bare];
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
      if (here < 0) return false;
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
  const lit = hover ?? ui.selected;
  // A landed pull request is drawn at main only where something here owns it: the history stays in the forge.
  const units = (stage: Stage | "main") =>
    ship.prs.flatMap((pr) => {
      const subject = ship.subjects.find((one) => one.pr?.key === pr.key);
      if (subject === undefined || stageOf(pr) !== stage || (subject.kind === "pr" && pr.stage === "landed")) return [];
      return [<Unit key={pr.key} subject={subject} pr={pr} linked={lit === subject.key} ui={ui} acts={acts} link={link} />];
    });
  const alarm = ship.mains.some((one) => one.state === "red") || ship.subjects.some((one) => one.alarm);
  const waiting = ship.subjects.some((one) => one.needs) || calls.front !== undefined;
  const mainStation = stations.find((one) => one.id === "main")!;
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
        <div className="proto-stage proto-board">
          <div className="proto-strips">
            {columns.map(({ phase, items }) => (
              <section key={phase} className="proto-col" data-phase={phase} aria-label={COLUMN[phase]}>
                <header className="proto-col__band">
                  <span>{COLUMN[phase]}</span>
                </header>
                <ul className="proto-col__list" aria-label={COLUMN[phase]}>
                  {items.map((one) => (
                    <Strip key={one.key} subject={one} selected={current?.key === one.key} linked={lit === one.key} ui={ui} acts={acts} link={link} />
                  ))}
                </ul>
              </section>
            ))}
          </div>
          <section className="proto-rail" aria-label="Merge line">
            <ol className="proto-rail__stages">
              {STAGES.map((stage) => (
                <li key={stage.id} className="proto-rail__stage" data-stage={stage.id}>
                  <span className="proto-rail__band">
                    {stage.said}
                    <ChevronRight size={14} aria-hidden />
                  </span>
                  <ul className="proto-rail__units" aria-label={stage.said}>
                    {units(stage.id)}
                  </ul>
                </li>
              ))}
            </ol>
            <MainMarker main={ship.main} station={mainStation} acts={acts}>
              {units("main")}
            </MainMarker>
          </section>
          {calls.tray}
          {calls.card}
          <KeySheet ui={ui} more={[[["←", "→"], "Next column"]]} />
        </div>
      </section>
    </div>
  );
}
