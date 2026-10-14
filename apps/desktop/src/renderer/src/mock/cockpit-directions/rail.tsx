// The merge line, the one real pipeline: pull requests only, left to right through Open, Checking and In queue,
// to the main marker. A unit is joined to its owner's strip or row by hover and selection, and Merge and Fix act
// on it. Shared by the strips board's rail and the wall's merge station. Mock only.
// proto/cockpit-three-directions.

import type { ReactNode } from "react";
import { ChevronRight, Hammer } from "lucide-react";
import { Button, Kbd } from "@armada/components";

import { FixAct, KEYS, MarkGlyph, PrChip, StationMark, type Acts, type ShipUi } from "./shared";
import { stationsOf, type Main, type Pr, type Ship, type Station, type Subject } from "./ship";

/** The stage of the one pipeline a pull request is at. */
export type Stage = "open" | "checking" | "queue";
const STAGES: readonly { id: Stage; said: string }[] = [
  { id: "open", said: "Open" },
  { id: "checking", said: "Checking" },
  { id: "queue", said: "In queue" },
];
const stageOf = (pr: Pr): Stage | "main" => (pr.stage === "landed" ? "main" : pr.stage === "queued" || pr.stage === "ready" ? "queue" : pr.ci === "waiting_on_main" ? "open" : "checking");

export type Link = { setHover: (key: string | undefined) => void };

/** A pull request on the rail. Joined to its owner: hover or select either and both light. Merge acts here. */
export function Unit({ subject, pr, linked, ui, acts, link }: { subject: Subject; pr: Pr; linked: boolean; ui: ShipUi; acts: Acts; link: Link }) {
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
export function MainMarker({ main, station, acts, children }: { main: Main | undefined; station: Station; acts: Acts; children: ReactNode }) {
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


/** The whole rail: three stages of units, then main with the pull requests that landed. */
export function Rail({ ship, ui, acts, lit, link }: { ship: Ship; ui: ShipUi; acts: Acts; lit: string | undefined; link: Link }) {
  // A landed pull request is drawn at main only where something here owns it: the history stays in the forge.
  const units = (stage: Stage | "main") =>
    ship.prs.flatMap((pr) => {
      const subject = ship.subjects.find((one) => one.pr?.key === pr.key);
      if (subject === undefined || stageOf(pr) !== stage || (subject.kind === "pr" && pr.stage === "landed")) return [];
      return [<Unit key={pr.key} subject={subject} pr={pr} linked={lit === subject.key} ui={ui} acts={acts} link={link} />];
    });
  const station = stationsOf(ship).find((one) => one.id === "main")!;
  return (
    <>
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
      <MainMarker main={ship.main} station={station} acts={acts}>
        {units("main")}
      </MainMarker>
    </>
  );
}
