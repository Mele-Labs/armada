// The frames a step's harness produced, fetched so the panel can lead with
// them.
//
// **`outputs.ts`'s shape, and one deliberate departure from it.** A Check's
// output is fetched when somebody opens that Check, because output is the
// payload the event stream is bounded to keep off itself and pre-fetching every
// row would spend exactly what the split avoids. Frames are fetched when the
// step is opened, without anybody pressing: the whole claim of `#209` is that a
// change whose point is not the code is reviewed by *looking at it*, and a
// panel that leads with a button saying there are pictures has not led with
// anything. The bound is the shape of the thing — one step is open at a time
// and a spec captures a handful of screens, where one Check prints a whole
// test run.
//
// **Keyed by `kept`, which is Fleet's own id for a row**: the run's directory
// and the file name, composed once on the Rust side and sent back as it was
// given. Nothing here derives it from a path.
//
// # The bytes become a `blob:` and this owns it
//
// What crosses the preload is an array, because the renderer never opens a
// socket and a base64 in between would inflate it by a third to say the same
// thing. `URL.createObjectURL` is what turns that into something an `img` can
// draw, and every one it mints has to be revoked — an object URL is held by the
// document until it is, so a person walking six Jobs would leave six sets of
// screenshots in memory. The revoking is this module's whole reason for holding
// state rather than fetching in a component.

// # A recording is pointed at rather than read
//
// A video is the one kind never fetched here. What a plate gets is an address
// on the scheme Bridge's main process handles, and main forwards a span of it
// at a time — so a two-minute capture starts playing without being held
// anywhere first, and seeking to the end does not read what came before.
//
// This is what retired the twenty-mebibyte skip (`#615`). Nothing is minted
// for a recording and nothing has to be revoked for one, because there is no
// `blob:` in it at all.

import { useCallback, useEffect, useRef } from "react";

import { useHeldReads } from "./held-reads";

import type { FrameContent, ShownFrame } from "@armada/components";
import type { FrameRead, KeptFrame } from "@armada/protocol";

/**
 * What one frame is, as this window has it.
 *
 * `undefined` — what the map answers for a frame nothing has asked for — is the
 * moment before the step's fetch goes out, and it draws as `reading…` for the
 * same reason a started one does: from a reader's side they are the same wait.
 */
export type FrameState =
  /** Asked for, nothing back yet. */
  | { state: "fetching" }
  /** What the bytes are, drawn as the kind they are. */
  | { state: "got"; content: FrameContent }
  /**
   * Nothing is drawn, and the reader says why in one sentence.
   *
   * **The frame's own, never the step's.** A frame the record holds and the
   * disk does not is a 422 about one file, and a kind Bridge does not know is a
   * decision made once, in [`drawn`]. Both are one frame's business, and the
   * other frames of that run are still drawn.
   */
  | { state: "absent"; note: string };

/**
 * What a media type says a frame is, for the four kinds `#605` names.
 *
 * **Read off what Fleet answered, never off the name a second time.** The
 * extension already decided the media type once, in `answers::media_type`; a
 * client that parsed the name again would be a second place that mapping is
 * written, and the one place it could disagree with the file it just read.
 * Anything this cannot place — `application/octet-stream`, which is also what
 * an SVG or an HTML file answers as, on purpose — is a kind Bridge does not
 * draw.
 */
function kindOf(mediaType: string): FrameContent["kind"] | undefined {
  if (mediaType.startsWith("image/")) return "image";
  if (mediaType.startsWith("video/")) return "video";
  if (mediaType === "application/json") return "json";
  if (mediaType.startsWith("text/")) return "text";
  return undefined;
}

/**
 * Whether a name is one of the extensions Fleet answers as a video.
 *
 * **A guess, and the only one this module makes.** Every other kind is read
 * off the media type Fleet already sent; this one is asked before any answer
 * exists, because a video is never fetched at all — it is drawn from an
 * address instead. `answers::media_type` is the one true mapping and this has
 * to agree with it on the two extensions it names; a false negative costs a
 * recording read whole the way an image is, which is what this replaced.
 */
function namedAsVideo(name: string): boolean {
  const said = name.slice(name.lastIndexOf(".") + 1).toLowerCase();
  return said === "webm" || said === "mp4";
}

/**
 * Where a recording streams from, as the screen's caller hands it in.
 *
 * **[`ReadFrame`]'s rule, for a thing that is not a read.** A screen may not
 * know how to reach Fleet, so the way to compose an address arrives as an
 * argument exactly as the way to ask does — and a story passes its own.
 */
export type FrameSrc = (jobId: string, kept: string) => string;

/** What a chapter needs: what is held, and how to ask for a step's worth. */
export type Frames = {
  of: (kept: string) => FrameState | undefined;
  /** Ask for every frame in this list that has not been asked for yet. */
  want: (frames: KeptFrame[]) => void;
};

/**
 * Reading one frame, as the screen's caller hands it in.
 *
 * **An argument, not a global**, which is `ReadCheckOutput`'s rule: the bytes
 * come from the process that can reach Fleet, and the screen is handed the way
 * to ask rather than the way to connect.
 */
export type ReadFrame = (jobId: string, kept: string) => Promise<FrameRead>;

/**
 * Hold one Job's fetched frames, and revoke what it minted.
 *
 * The Job id is a dependency rather than an argument, for `useCheckOutputs`'
 * reason: a `kept` is only meaningful under the Job whose `.armada/frames`
 * holds it, and carried into the next Job it would name a file that Job never
 * wrote.
 */
