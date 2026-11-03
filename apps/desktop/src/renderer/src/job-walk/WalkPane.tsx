// One walk drawn in whatever holds it: a tab of the Job's panes, or a slot of the review sheet. It assumes nothing of what is around it. On a real Bridge the walk's page and its bar are
// views main lays over this frame (`main/capture/job-walks.ts`), so the pane is an empty box that reports where it is; the mock has no such views and draws `WalkView` with a fixture.

import { useEffect, useRef, useState } from "react";

import { holdLayerUp } from "../hold-layers";
import { useSessions } from "../sessions-draft";
import { useSendHeld } from "./held";
import { mockPage } from "./mock-pages";
import { approveWalk, goToStep, holdNote, pickTake, useHeldNotes, useWalkState } from "./store";
import { walksOf } from "./walks";
import { WalkView } from "./WalkView";

type Box = {
  x: number;
  y: number;
  width: number;
  height: number;
};
type Live = NonNullable<Window["armada"]["jobWalk"]>;

const sameBox = (a: Box | undefined, b: Box): boolean =>
  a !== undefined && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

const pictureOf = (png: Uint8Array): string =>
  URL.createObjectURL(
    new Blob(
      [
        png as Uint8Array<ArrayBuffer>,
      ],
      {
        type: "image/png",
      },
    ),
  );

export function WalkPane({ jobId, walkId }: { jobId: string; walkId: string }) {
  const live = window.armada?.jobWalk;
  return live === undefined ? (
    <MockWalk jobId={jobId} walkId={walkId} />
  ) : (
    <LiveWalk jobId={jobId} walkId={walkId} api={live} />
  );
}

/**
 * The frame is reported on every frame it moves, as the browser pane's is: a pane resized, zoomed or swept in by a layout moves it and nothing of the frame's own says so.
 * While a layer of Bridge's own lies over it, main hides both views and the pictures it returns are drawn here instead.
 */
function LiveWalk({ jobId, walkId, api }: { jobId: string; walkId: string; api: Live }) {
  const frame = useRef<HTMLDivElement>(null);
  const [held, setHeld] = useState(false);
  const [pictures, setPictures] = useState<{
    page?: string;
    bar?: string;
  }>();
  useEffect(() => {
    let moving = 0;
    let last: Box | undefined;
    const follow = () => {
      setHeld(holdLayerUp());
      const box = frame.current?.getBoundingClientRect();
      if (box !== undefined) {
        const now = {
          x: box.x,
          y: box.y,
          width: box.width,
          height: box.height,
        };
        if (!sameBox(last, now)) void api.place(jobId, walkId, now);
        last = now;
      }
      moving = requestAnimationFrame(follow);
    };
    moving = requestAnimationFrame(follow);
    return () => {
      cancelAnimationFrame(moving);
      void api.unplace(jobId, walkId);
    };
  }, [
    api,
    jobId,
    walkId,
  ]);
  useEffect(() => {
    if (!held) return;
    let shown: {
      page?: string;
      bar?: string;
    } = {};
    let current = true;
    void api.freeze(jobId, walkId).then((frozen) => {
      if (!current || frozen === null) return;
      shown = {
        ...(frozen.page === null
          ? {}
          : {
              page: pictureOf(frozen.page),
            }),
        ...(frozen.bar === null
          ? {}
          : {
              bar: pictureOf(frozen.bar),
            }),
      };
      setPictures(shown);
    });
    return () => {
      current = false;
      setPictures(undefined);
      for (const url of Object.values(shown)) URL.revokeObjectURL(url);
      void api.thaw(jobId, walkId);
    };
  }, [
    api,
    jobId,
    walkId,
    held,
  ]);
  return (
    <div ref={frame} className="job-walk__live" role="region" aria-label="Walk">
      {pictures?.page === undefined ? null : (
        <img className="job-walk__picture" src={pictures.page} alt="" />
      )}
      {pictures?.bar === undefined ? null : (
        <img className="job-walk__bar-picture" src={pictures.bar} alt="" />
      )}
    </div>
  );
}

function MockWalk({ jobId, walkId }: { jobId: string; walkId: string }) {
  const session = useSessions().find((one) => one.id === jobId);
  const walk =
    session === undefined ? undefined : walksOf(session).find((one) => one.id === walkId);
  const state = useWalkState(jobId, walkId);
  const notes = useHeldNotes(jobId);
  const send = useSendHeld(session);
  if (session === undefined || walk === undefined) return null;
  const take = walk.takes.length === 0 ? undefined : walk.takes[state.take];
  return (
    <WalkView
      name={walk.title}
      takes={walk.takes}
      {...(walk.brief === undefined
        ? {}
        : {
            brief: walk.brief,
          })}
      take={state.take}
      onTake={(at) => pickTake(jobId, walk.id, at)}
      steps={walk.steps}
      step={state.step}
      onStep={(to) => goToStep(jobId, walk.id, to)}
      noteKey={`${jobId}:${walk.id}`}
      aim={`Onto ${session.title ?? session.id}`}
      onNote={(said, on) => holdNote(jobId, said, on)}
      onApprove={() => approveWalk(jobId, walk.id)}
      {...(notes.length === 0
        ? {}
        : {
            held: {
              notes: notes.length === 1 ? ("one" as const) : ("several" as const),
              onSend: () => void send(),
              onSendAndApprove: () => void send().then(() => approveWalk(jobId, walk.id)),
            },
          })}
      {...(state.readiness === undefined
        ? {}
        : {
            readiness: state.readiness,
          })}
      page={mockPage(walk.title, take)}
    />
  );
}
