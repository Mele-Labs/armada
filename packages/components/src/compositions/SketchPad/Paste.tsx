import { useReactFlow } from "@xyflow/react";
import { useCallback, useEffect, type RefObject } from "react";

import { clearOf } from "../GraphCanvas/GraphCanvas";
import type { SketchPictureLanding } from "./SketchPad";

/**
 * A paste on the sketch pad, and where anything new on the pad lands.
 *
 * **A paste aimed at the pad lands on it** (the owner, 1 Oct 2026): text is a
 * new box holding it, and a screenshot is a picture — moved, joined, drawn
 * over and removed like a box, and never resized. A paste into a box's own
 * field is that field's, and makes nothing.
 *
 * Both halves read the viewport React Flow holds, so both mount inside the
 * board, as `Ink` does.
 */

/**
 * How much of the pad a pasted picture may take, each way. **A screenshot is
 * the whole screen**, which at the pad's own scale is several pads across; it
 * is brought down to sit inside the part of the pad on show, with room around
 * it for the boxes it is about. A smaller picture keeps its own size.
 */
const A_PICTURE_FILLS = 0.6;

type Room = { x: number; y: number; width: number; height: number };

/** How finely a free spot is searched for, in box widths. */
const A_SEARCH_STEP = 1 / 8;

/**
 * The free spot nearest `centre` for something of `size`, inside `view` and
 * clear of every `taken` rect by a step's air — or `null` where the view holds
 * none.
 */
function freeSpot(
  taken: readonly Room[],
  size: { width: number; height: number },
  view: Room,
  centre: { x: number; y: number },
  step: number,
): { x: number; y: number } | null {
  const clear = (x: number, y: number) =>
    taken.every(
      (one) =>
        x + size.width + step <= one.x ||
        one.x + one.width + step <= x ||
        y + size.height + step <= one.y ||
        one.y + one.height + step <= y,
    );
  const spots: { x: number; y: number; far: number }[] = [];
  for (let y = view.y; y + size.height <= view.y + view.height; y += step) {
    for (let x = view.x; x + size.width <= view.x + view.width; x += step) {
      const far = Math.hypot(x + size.width / 2 - centre.x, y + size.height / 2 - centre.y);
      spots.push({ x, y, far });
    }
  }
  spots.sort((a, b) => a.far - b.far);
  const found = spots.find((spot) => clear(spot.x, spot.y));
  return found === undefined ? null : { x: Math.round(found.x), y: Math.round(found.y) };
}

/**
 * Where a new box or picture lands: **the free spot nearest the middle of the
 * pad on show**, so it is in view and on top of nothing. Add a box, a pasted
 * line of text and a pasted picture all put one down here.
 *
 * **In view first.** This stepped a box a box-width down and across until no
 * corner sat near it, which on a pad already holding the arc's three boxes put
 * every new one below a pad 300 high — the owner's "not allowing me to paste
 * links", 1 Oct 2026, measured at (935, 656) in a 1110×300 pad. Where the pad
 * on show has no free spot, it lands in the middle over what is there, and a
 * second one steps a little off the first rather than hiding it.
 *
 * A hook inside the board, because only there does React Flow's viewport resolve.
 */
