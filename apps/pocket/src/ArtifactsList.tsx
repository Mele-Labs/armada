// A Job's artifacts listed on the phone, as native Bridge's Artifacts tab lists them (`crates/bridge-native/src/job_view/
// artifacts_tab.rs`): one row each, newest first as Fleet serves them, with the archived folded at the foot. A row opens the
// screen the artifact already has, chosen as the thread's own cards choose.

import { useEffect, useState } from "react";
import {
  Archive,
  BadgeCheck,
  Ban,
  BookOpen,
  ChevronDown,
  ChevronRight,
  FileText,
  Layers,
  MessageCircleQuestion,
  Reply,
  ScrollText,
  Undo2,
  type LucideIcon,
} from "lucide-react";
import { Skeleton } from "./kit/components/primitives/Skeleton/Skeleton";
import { Stamp } from "./kit/components/primitives/Stamp/Stamp";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import type { ArtifactForm, ArtifactState, JobArtifact } from "./wire";

import { keptAt, ofParts, useArtifacts } from "./artifacts";
import { macOf, start, useFleet } from "./fleet";
import { artifactPath, documentPath, go, jobPath, sketchPath } from "./router";
import { Band, Trouble } from "./ui";

/** A kind's glyph, its hue and its word, as native's tile has them: a sketch is a step's, a draft a setup's, a document a gate's. */
const KIND: Record<
  ArtifactForm,
  {
    glyph: LucideIcon;
    said: string;
  }
> = {
  sketch: {
    glyph: Layers,
    said: "Sketch",
  },
  draft: {
    glyph: FileText,
    said: "Draft",
  },
  page: {
    glyph: ScrollText,
    said: "Document",
  },
};

/** A state's glyph and the words its tooltip says, which are native's chip words. */
const STATE: Record<
  ArtifactState,
  {
    glyph: LucideIcon;
    said: string;
  }
> = {
  asking: {
    glyph: MessageCircleQuestion,
    said: "Waiting on your answer",
  },
  approved: {
    glyph: BadgeCheck,
    said: "You approved it",
  },
  sent_back: {
    glyph: Reply,
    said: "You sent it back with changes",
  },
  reading: {
    glyph: BookOpen,
    said: "Shown to you, with no answer asked",
  },
  withdrawn: {
    glyph: Undo2,
    said: "The Job took it back",
  },
  dismissed: {
    glyph: Ban,
    said: "You passed on it",
  },
};

/** Where a row opens: the artifact's own screen where it holds parts, else its sketch or its document, else the thread that shows the draft. */
export function openPathOf(origin: string, id: string, one: JobArtifact): string {
  if (ofParts(one)) return artifactPath(origin, id, one.id);
  if (one.form === "page") return documentPath(origin, id, one.id);
  if (one.form === "sketch") return sketchPath(origin, id, one.id);
  return jobPath(origin, id);
}

function Row({ one, folded, onOpen }: { one: JobArtifact; folded: boolean; onOpen: () => void }) {
  const kind = KIND[one.form];
  const state = STATE[one.state];
  const said = [
    folded ? "Archived" : undefined,
    state.said,
  ]
    .filter((part) => part !== undefined)
    .join(". ");
  const tip = one.words === undefined || one.words.trim() === "" ? said : `${said}: ${one.words}`;
  const line = one.for_line?.text;
  const KindGlyph = kind.glyph;
  const StateGlyph = state.glyph;
  return (
    <li
      className="pk-art-row"
      data-form={one.form}
      data-state={one.state}
      data-folded={folded || undefined}
    >
      <button
        type="button"
        className="pk-art-row__open"
        aria-label={`Open ${kind.said.toLowerCase()} ${one.title}`}
        onClick={onOpen}
      >
        <Tooltip label={kind.said}>
          <span className="pk-art-row__kind" role="img" aria-label={kind.said}>
            <KindGlyph size={16} aria-hidden="true" />
          </span>
        </Tooltip>
        <span className="pk-art-row__text">
          <span className="pk-art-row__title">{one.title}</span>
          {line === undefined || line.trim() === "" ? null : (
            <span className="pk-art-row__for">{line}</span>
          )}
        </span>
        <span className="pk-art-row__end">
          <Tooltip label="Kept">
            <span className="pk-art-row__time">
              <Stamp still first="age" at={keptAt(one)} />
            </span>
          </Tooltip>
          <Tooltip label={tip}>
            <span className="pk-art-row__state" role="img" aria-label={tip}>
              <StateGlyph size={16} aria-hidden="true" />
            </span>
          </Tooltip>
        </span>
      </button>
    </li>
  );
}

function Shapes() {
  return (
    <ul className="pk-art-list" role="status" aria-label="Loading">
      {[
        "55%",
        "80%",
        "65%",
      ].map((width) => (
        <li key={width} className="pk-art-row pk-art-row--shape">
          <span className="pk-art-row__kind" />
          <span className="pk-art-row__text">
            <Skeleton width={width} />
            <Skeleton width="35%" />
          </span>
        </li>
      ))}
    </ul>
  );
}

export function ArtifactsView({ origin, id }: { origin: string; id: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read] = useArtifacts(machine, id);
  const [unfolded, setUnfolded] = useState(false);
  const live = (read.artifacts ?? []).filter((one) => one.archived === undefined);
  const archived = (read.artifacts ?? []).filter((one) => one.archived !== undefined);
  const back = () => go(jobPath(origin, id));
  return (
    <section className="pk-screen" aria-label="Artifacts">
      <Band title="Artifacts" onBack={back} back="Back to the Job" />
      <div className="pk-body pk-art-body">
        {read.artifacts === undefined && read.error !== undefined ? (
          <Trouble says={read.error} />
        ) : null}
        {read.artifacts === undefined && read.error === undefined ? <Shapes /> : null}
        {read.artifacts !== undefined && live.length === 0 && archived.length === 0 ? (
          <p className="pk-art-empty">This Job has shown nothing yet</p>
        ) : null}
        {live.length === 0 ? null : (
          <ul className="pk-art-list" aria-label="Artifacts">
            {live.map((one) => (
              <Row
                key={one.id}
                one={one}
                folded={false}
                onOpen={() => go(openPathOf(origin, id, one))}
              />
            ))}
          </ul>
        )}
        {archived.length === 0 ? null : (
          <>
            <Tooltip label={unfolded ? "Fold the archived away" : "Show the archived"}>
              <button
                type="button"
                className="pk-art-fold"
                aria-expanded={unfolded}
                onClick={() => setUnfolded((was) => !was)}
              >
                {unfolded ? (
                  <ChevronDown size={14} aria-hidden="true" />
                ) : (
                  <ChevronRight size={14} aria-hidden="true" />
                )}
                <Archive size={14} aria-hidden="true" />
                Archived
              </button>
            </Tooltip>
            {unfolded ? (
              <ul className="pk-art-list" aria-label="Archived">
                {archived.map((one) => (
                  <Row
                    key={one.id}
                    one={one}
                    folded
                    onOpen={() => go(openPathOf(origin, id, one))}
                  />
                ))}
              </ul>
            ) : null}
          </>
        )}
      </div>
    </section>
  );
}
