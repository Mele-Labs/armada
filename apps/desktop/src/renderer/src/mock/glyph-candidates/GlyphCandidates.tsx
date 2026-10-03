// The candidate sheet the `glyphCandidates` walk plays over: for each meaning,
// its candidates side by side, each drawn where it would sit — on a criterion
// row as Overview's approval panel and Plan draw one, or on a proposing Job's
// row. SCRATCH, mounted by `main.tsx` on `?scenario=glyph-candidates` in place
// of the app; deleted once the owner has picked.
//
// **Drawn with the surfaces' own classes**, so a candidate is judged at the
// size, colour and spacing it would have there. No real surface imports any of
// this, and no glyph here is wired into one.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Card, CardContent, JOB_STATUS, JobRowStacked } from "@armada/components";
import { ScanLine, ScrollText } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import "../../styles/index.css";
import "./glyph-candidates.css";
import { MEANINGS } from "./candidates";
import type { Candidate, Meaning } from "./candidates";

/** The scenario name `main.tsx` mounts this sheet on. */
export const GLYPH_CANDIDATES = "glyph-candidates";

const GLYPH = 12;
const STROKE = 2;

const ISSUE_REF = "armada#1162";
/** A line for each origin, so a list of three reads as three. */
const CRITERION: Record<"issue" | "prompt" | "person", string> = {
  issue: "The rail's Drones stat reads one running while a Drone works",
  prompt: "Hovering the stat names each running Drone",
  person: "Nothing else on the rail moves",
};
const REQUEST = "Make the stat say what is running";

/** A 12px glyph that names itself, the way a bare mark is labelled in Bridge. */
function Mark({ glyph: Glyph, label, motion }: { glyph: LucideIcon; label: string; motion?: Candidate["motion"] }) {
  return (
    <span className="gc-mark" role="img" aria-label={label} title={label} data-motion={motion}>
      <Glyph size={GLYPH} strokeWidth={STROKE} aria-hidden />
    </span>
  );
}

/** Where the words came from, as `originSaidOf` says it for each origin. */
const ORIGIN_SAID: Record<"issue" | "prompt" | "person", string> = {
  issue: "From issue",
  prompt: "From your prompt",
  person: "You wrote this",
};

function OriginWords({ origin }: { origin: "issue" | "prompt" | "person" }) {
  return origin === "issue" ? (
    <span>
      {ORIGIN_SAID.issue} <span className="armada-proposal__criterion-ref">{ISSUE_REF}</span>
    </span>
  ) : (
    <span>{ORIGIN_SAID[origin]}</span>
  );
}

/** One criterion on Overview's approval panel, frozen. */
function ApprovalRow({
  origin,
  originMark,
  movedMark,
}: {
  origin: "issue" | "prompt" | "person";
  originMark?: Candidate;
  movedMark?: Candidate;
}) {
  return (
    <li className="armada-proposal__criterion">
      <div className="armada-proposal__criterion-text">{CRITERION[origin]}</div>
      <p className="armada-proposal__criterion-origin">
        {originMark === undefined ? null : <Mark glyph={originMark.glyph} label={ORIGIN_SAID[origin]} />}
        <OriginWords origin={origin} />
        <span className="armada-proposal__criterion-dot" aria-hidden="true">
          ·
        </span>
        <span>The Judge will decide it</span>
      </p>
      {movedMark === undefined ? null : (
        <p className="armada-proposal__moved gc-moved" role="note">
          <Mark glyph={movedMark.glyph} label="The issue has moved since" />
          <span>
            The issue has been edited since these words were frozen — last on 1 Oct. The Job is held to the words
            above.
          </span>
        </p>
      )}
    </li>
  );
}

/** The same criterion on Plan's "What this Job is held to". */
function PlanRow({
  origin,
  originMark,
  movedMark,
}: {
  origin: "issue" | "prompt" | "person";
  originMark?: Candidate;
  movedMark?: Candidate;
}) {
  const said = origin === "issue" ? `${ORIGIN_SAID.issue} ${ISSUE_REF}` : ORIGIN_SAID[origin];
  return (
    <li>
      <span className="armada-plan-tab__criterion-text">{CRITERION[origin]}</span>
      <span className="armada-plan-tab__criterion-origin gc-line">
        {originMark === undefined ? null : <Mark glyph={originMark.glyph} label={ORIGIN_SAID[origin]} />}
        <span>{`${said} · The Judge will decide it`}</span>
      </span>
      {movedMark === undefined ? null : (
        <span className="armada-plan-tab__criterion-moved gc-line">
          <Mark glyph={movedMark.glyph} label="The issue has moved since" />
          <span>The issue has been edited since these words were frozen.</span>
        </span>
      )}
    </li>
  );
}

