// What a Job showed, opened from its thread: a file read from the Mac (`/api/sessions/shown`), and a
// sketch drawn from the Job's own record, which already carries its scene.

import { useEffect, useState } from "react";
import { Prose, SketchScene } from "@armada/components";

import { macOf, start, useFleet } from "./fleet";
import { readShown, useJob, type Shown } from "./job";
import { go, jobPath } from "./router";
import { sessionOfThread, sketchOf } from "./thread";
import { Band, Trouble } from "./ui";

const nameOf = (path: string) => path.slice(path.lastIndexOf("/") + 1);

export function FileView({ origin, id, path }: { origin: string; id: string; path: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  useEffect(() => {
    if (machine === undefined) return;
    let gone = false;
    void readShown(machine, id, path).then((read) => !gone && setShown(read));
    return () => {
      gone = true;
      if (shown?.kind === "image") URL.revokeObjectURL(shown.url);
    };
  }, [machine?.origin, id, path]);
  return (
    <section className="pk-screen" aria-label={nameOf(path)}>
      <Band title={nameOf(path)} onBack={() => go(jobPath(origin, id))} back="Back to the Job" />
      <div className="pk-body pk-shown">
        {shown?.kind === "refused" ? <Trouble says={shown.says} /> : null}
        {shown?.kind === "image" ? <img className="pk-shown__image" src={shown.url} alt={nameOf(path)} /> : null}
        {shown?.kind === "text" ? (/\.(md|markdown)$/i.test(path) ? <Prose text={shown.text} /> : <pre className="pk-shown__text">{shown.text}</pre>) : null}
      </div>
    </section>
  );
}

export function SketchView({ origin, id, sketch }: { origin: string; id: string; sketch: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read] = useJob(machine, id);
  const drawn = read.thread === undefined ? undefined : sketchOf(sessionOfThread(read.thread), sketch);
  const title = drawn?.title ?? sketch.replace(/^sketch:/, "");
  return (
    <section className="pk-screen" aria-label={title}>
      <Band title={title} onBack={() => go(jobPath(origin, id))} back="Back to the Job" />
      <div className="pk-body pk-shown">
        {drawn === undefined ? (
          <Trouble says={read.error ?? (read.thread === undefined ? "" : "That Job did not show that sketch.")} />
        ) : (
          <SketchScene scene={drawn.scene} {...(drawn.against === undefined ? {} : { against: drawn.against })} title={drawn.title} />
        )}
      </div>
    </section>
  );
}
