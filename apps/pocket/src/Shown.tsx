// What a Job showed, opened from its thread: a file read from the Mac (`/api/sessions/shown`), and a
// sketch drawn from the Job's own record, which already carries its scene.

import { useEffect, useState, type CSSProperties } from "react";
import {
  mediaKindOf,
  ShownVideo,
  type ShownMedia,
} from "./kit/components/compositions/MediaRow/MediaRow";
import { Prose } from "./kit/components/primitives/Prose/Prose";
import { aspectOf } from "./kit/components/compositions/SketchScene/aspect";
import { SketchScene } from "./kit/components/compositions/SketchScene/SketchScene";
import { ZoomablePicture } from "./kit/components/primitives/Enlargeable/ZoomablePicture";

import { macOf, start, useFleet } from "./fleet";
import {
  actsOn,
  attempt,
  attemptPage,
  readShown,
  readShownMedia,
  useJob,
  type Shown,
} from "./jobState";
import { go, jobPath } from "./router";
import { sessionOfThread, sketchOf } from "./thread";
import { WithKeptStyles } from "./keptStyles";
import { Band, Trouble } from "./ui";
import { revisionSaid, type BandFacts } from "./kit/components/compositions/BandCard/BandCard";
import type { SketchAnswer } from "./kit/components/compositions/SessionThread/sketch-answer";
import { sketchAnswerOf } from "./kit/screens/thread-cards";
import { useArtifacts } from "./artifacts";
import { Reader } from "./Reader";

const nameOf = (path: string) => path.slice(path.lastIndexOf("/") + 1);

export function FileView({ origin, id, path }: { origin: string; id: string; path: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [shown, setShown] = useState<Shown | ShownMedia | undefined>(undefined);
  useEffect(() => {
    if (machine === undefined) return;
    let gone = false;
    // Held here and not read from `shown`, which this cleanup would see as it was when the effect ran.
    let made: string | undefined;
    void (mediaKindOf(path) === "video" ? readShownMedia : readShown)(machine, id, path).then(
      (read) => {
        const blob = read.kind === "image" ? read.url : undefined;
        if (gone) {
          if (blob !== undefined) URL.revokeObjectURL(blob);
          return;
        }
        made = blob;
        setShown(read);
      },
    );
    return () => {
      gone = true;
      if (made !== undefined) URL.revokeObjectURL(made);
    };
  }, [
    machine?.origin,
    id,
    path,
  ]);
  return (
    <section className="pk-screen" aria-label={nameOf(path)}>
      <Band title={nameOf(path)} onBack={() => go(jobPath(origin, id))} back="Back to the Job" />
      <div
        className={
          shown?.kind === "image" || shown?.kind === "video"
            ? "pk-body pk-shown pk-shown--media"
            : "pk-body pk-shown"
        }
      >
        {shown?.kind === "refused" ? <Trouble says={shown.says} /> : null}
        {shown?.kind === "image" ? (
          <ZoomablePicture className="pk-shown__zoom" src={shown.url} alt={nameOf(path)} />
        ) : null}
        {shown?.kind === "video" ? <ShownVideo media={shown} name={nameOf(path)} /> : null}
        {shown?.kind === "text" ? (
          /\.(md|markdown)$/i.test(path) ? (
            <Prose text={shown.text} />
          ) : (
            <pre className="pk-shown__text">{shown.text}</pre>
          )
        ) : null}
      </div>
    </section>
  );
}

export function SketchView({ origin, id, sketch }: { origin: string; id: string; sketch: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read, reread] = useJob(machine, id);
  const [listed] = useArtifacts(machine, id);
  const [says, setSays] = useState("");
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const drawn = session === undefined ? undefined : sketchOf(session, sketch);
  const artifact = listed.artifacts?.find((one) => one.id === sketch);
  const title = drawn?.title ?? sketch.replace(/^sketch:/, "");
  const last = artifact?.revisions[artifact.revisions.length - 1];
  const facts: BandFacts = {
    form: "sketch",
    title,
    stand: artifact?.state ?? "reading",
    words: artifact?.words,
    ...(last === undefined
      ? {}
      : {
          revision: {
            number: last.number,
            said: revisionSaid(last.number, last.by, last.at),
          },
        }),
    ...(last?.by === "owner"
      ? {
          yours: `r${String(last.number)} is your edit`,
        }
      : {}),
    forLine: artifact?.for_line,
  };
  const answer = (given: SketchAnswer) => {
    if (machine === undefined || session === undefined) return;
    setSays("");
    void attemptPage(
      () => actsOn(machine, reread).answerSketch(sketchAnswerOf(session, sketch, given)),
      machine,
      reread,
    ).then((refused) => setSays(refused ?? ""));
  };
  const dismiss = () => {
    if (machine === undefined) return;
    setSays("");
    void attempt(
      () =>
        actsOn(machine, reread).dismissWaiting({
          session_id: id,
          item_id: artifact?.item ?? sketch,
        }),
      machine,
    ).then((refused) => setSays(refused ?? ""));
  };
  return (
    <Reader
      facts={facts}
      keep={`pocket:${origin}:${id}:sketch:${sketch}`}
      onBack={() => go(jobPath(origin, id))}
      {...(artifact?.state === "asking"
        ? {
            acts: {
              approve: () =>
                answer({
                  choice: 0,
                }),
              ask: (note: string) =>
                answer({
                  text: note,
                }),
              dismiss,
            },
          }
        : {})}
      says={says === "" ? undefined : says}
    >
      {drawn === undefined ? (
        <Trouble
          says={
            read.error ?? (read.thread === undefined ? "" : "That Job did not show that sketch.")
          }
        />
      ) : (
        <WithKeptStyles machine={machine}>
          <div
            className="pk-reader__sketch"
            style={
              {
                "--aspect": aspectOf(drawn.scene),
              } as CSSProperties
            }
          >
            <SketchScene
              scene={drawn.scene}
              {...(drawn.against === undefined
                ? {}
                : {
                    against: drawn.against,
                  })}
              title={drawn.title}
            />
          </div>
        </WithKeptStyles>
      )}
    </Reader>
  );
}
