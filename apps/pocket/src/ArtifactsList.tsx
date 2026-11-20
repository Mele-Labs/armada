// A Job's artifacts listed on the phone, as native Bridge's Artifacts tab lists them, in the Rows look (`crates/bridge-native/src/
// job_view/artifacts_tab.rs`): one row each, newest first as Fleet serves them, with the archived folded at the foot. A row opens the
// screen the artifact already has, chosen as the thread's own cards choose.

import { useEffect, useState } from "react";
import { Archive, ChevronDown, ChevronRight } from "lucide-react";
import {
  KindTile,
  RevisionChip,
  revisionSaid,
  StateChip,
  type Form,
} from "./kit/components/compositions/BandCard/BandCard";
import { Skeleton } from "./kit/components/primitives/Skeleton/Skeleton";
import { Stamp } from "./kit/components/primitives/Stamp/Stamp";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import type { JobArtifact } from "./wire";

import { keptAt, ofParts, useArtifacts } from "./artifacts";
import { macOf, start, useFleet } from "./fleet";
import { artifactPath, documentPath, go, jobPath, sketchPath } from "./router";
import { Band, Trouble } from "./ui";

const KIND_WORD: Record<Form, string> = {
  sketch: "sketch",
  draft: "draft",
  page: "page",
};

/** Where a row opens: the artifact's own screen where it holds parts, else its sketch or its document, else the screen that reads the draft. */
export function openPathOf(origin: string, id: string, one: JobArtifact): string {
  if (ofParts(one)) return artifactPath(origin, id, one.id);
  if (one.form === "page") return documentPath(origin, id, one.id);
  if (one.form === "sketch") return sketchPath(origin, id, one.id);
  return artifactPath(origin, id, one.id);
}

/** One artifact: its kind's tile beside its title, what it is for, and a line of where it stands, its revision and when it was kept. */
function Row({ one, folded, onOpen }: { one: JobArtifact; folded: boolean; onOpen: () => void }) {
  const last = one.revisions[one.revisions.length - 1];
  const line = one.for_line?.text?.trim();
  return (
    <li className="pk-art-row" data-folded={folded || undefined}>
      <button
        type="button"
        className="pk-art-row__open"
        aria-label={`Open ${KIND_WORD[one.form]} ${one.title}`}
        onClick={onOpen}
      >
        <KindTile form={one.form} />
        <span className="pk-art-row__text">
          <span className="pk-art-row__title">{one.title}</span>
          {line === undefined || line === "" ? null : (
            <span className="pk-art-row__for">{line}</span>
          )}
          <span className="pk-art-row__meta">
            <StateChip stand={one.state} words={one.words} />
            {last === undefined ? null : (
              <RevisionChip
                number={last.number}
                said={revisionSaid(last.number, last.by, last.at)}
              />
            )}
            <Tooltip label="Kept">
              <span className="pk-art-row__time">
                <Stamp still first="age" at={keptAt(one)} />
              </span>
            </Tooltip>
          </span>
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
          <span className="pk-bc__tile pk-art-row__shape" />
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
