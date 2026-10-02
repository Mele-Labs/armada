import { createContext, useContext, useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

import { Prose } from "../../primitives/Prose/Prose";
import { DestinationCard } from "../DestinationCard/DestinationCard";
import { ProposalField, ProposalFields } from "../JobProposal/ProposalFields";
import type { VerdictFigure, VerdictSheetProps } from "./VerdictSheet";

/**
 * Three arrangements of the review gate's **The Job's record**, for the owner
 * to pick one from — #1680, 2 Oct 2026. **A design round, not the build**:
 * nothing draws these unless a mock scenario asks for one through
 * `RecordArrangementsFrom`, and the app's own mount provides nothing, so
 * `VerdictSheet` draws the record as it always has.
 *
 * Every arrangement takes the same props the record already takes, so the
 * three differ in arrangement alone and never in what they were given.
 */
export type RecordArrangement =
  /** A: each section its own `DestinationCard`, in the Settings board's balanced columns. */
  | "cards"
  /** B: one glass card, each section a row whose value leads and whose name trails it. */
  | "rows"
  /** C: the pull request, its figures and its proof lead; the Drone's account folds inside. */
  | "work-first";

/** What a scenario asks of one Job's record. */
export type RecordAsked = {
  arrangement: RecordArrangement;
  /** Opened on arrival, so a comparison can be seen without a press. The fold is unchanged. */
  open?: boolean;
};

/** Each Job's asked arrangement, by its id. Empty is the app on a real Fleet. */
const Arrangements = createContext<Readonly<Record<string, RecordAsked>>>({});

/** Hand the window each Job's record arrangement. The mock's own mount is the one caller. */
export function RecordArrangementsFrom({
  asked,
  children,
}: {
  asked: Readonly<Record<string, RecordAsked>>;
  children: ReactNode;
}) {
  return <Arrangements.Provider value={asked}>{children}</Arrangements.Provider>;
}

/** What was asked of this Job's record, or `undefined` for the record as it is. */
export function useRecordAsked(jobId: string | undefined): RecordAsked | undefined {
  const asked = useContext(Arrangements);
  return jobId === undefined ? undefined : asked[jobId];
}

/** The record's own props — everything `VerdictSheet` draws inside the fold. */
export type RecordProps = Pick<
  VerdictSheetProps,
  | "title"
  | "brief"
  | "criteria"
  | "criteriaAbsent"
  | "cameBack"
  | "deliverable"
  | "pullRequest"
  | "provesIt"
  | "provesItNote"
  | "risks"
  | "leftAlone"
  | "figures"
>;

/** The record folded behind one line, in `arrangement`. Folding is the 29 Sep decision, unchanged. */
export function ArrangedRecord({
  asked,
  record,
}: {
  asked: RecordAsked;
  record: RecordProps;
}) {
  const [open, setOpen] = useState(asked.open === true);
  const fold = <Fold open={open} onToggle={() => setOpen((was) => !was)} label="The Job's record" />;
  // **A's cards sit beside the fold, not inside it**: a card inside the glass
  // the fold is drawn on is a card in a card, which `glass.css` refuses.
  if (asked.arrangement === "cards") {
    return (
      <>
        <div className="armada-verdict__record-card armada-glass">{fold}</div>
        <div className="armada-record-cards" hidden={!open}>
          <Cards {...record} />
        </div>
      </>
    );
  }
  return (
    <div className="armada-verdict__record-card armada-glass">
      {fold}
      <div className="armada-verdict__record-body" hidden={!open}>
        {asked.arrangement === "rows" ? <Rows {...record} /> : <WorkFirst {...record} />}
      </div>
    </div>
  );
}

/** One line that opens what is under it — the fold's own control. */
function Fold({ open, onToggle, label }: { open: boolean; onToggle: () => void; label: string }) {
  const Mark = open ? ChevronDown : ChevronRight;
  return (
    <button type="button" className="armada-verdict__fold" aria-expanded={open} onClick={onToggle}>
      <Mark size={12} aria-hidden="true" />
      <span className="armada-verdict__label">{label}</span>
    </button>
  );
}

/** What was asked for: the title, Fleet's brief and the criteria. Shared, because the content is. */
function AskedFor({ title, brief, criteria, criteriaAbsent }: RecordProps) {
  return (
    <>
      <p className="armada-verdict__lede">{title}</p>
      {criteria.length === 0 ? (
        criteriaAbsent === undefined ? null : <p className="armada-verdict__said">{criteriaAbsent}</p>
      ) : (
        <ul className="armada-verdict__criteria">
          {criteria.map((one, i) => (
            <li key={i}>{one}</li>
          ))}
        </ul>
      )}
      {brief === undefined ? null : (
        <div className="armada-verdict__said">
          <Prose text={brief} />
        </div>
      )}
    </>
  );
}

function Said({ text }: { text: string }) {
  return (
    <div className="armada-verdict__said">
      <Prose text={text} />
    </div>
  );
}

function Proof({ provesIt, provesItNote }: RecordProps) {
  return (
    <>
      {provesIt}
      {provesItNote === undefined ? null : <p className="armada-verdict__said">{provesItNote}</p>}
    </>
  );
}

const figureValue = (figure: VerdictFigure) => (figure.value === undefined ? figure.absent : figure.value);

// ---------------------------------------------------------------------------
// A. Settings-board cards
// ---------------------------------------------------------------------------

/**
 * **A: one card per section, in the Settings board's columns.** The pull
 * request's card carries the figures as Settings carries a cap: the name
 * outside, the value in a box.
 *
 * Trades height for scanning: every section is a head a person can find, and
 * the brief's card is as tall as the issue it quotes.
 */
function Cards(record: RecordProps) {
  const { cameBack, deliverable, pullRequest, risks, leftAlone, figures } = record;
  return (
    <>
      <DestinationCard label="What you asked for">
        <AskedFor {...record} />
      </DestinationCard>
      <DestinationCard label="The work">
        {pullRequest}
        <ProposalFields>
          {figures.map((figure) => (
            <ProposalField key={figure.label} label={figure.label}>
              <span className="armada-record__figure" data-mono={figure.mono || undefined}>
                {figureValue(figure)}
              </span>
            </ProposalField>
          ))}
        </ProposalFields>
      </DestinationCard>
      <DestinationCard label="What proves it">
        <Proof {...record} />
      </DestinationCard>
      {risks === undefined ? null : (
        <DestinationCard label="What nothing checked">
          <Said text={risks} />
        </DestinationCard>
      )}
      <DestinationCard label="What the Drone says it did">
        <Said text={cameBack} />
        {deliverable === undefined ? null : <div className="armada-verdict__document">{deliverable}</div>}
      </DestinationCard>
      <DestinationCard label="What the Drone says it left alone">
        <Said text={leftAlone} />
      </DestinationCard>
    </>
  );
}

// ---------------------------------------------------------------------------
// B. One glass card, structured rows
// ---------------------------------------------------------------------------

/**
 * **B: one card, a row per section, the value first.** The approving panel's
 * grammar on Overview (`approving.tsx`): what the row says at full weight, and
 * what kind of thing it is in a quieter name at the row's trailing edge.
 *
 * Trades a heading per section for one continuous read: nothing is boxed, so
 * it is the shortest of the three open, and the names are the easiest to miss.
 */
function Rows(record: RecordProps) {
  const { cameBack, deliverable, pullRequest, risks, leftAlone, figures } = record;
  return (
    <dl className="armada-record-rows">
      <Row name="What you asked for">
        <AskedFor {...record} />
      </Row>
      {pullRequest === undefined ? null : <Row name="The pull request">{pullRequest}</Row>}
      <Row name="The figures">
        <ul className="armada-record-rows__figures">
          {figures.map((figure) => (
            <li className="armada-record-rows__figure" key={figure.label}>
              <span className="armada-record__figure" data-mono={figure.mono || undefined}>
                {figureValue(figure)}
              </span>
              <span className="armada-record-rows__figure-name">{figure.label}</span>
            </li>
          ))}
        </ul>
      </Row>
      <Row name="What the Drone says it did">
        <Said text={cameBack} />
        {deliverable === undefined ? null : <div className="armada-verdict__document">{deliverable}</div>}
      </Row>
      <Row name="What proves it">
        <Proof {...record} />
      </Row>
      {risks === undefined ? null : (
        <Row name="What nothing checked">
          <Said text={risks} />
        </Row>
      )}
      <Row name="What the Drone says it left alone">
        <Said text={leftAlone} />
      </Row>
    </dl>
  );
}

/** One row of B. `dd` before `dt` in the drawing only; the list reads name then value. */
function Row({ name, children }: { name: string; children: ReactNode }) {
  return (
    <div className="armada-record-rows__row">
      <dt className="armada-record-rows__name">{name}</dt>
      <dd className="armada-record-rows__value">{children}</dd>
    </div>
  );
}

// ---------------------------------------------------------------------------
// C. The work leads, the account folds
// ---------------------------------------------------------------------------

/**
 * **C: what a person decides on first, and the Drone's account folded under
 * it.** The pull request and its figures head the card; what proves it and what
 * nothing checked sit side by side, so the evidence and its gap are read as a
 * pair; what was asked for, what the Drone says it did and what it left alone
 * each fold behind their own line.
 *
 * Trades the Drone's own words for the decision material: the claim is one
 * more press away, and a second level of folding lives inside the first.
 */
function WorkFirst(record: RecordProps) {
  const { cameBack, deliverable, pullRequest, risks, leftAlone, figures } = record;
  return (
    <div className="armada-record-work">
      <div className="armada-record-work__head">
        {pullRequest}
        <ul className="armada-verdict__figures armada-record-work__figures">
          {figures.map((figure) => (
            <li className="armada-verdict__figure" key={figure.label}>
              <span className="armada-verdict__figure-label">{figure.label}</span>
              <span className="armada-verdict__figure-value" data-mono={figure.mono || undefined}>
                {figureValue(figure)}
              </span>
            </li>
          ))}
        </ul>
      </div>
      <div className="armada-record-work__pair">
        <section className="armada-verdict__block" aria-label="What proves it">
          <span className="armada-verdict__label">What proves it</span>
          <Proof {...record} />
        </section>
        {risks === undefined ? null : (
          <section className="armada-verdict__block" aria-label="What nothing checked">
            <span className="armada-verdict__label">What nothing checked</span>
            <Said text={risks} />
          </section>
        )}
      </div>
      <div className="armada-record-work__account">
        <Folded label="What you asked for">
          <AskedFor {...record} />
        </Folded>
        <Folded label="What the Drone says it did">
          <Said text={cameBack} />
          {deliverable === undefined ? null : <div className="armada-verdict__document">{deliverable}</div>}
        </Folded>
        <Folded label="What the Drone says it left alone">
          <Said text={leftAlone} />
        </Folded>
      </div>
    </div>
  );
}

/** One of C's inner folds. `hidden`, not unmounted, on the record's own rule. */
function Folded({ label, children }: { label: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="armada-record-work__fold">
      <Fold open={open} onToggle={() => setOpen((was) => !was)} label={label} />
      <div className="armada-verdict__block armada-record-work__folded" hidden={!open}>
        {children}
      </div>
    </div>
  );
}
