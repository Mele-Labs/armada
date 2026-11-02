// What a `mock` node of a sketch is drawn with, read from Fleet: the style set its `kept` hash names. The hash is a content
// address, so an answer never changes and one read serves every frame that names it, for as long as the window lives.

import type { MockFont, MockStyles, ResolveMockStyles } from "@armada/components";
import type { KeptStyles } from "@armada/protocol";

import type { SessionActed } from "../../shared/api/sessions";

const NONE: MockStyles = { css: [], fonts: [], drawnWith: [] };

const bytesOf = (base64: string): Uint8Array => Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));

/** A font as the frame adds it: its bytes decoded, its weight as written. */
const fontOf = (font: KeptStyles["fonts"][number]): MockFont => ({
  family: font.family,
  weight: font.weight,
  ...(font.style === undefined ? {} : { style: font.style }),
  bytes: bytesOf(font.bytes),
});

/**
 * Reads a kept style set through `read` once per hash. A read that failed is dropped from the cache, so a later frame asks again, and
 * rejects, which draws the frame with no stylesheet of the app. `theme` is the attribute the stylesheets switch on, read when a frame is drawn.
 */
export function createKeptStyles(read: (hash: string) => Promise<SessionActed<KeptStyles>>, theme: () => string | null): ResolveMockStyles {
  const held = new Map<string, Promise<Omit<MockStyles, "root">>>();
  const keep = (hash: string): Promise<Omit<MockStyles, "root">> => {
    const had = held.get(hash);
    if (had !== undefined) return had;
    const asked = read(hash).then((answer) => {
      if (!answer.ok) throw new Error("kept styles were not read");
      return { css: answer.value.sheets.map((sheet) => sheet.css), fonts: answer.value.fonts.map(fontOf), drawnWith: answer.value.sheets.map((sheet) => sheet.path) };
    });
    held.set(hash, asked);
    asked.catch(() => held.delete(hash));
    return asked;
  };
  return async ({ kept }) => {
    if (kept === undefined) return NONE;
    const styles = await keep(kept);
    const now = theme();
    return now === null ? styles : { ...styles, root: { "data-theme": now } };
  };
}
