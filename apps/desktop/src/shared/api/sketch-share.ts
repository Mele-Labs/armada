// Saving a shared sketch to disk: the one place a sketch leaves Bridge as a file. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

/** What a sketch is saved as: a web page, or a picture in one of two formats. */
export type SketchSaveKind = "html" | "png" | "svg";

/** One rendering of the sketch. `name` is a file name and never a path: where it goes is the person's pick in a dialog main opens. */
export type SketchSaveFile = { kind: SketchSaveKind; name: string; bytes: Uint8Array };

/** What a save came to. A dialog closed without a place is not a failure. */
export type SketchSaved = { saved: true; name: string } | { saved: false; why: "cancelled" | "failed" };

export type SketchShareApi = {
  /**
   * Asks the person where to put the sketch and writes it there. Several `files` are the same sketch in
   * different formats, offered as the dialog's file types; the one whose extension they choose is written.
   */
  saveSketchFile: (files: readonly SketchSaveFile[]) => Promise<SketchSaved>;
};

export const SKETCH_SHARE_CHANNELS = {
  saveSketchFile: "bridge:save-sketch-file",
} as const;
