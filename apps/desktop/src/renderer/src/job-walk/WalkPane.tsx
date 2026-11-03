// One walk drawn in whatever holds it: a tab of the Job's panes, or a slot of the review sheet (`front`). It assumes nothing of what is around it. On a real Bridge the walk's page and its bar are
// views main lays over this frame (`main/capture/job-walks.ts`), so the pane is an empty box that reports where it is; the mock has no such views and draws `WalkView` with a fixture.

import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";

import { holdLayerUp } from "../hold-layers";
import { useSessions, useSessionsDraft } from "../sessions-draft";
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

// Main holds one pair of views for a walk, and a walk may be drawn twice: as a tab of the Job and in the review sheet over it. The one in front has the views; the other
// draws nothing and tells main nothing until it is the only one. `fronts` counts the panes in front of the rest, by walk.
const fronts = new Map<string, number>();
const listeners = new Set<() => void>();
const keyOf = (jobId: string, walkId: string) => `${jobId}\n${walkId}`;
const changed = () => listeners.forEach((one) => one());

/** Whether another pane is in front of this one for the walk. A pane in front (`front`) says so for as long as it is drawn. */
function useYielded(jobId: string, walkId: string, front: boolean): boolean {
  const key = keyOf(jobId, walkId);
  useLayoutEffect(() => {
    if (!front) return;
    fronts.set(key, (fronts.get(key) ?? 0) + 1);
    changed();
    return () => {
      const left = (fronts.get(key) ?? 1) - 1;
      if (left === 0) fronts.delete(key);
      else fronts.set(key, left);
      changed();
    };
  }, [
    front,
    key,
  ]);
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    () => !front && (fronts.get(key) ?? 0) > 0,
  );
}

/** Who placed a walk's views last, by walk: a pane's unplace hides the views only while they are still its own, not after the other pane placed them. */
const placed = new Map<string, object>();
/** The panes drawing each walk now. A pane that leaves while another draws it does not unplace: the other places, and a hide between would stop a walk that plays by itself. */
const drawn = new Map<string, Set<object>>();

export function WalkPane({
  jobId,
  walkId,
  front = false,
}: {
  jobId: string;
  walkId: string;
  /** Drawn in a layer over the Job (the review sheet): the walk's views are this pane's while it is drawn, and the Job's own tab for it yields. */
  front?: boolean;
}) {
  const live = window.armada?.jobWalk;
  return live === undefined ? (
    <MockWalk jobId={jobId} walkId={walkId} />
  ) : (
    <LiveWalk jobId={jobId} walkId={walkId} api={live} front={front} />
  );
}

/**
 * The frame is reported on every frame it moves, as the browser pane's is: a pane resized, zoomed or swept in by a layout moves it and nothing of the frame's own says so.
 * While a layer of Bridge's own lies over it, main hides both views and the pictures it returns are drawn here instead.
 */
function LiveWalk({
  jobId,
  walkId,
  api,
  front,
}: {
  jobId: string;
  walkId: string;
  api: Live;
  front: boolean;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const [held, setHeld] = useState(false);
  const yielded = useYielded(jobId, walkId, front);
  const self = useRef({});
  useLayoutEffect(() => {
    const key = keyOf(jobId, walkId);
    drawn.set(key, (drawn.get(key) ?? new Set()).add(self.current));
    return () => void drawn.get(key)?.delete(self.current);
  }, [
    jobId,
    walkId,
  ]);
  const [pictures, setPictures] = useState<{
    page?: string;
    bar?: string;
  }>();
  useEffect(() => {
    if (yielded) return;
    const key = keyOf(jobId, walkId);
    const me = {};
    let moving = 0;
    let last: Box | undefined;
    const follow = () => {
      // The sheet a walk plays in is not over it; a menu or dialog over the sheet is.
      setHeld(holdLayerUp(frame.current));
      const box = frame.current?.getBoundingClientRect();
      if (box !== undefined) {
        const now = {
          x: box.x,
          y: box.y,
          width: box.width,
          height: box.height,
        };
        if (!sameBox(last, now)) {
          placed.set(key, me);
          void api.place(jobId, walkId, now);
        }
        last = now;
      }
      moving = requestAnimationFrame(follow);
    };
    moving = requestAnimationFrame(follow);
    return () => {
      cancelAnimationFrame(moving);
      setHeld(false);
      if (placed.get(key) !== me) return;
      if (
        [
          ...(drawn.get(key) ?? []),
        ].some((one) => one !== self.current)
      )
        return;
      placed.delete(key);
      void api.unplace(jobId, walkId);
    };
  }, [
    api,
    jobId,
    walkId,
    yielded,
  ]);
  useEffect(() => {
    if (!held || yielded) return;
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
    yielded,
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
  const draft = useSessionsDraft();
  if (session === undefined || walk === undefined) return null;
  // The mock's Approve settles the walk in `store.ts`, and ends it as Fleet does for a walk Fleet holds an item for.
  const approve = () => {
    approveWalk(jobId, walk.id);
    draft?.approveWalk?.(jobId, walk.id);
  };
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
      onApprove={approve}
      {...(notes.length === 0
        ? {}
        : {
            held: {
              notes: notes.length === 1 ? ("one" as const) : ("several" as const),
              onSend: () => void send(),
              onSendAndApprove: () => void send().then(approve),
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
