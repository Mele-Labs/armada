// A native view (a browser page, a walk's page and bar) paints over everything of Bridge's own. While anything of Bridge's lies over its frame the view is held: hidden, its picture drawn in its
// place. What covers the frame is found by asking what lies on it, so a surface that takes the screen (a zoomed pane, the palette, a sheet) needs nothing here.

// A layer small enough to fall between the points asked, or one that takes no presses and so is not found on them.
const HOLD_LAYERS = [
  // The layer drawn, and not its host: `[data-armada-annotate]` stays in the document for as long as a dev build runs, on or off.
  ".armada-annotate",
  ".job-browser__site-panel",
  ".armada-dialog-scrim",
  ".armada-job-review",
  ".armada-dropdown-menu__panel",
  ".armada-popover__panel",
  ".job-pane__drops",
].join(", ");

/** Where across the frame, and down it, is asked what lies there: in from each edge, clear of what only touches it. */
const POINTS = [
  0.1,
  0.5,
  0.9,
];

/** Whether something other than the frame and what is in it lies on the frame. A point outside the window finds nothing, and says nothing. */
const covered = (frame: Element): boolean => {
  const box = frame.getBoundingClientRect();
  return POINTS.some((across) =>
    POINTS.some((down) => {
      const there = document.elementFromPoint(
        box.x + box.width * across,
        box.y + box.height * down,
      );
      return there !== null && !frame.contains(there);
    }),
  );
};

/**
 * Whether anything of Bridge's lies over a native view now. `frame` is the view's own place: a layer that holds it is the ground it is drawn on, not over it, as the
 * review sheet is for a walk playing inside it. Anything drawn over that layer still holds the view.
 */
export const holdLayerUp = (frame?: Element | null): boolean => {
  if (frame === undefined || frame === null) return document.querySelector(HOLD_LAYERS) !== null;
  for (const layer of document.querySelectorAll(HOLD_LAYERS))
    if (!layer.contains(frame)) return true;
  return covered(frame);
};
