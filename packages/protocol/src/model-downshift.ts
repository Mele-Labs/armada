// The Session model's step down on the wire, mirrored by hand from `crates/ipc/src/model_downshift.rs`.

/** Where the step down stands. The tag is `phase`. */
export type ModelDownshift =
  /** The weekly window is at or over the line and the Session model, `was`, is not yet changed. */
  | { phase: "offered"; percent: number; was: string; to: string }
  /** Accepted: the Session model is `to` until `until`, the window's reset. `was` is what restoring puts back. */
  | { phase: "shifted"; was: string; to: string; until: string }
  /** The window reset and the Session model is still `to`. */
  | { phase: "reset"; was: string; to: string };

/**
 * `GET /model_downshift`, what `POST /model_downshift/{accept,restore,keep,settle}` answer, and
 * `model_downshift.changed` published whole. Absent `downshift` is none.
 */
export type ModelDownshiftState = {
  downshift?: ModelDownshift;
};
