// A Job as the palette lists it: where it works, how it stands, and the pull request it holds. Apart from `PaletteMount`
// so a test can hold the mapping without a window.

import { sessionGlyphOf } from "@armada/components";
import type { PaletteSession } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

import { stateOf } from "./session-state";

/**
 * **The repository is the name its Manifest goes by in the title row's picker**, so a word typed for a repository finds its
 * Jobs. The state is the registry's glyph for it with the words its tooltip says; a pull request is a number that finds the
 * row and is not drawn on it.
 */
export function jobRow(one: Session, repoOf: ((manifestId: string) => string | undefined) | undefined): PaletteSession {
  const repo = one.manifestId === undefined ? undefined : repoOf?.(one.manifestId);
  const { state, said } = stateOf(one);
  const held = one.attachments.find((attachment) => attachment.kind === "pull_request");
  return {
    id: one.id,
    label: one.title ?? one.address ?? one.id,
    aliases: [one.id, ...(one.address === undefined ? [] : [one.address])],
    ...(repo === undefined ? {} : { repo }),
    state: { icon: sessionGlyphOf(state), said, ...(state === "working" ? { pulsing: true as const } : {}) },
    ...(one.paused === true ? { paused: true } : {}),
    ...(held === undefined || held.kind !== "pull_request" ? {} : { pull: held.number }),
  };
}