export function useMiddle() {
  const flow = useReactFlow();
  return useCallback(
    (picture?: { width: number; height: number }) => {
      const pane = document
        .querySelector(".armada-sketch-pad__canvas .react-flow")
        ?.getBoundingClientRect();
      if (pane === undefined) return { x: 0, y: 0 };
      const from = flow.screenToFlowPosition({ x: pane.left, y: pane.top });
      const to = flow.screenToFlowPosition({ x: pane.right, y: pane.bottom });
      const view = { x: from.x, y: from.y, width: to.x - from.x, height: to.y - from.y };
      const centre = { x: view.x + view.width / 2, y: view.y + view.height / 2 };
      // A box's width is read off the token it is drawn at rather than restated
      // here; its height is the shortest box already measured, which is what an
      // empty or one-line box comes to.
      const read = Number.parseFloat(
        getComputedStyle(document.body).getPropertyValue("--w-sketch-box"),
      );
      const wide = Number.isFinite(read) ? read : 0;
      const nodes = flow.getNodes();
      const tall = nodes
        .filter((node) => node.type === "sketch")
        .map((node) => node.measured?.height ?? 0)
        .filter((height) => height > 0);
      const size = picture ?? {
        width: wide,
        height: tall.length > 0 ? Math.min(...tall) : wide * A_SEARCH_STEP * 2,
      };
      const taken = nodes.map((node) => ({
        x: node.position.x,
        y: node.position.y,
        width: node.measured?.width ?? 0,
        height: node.measured?.height ?? 0,
      }));
      const step = Math.max(1, wide * A_SEARCH_STEP);
      const spot = freeSpot(taken, size, view, centre, step);
      if (spot !== null) return spot;
      // Nowhere free in view: the middle, a step off anything already put
      // down exactly there, and never walked out of the view.
      const stepped = clearOf(
        nodes.map((node) => node.position),
        { x: centre.x - size.width / 2, y: centre.y - size.height / 2 },
        step,
      );
      return {
        x: Math.round(Math.min(Math.max(stepped.x, view.x), view.x + view.width - size.width)),
        y: Math.round(Math.min(Math.max(stepped.y, view.y), view.y + view.height - size.height)),
      };
    },
    [flow],
  );
}

/** Whether a paste was aimed at somewhere a person types, which takes it as text. */
function isTyping(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && target.closest("textarea, input, [contenteditable]") !== null;
}

/**
 * A paste aimed at the pad rather than at a field on it. **A picture wins over
 * text**, because a copied screenshot often carries a name beside its bytes and
 * the bytes are what was meant. Text lands at once as a box holding it, with
 * no field opening first — the rule the owner set for a Studio's paste on
 * 1 Oct 2026.
 *
 * **Listened for on the pad's frame**, which is outside the board; this sits
 * inside it so where a paste lands is read off the same viewport Add a box
 * reads.
 */
export function Paste({
  pad,
  onAdd,
  onPicture,
}: {
  pad: RefObject<HTMLDivElement | null>;
  onAdd: (at: { x: number; y: number }, body: string) => void;
  onPicture: (picture: SketchPictureLanding, at: { x: number; y: number }) => void;
}) {
  const flow = useReactFlow();
  const middle = useMiddle();

  useEffect(() => {
    const frame = pad.current;
    if (frame === null) return;

    const land = async (file: File) => {
      const bitmap = await createImageBitmap(file);
      const { width, height } = bitmap;
      bitmap.close();
      const shown = frame.getBoundingClientRect();
      const zoom = flow.getZoom();
      const scale = Math.min(
        1,
        ((shown.width / zoom) * A_PICTURE_FILLS) / width,
        ((shown.height / zoom) * A_PICTURE_FILLS) / height,
      );
      const size = { width: Math.round(width * scale), height: Math.round(height * scale) };
      onPicture({ src: URL.createObjectURL(file), ...size }, middle(size));
    };

    const pasted = (event: ClipboardEvent) => {
      if (isTyping(event.target) || event.clipboardData === null) return;
      const picture = Array.from(event.clipboardData.items)
        .find((item) => item.kind === "file" && item.type.startsWith("image/"))
        ?.getAsFile();
      if (picture !== null && picture !== undefined) {
        event.preventDefault();
        void land(picture);
        return;
      }
      const text = event.clipboardData.getData("text/plain").trim();
      if (text === "") return;
      event.preventDefault();
      onAdd(middle(), text);
    };

    frame.addEventListener("paste", pasted);
    return () => frame.removeEventListener("paste", pasted);
  }, [pad, flow, middle, onAdd, onPicture]);

  return null;
}