export function useFrames(read: ReadFrame, jobId: string, streamed: FrameSrc): Frames {
  // **Every object URL this window minted, revoked when the Job changes or the
  // window goes.** An effect, because a `blob:` is held by the document rather
  // than by React, and synchronising with something outside React is the one
  // thing an effect is for.
  const minted = useRef<string[]>([]);
  useEffect(() => {
    return () => {
      for (const src of minted.current) URL.revokeObjectURL(src);
      minted.current = [];
    };
  }, [jobId]);

  const { of, fetch, hold } = useHeldReads<FrameRead, FrameState>({
    read,
    jobId,
    settle: (answer) => drawn(answer, minted),
    asking: { state: "fetching" },
    // A rejected invoke is main gone, which is the window closing. Recorded as
    // an absence so the plate is not left saying `reading…` for the rest of
    // its life.
    failed: { state: "absent", note: NOT_ANSWERED },
  });

  const want = useCallback(
    (frames: KeptFrame[]) => {
      for (const frame of frames) {
        if (of(frame.kept) !== undefined) continue;
        // **A video is never fetched; it is pointed at.** The address is
        // composed from the id the record already carries, so a recording is
        // `got` in the same breath it is asked for and the player does the
        // reading — which is what makes one longer than a minute watchable.
        if (namedAsVideo(frame.name)) {
          hold(frame.kept, { state: "got", content: { kind: "video", src: streamed(jobId, frame.kept) } });
        } else {
          fetch(frame.kept);
        }
      }
    },
    [of, fetch, hold, streamed, jobId],
  );

  return { of, want };
}

/**
 * What came back, as the plate will draw it.
 *
 * **The URL is minted here and recorded in the same breath**, so there is one
 * place a `blob:` comes into existence and one list that has to be revoked. A
 * component calling `createObjectURL` in a render would mint one per paint.
 *
 * **The kind is read off `read.type`, never off the name again.** Fleet
 * already decided it once, in `answers::media_type`, and sent it with the
 * bytes; a second opinion composed here would be the one that could disagree
 * with the file it just read. A type this cannot place — `svg`, `html`, or
 * anything else Fleet answers as `application/octet-stream` — draws as a kind
 * Bridge does not know, named rather than shown broken.
 */
function drawn(read: FrameRead, minted: { current: string[] }): FrameState {
  if (!read.ok) {
    const refused = !read.outcome.ok && read.outcome.why === "refused";
    return { state: "absent", note: refused ? NOT_ON_DISK : NOT_ANSWERED };
  }
  const kind = kindOf(read.type);
  if (kind === "image" || kind === "video") {
    const src = URL.createObjectURL(new Blob([read.bytes as BlobPart], { type: read.type }));
    minted.current.push(src);
    return { state: "got", content: { kind, src } };
  }
  if (kind === "text" || kind === "json") {
    return { state: "got", content: { kind, text: new TextDecoder().decode(read.bytes) } };
  }
  return { state: "absent", note: CANNOT_DRAW };
}

/**
 * The row is on the record and the file is not. The 422, in the app's voice.
 *
 * **A frame's whole content is the image**, so this is the sentence for a
 * `.armada/frames` directory that was reclaimed after the record was written —
 * which is the case the refusal was written for.
 */
const NOT_ON_DISK = "This frame is on the record and no longer on disk.";

/** Fleet did not answer. The same sentence the brief and the outputs use. */
const NOT_ANSWERED = "Fleet did not answer for this frame.";

/**
 * The sentence for a kind Fleet answered as bytes and nothing here can draw —
 * an SVG or an HTML file, or anything else `answers::media_type` did not name.
 * The frame's own name is already on the line beside the plate, so this only
 * has to say why there is nothing on it.
 */
const CANNOT_DRAW = "Bridge does not know how to draw this kind of file.";

/**
 * What the two sides are called on the screen.
 *
 * **The reader's words and not the wire's.** `base` and `branch` name the two
 * checkouts Fleet had to serve; what a person reading a step wants to know is
 * which of these is how the screen was. The translation happens once, here, so
 * no component learns what a base branch is.
 */
const asBefore = "before";
const asAfter = "after";

/**
 * The rows the record holds, married to what this window has fetched.
 *
 * **Wire order, and no sort.** Fleet answers oldest run first and that ordering
 * is the record's; re-sorting on arrival is the column flip-flop the failure
 * log named, and here it would silently reorder the runs a person is comparing.
 *
 * A frame nothing has asked for yet carries neither `content` nor `why`, which
 * the plate draws as `reading…` — the honest reading, because from where the
 * person is sitting a fetch that has not gone out and one that has not come
 * back are the same wait.
 */
export function shownFrames(frames: KeptFrame[], held: Frames): ShownFrame[] {
  // **Labelled only where there is something to compare against.** A step with
  // frames from one side is a repository with no base, a base run that would
  // not start, or a Fleet older than 9.5 — and `after` written on every frame
  // of a set with no before is a word that says nothing and implies a missing
  // half.
  const both = new Set(frames.map((frame) => frame.side ?? "branch")).size > 1;
  return frames.map((frame) => {
    const state = held.of(frame.kept);
    return {
      kept: frame.kept,
      name: frame.name,
      attempt: frame.attempt,
      weight: weighs(frame.bytes),
      ...(both ? { side: (frame.side ?? "branch") === "base" ? asBefore : asAfter } : {}),
      ...(state?.state === "got" ? { content: state.content } : {}),
      ...(state?.state === "absent" ? { why: state.note } : {}),
    };
  });
}

/**
 * A holder that has nothing and asks for nothing.
 *
 * **For a caller whose subject is not the frames.** It is not a fallback: a
 * screen that reached for this rather than being handed a real one would draw
 * every frame as `reading…` forever.
 */
export const NO_FRAMES: Frames = { of: () => undefined, want: () => {} };

/** What a file weighs, at the precision a person reads rather than counts. */
function weighs(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
