// What a sketch is shared as, and what the host is asked to do with it. The three transforms are pure
// (`export-mermaid.ts`, `export-svg.ts`, `export-html.ts`); this is the vocabulary between them and the
// host that writes a file or fills the clipboard.

export type SketchShareKind = "html" | "png" | "svg";

/** One rendering of the sketch, as a host writes it. `name` is the whole file name, extension included. */
export type SketchShareFile = { kind: SketchShareKind; name: string; data: string | Uint8Array };

/** What saving came to. Where a person was asked for a place and gave none, nothing failed. */
export type SketchShareSaved = { saved: true; name: string } | { saved: false; why: "cancelled" | "failed" };

/**
 * What the app around the sketch does with it. **Every member is optional**: with none, Save is a browser
 * download and Copy is the browser's clipboard, which is what Storybook and any host without a file dialog get.
 */
export type SketchShareHost = {
  /** Writes one of `files`, whichever the person picks a place for: the same sketch in each. */
  save?: (files: readonly SketchShareFile[]) => Promise<SketchShareSaved>;
  /** Puts text on the clipboard. */
  copy?: (text: string) => Promise<void>;
  /** A copy is silent, so the host says so: the name of what was copied. */
  copied?: (what: string) => void;
  /** A sentence for the person, where a save finished or did not. */
  said?: (sentence: string) => void;
};

export const MIME: Record<SketchShareKind, string> = { html: "text/html", png: "image/png", svg: "image/svg+xml" };

/** What the toast names after a copy, as `The debug info` is named. */
export const MERMAID_COPIED = "The Mermaid text";

/** A save that ended, said in one sentence. */
export function savedSaid(name: string, saved: SketchShareSaved): string | undefined {
  if (saved.saved) return `Saved ${saved.name}.`;
  return saved.why === "cancelled" ? undefined : `${name} was not saved.`;
}

/** The title as a file name: nothing a file system or a shell reads as more than a letter, and never empty. */
export function sketchFileName(title: string | undefined, extension: string): string {
  const base = (title ?? "")
    // eslint-disable-next-line no-control-regex -- control characters are what is being taken out
    .replace(/[\u0000-\u001f\u007f\\/:*?"<>|]+/g, " ")
    .replace(/\s+/g, " ")
    .replace(/^[\s.]+|[\s.]+$/g, "")
    .slice(0, 80)
    .trim();
  return `${base === "" ? "Sketch" : base}.${extension}`;
}
