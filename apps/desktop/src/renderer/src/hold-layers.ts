// The layers of Bridge's own that lie over a native view (a browser page, a walk's page and bar), which would hide them. While one is in the document the view is held:
// hidden, its picture drawn in its place. One list, so a pane that draws a native view cannot miss one another pane knows.

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

/** Whether a layer that must lie over a native view is up now. */
export const holdLayerUp = (): boolean => document.querySelector(HOLD_LAYERS) !== null;
