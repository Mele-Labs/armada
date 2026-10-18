// A pinned note, whole: `send_session_note`'s body, field for field as the pinned-note spec names it.
//
// **Local until the protocol mirror lands**, then swapped for it. Shared because both ends build one:
// the capture window in main (`capture/pinned-note.ts`) and Bridge's own ⌥⌘A layer in the renderer.

import type { Annotation } from "./annotations";

export type NoteElement = { tag: string; text: string; label?: string };

/** Where he pointed. */
export type NotePlace = {
  component?: string;
  owners: string[];
  selector: string;
  element: NoteElement;
  /** Accessible names of the enclosing landmarks, lists, sections and dialogs, and the nearest heading above, innermost first. At most 8. */
  nearby: string[];
  /** `path:line` off the closest `[data-armada-source]`. */
  stamp?: string;
  /** Path, query and hash. */
  location: string;
  screen?: string;
  layer?: string;
};

/** What the page was showing. */
export type NoteState = {
  /** At most 12, each `role "name"`. */
  selected: string[];
  walk?: { name: string; step: number; of: number };
  take?: { letter: string; name: string };
  scenario?: string;
};

/** A base64 PNG of the spot. */
export type NoteCrop = { media_type: string; data: string };

export type SendSessionNote = {
  session_id: string;
  said: string;
  address: string;
  /** The `.armada/annotations/<id>.json` note, when Bridge's own layer took it. */
  note_id?: string;
  place: NotePlace;
  state: NoteState;
  crop?: NoteCrop;
};

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