/** A criterion drawn on both surfaces, with this candidate in its place. */
function OnACriterion({ meaning, candidate }: { meaning: Meaning; candidate: Candidate }) {
  const origin = meaning.id === "prompt" || meaning.id === "person" ? meaning.id : "issue";
  const marks = meaning.where === "moved" ? { movedMark: candidate } : { originMark: candidate };
  return (
    <div className="gc-surfaces">
      <p className="gc-surface">Overview, approval panel</p>
      <Card>
        <CardContent>
          <ul className="armada-proposal__criteria">
            <ApprovalRow origin={origin} {...marks} />
          </ul>
        </CardContent>
      </Card>
      <p className="gc-surface">Plan</p>
      <Card>
        <CardContent>
          <ul className="armada-plan-tab__criteria">
            <PlanRow origin={origin} {...marks} />
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}

const PROPOSING = JOB_STATUS["proposing"];

/** A dispatched request's Board row, its title and workflow not settled yet. */
function OnAProposingRow({ candidate }: { candidate: Candidate }) {
  const settling = (field: string) => (
    <Mark glyph={candidate.glyph} label={`${field}, still being settled`} {...(candidate.motion === undefined ? {} : { motion: candidate.motion })} />
  );
  return (
    <div className="gc-surfaces">
      <p className="gc-surface">Board, a proposing row</p>
      <div role="list" aria-label={`Proposing row, ${candidate.name}`}>
        <JobRowStacked
          view="card"
          status={PROPOSING?.badgeStatus ?? "running"}
          statusIcon={PROPOSING?.icon ?? ScanLine}
          statusLabel={PROPOSING?.verb ?? "proposing"}
          headline={
            <span className="gc-line">
              <span>{REQUEST}</span>
              {settling("Title")}
            </span>
          }
          handle="J-218"
          fields={[
            { label: "Workflow", icon: ScrollText, value: settling("Workflow") },
            { value: undefined },
            { value: undefined },
            { label: "Dispatched by", value: "You" },
          ]}
        />
      </div>
    </div>
  );
}

function CandidateCard({ meaning, candidate }: { meaning: Meaning; candidate: Candidate }) {
  return (
    <article className="gc-candidate" aria-label={`${meaning.says}: ${candidate.name}`}>
      <header className="gc-candidate__head">
        <span className="gc-candidate__bare">
          <candidate.glyph size={16} strokeWidth={STROKE} aria-hidden />
        </span>
        <h3 className="gc-candidate__name">{candidate.name}</h3>
        {candidate.motion === undefined ? null : <span className="gc-candidate__motion">{candidate.motion}</span>}
      </header>
      {meaning.where === "row" ? (
        <OnAProposingRow candidate={candidate} />
      ) : (
        <OnACriterion meaning={meaning} candidate={candidate} />
      )}
      <dl className="gc-entry">
        <dt>means</dt>
        <dd>{candidate.means}</dd>
        <dt>reserved</dt>
        <dd>{candidate.reserved}</dd>
        <dt>notes</dt>
        <dd>{candidate.notes}</dd>
      </dl>
    </article>
  );
}

function MeaningSection({ meaning, at }: { meaning: Meaning; at: number }) {
  return (
    <section className="gc-meaning" aria-label={meaning.says} data-where={meaning.where}>
      <h2 className="gc-meaning__title">{`${at + 1}. ${meaning.says}`}</h2>
      <div className="gc-meaning__candidates">
        {meaning.candidates.map((candidate) => (
          <CandidateCard key={candidate.name} meaning={meaning} candidate={candidate} />
        ))}
      </div>
    </section>
  );
}

/** The recommended origin and moved marks on one list, as they would meet. */
function Together() {
  const pick = (id: Meaning["id"]) => {
    const meaning = MEANINGS.find((one) => one.id === id)!;
    return meaning.candidates.find((one) => one.name === meaning.recommended)!;
  };
  return (
    <section className="gc-meaning" aria-label="Together">
      <h2 className="gc-meaning__title">Together, with the recommended marks: one of each origin, and a moved issue</h2>
      <div className="gc-together">
        <Card>
          <CardContent>
            <ul className="armada-proposal__criteria">
              <ApprovalRow origin="issue" originMark={pick("issue")} movedMark={pick("moved")} />
              <ApprovalRow origin="prompt" originMark={pick("prompt")} />
              <ApprovalRow origin="person" originMark={pick("person")} />
            </ul>
          </CardContent>
        </Card>
        <Card>
          <CardContent>
            <ul className="armada-plan-tab__criteria">
              <PlanRow origin="issue" originMark={pick("issue")} movedMark={pick("moved")} />
              <PlanRow origin="prompt" originMark={pick("prompt")} />
              <PlanRow origin="person" originMark={pick("person")} />
            </ul>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}

export function GlyphCandidates() {
  return (
    <main className="gc-sheet" aria-label="Glyph candidates">
      <h1 className="gc-sheet__title">Glyph candidates</h1>
      {MEANINGS.map((meaning, at) => (
        <MeaningSection key={meaning.id} meaning={meaning} at={at} />
      ))}
      <Together />
    </main>
  );
}

export function mountGlyphCandidates(host: HTMLElement): void {
  createRoot(host).render(
    <StrictMode>
      <GlyphCandidates />
    </StrictMode>,
  );
}
