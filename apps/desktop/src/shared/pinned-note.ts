// What a pinned note's body is built from, whoever read the page: the spec's bounds, and the body
// one of Bridge's own ⌥⌘A pins makes. Shared because both ends build one: the capture window in main
// (`capture/pinned-note.ts`) and Bridge's own layer in the renderer.

import type { NoteCrop, SendSessionNote } from "@armada/protocol";

import type { Annotation } from "./annotations";

/** The spec's bounds, applied whoever read the page. */
export const MOST_ELEMENT_TEXT = 200;
export const MOST_NEARBY = 8;
export const MOST_SELECTED = 12;

export const cutText = (text: string): string => (text.length > MOST_ELEMENT_TEXT ? text.slice(0, MOST_ELEMENT_TEXT) : text);

export const cropOf = (base64: string | null): NoteCrop | undefined =>
  base64 === null || base64 === "" ? undefined : { media_type: "image/png", data: base64 };

/** The note one of Bridge's own ⌥⌘A pins makes, carrying its file's id so `mark_annotation_done` still finds it. */
export function annotationNoteBody(note: Annotation, sessionId: string, address: string, crop: string | null): SendSessionNote {
  const cropped = cropOf(crop);
  return {
    session_id: sessionId,
    said: note.text.trim(),
    address,
    note_id: note.id,
    place: {
      ...(note.component === null ? {} : { component: note.component }),
      owners: note.owners,
      selector: note.selector,
      element: {
        tag: note.element.tag,
        text: cutText(note.element.text),
        ...(note.element.label === null ? {} : { label: note.element.label }),
      },
      nearby: (note.nearby ?? []).slice(0, MOST_NEARBY),
      ...(note.source === undefined ? {} : { stamp: `${note.source.file}:${note.source.line}` }),
      location: note.location,
      ...(note.screen === null ? {} : { screen: note.screen }),
      ...(note.layer === null ? {} : { layer: note.layer }),
    },
    state: {
      selected: (note.selected ?? []).slice(0, MOST_SELECTED),
      ...(note.scenario === null ? {} : { scenario: note.scenario }),
    },
    ...(cropped === undefined ? {} : { crop: cropped }),
  };
}
