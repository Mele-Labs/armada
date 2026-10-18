// What a note taken in a Session's window becomes: `send_session_note`'s body, built from what the window held.
//
// Pure. **`scenario` is read off the page's own query**, `?scenario=`, as Bridge's own layer reads it
// (`annotate/capture.ts`); a walk that picks its scenario inside the page names none.

import type { SendSessionNote, StudioCapture } from "@armada/protocol";

import type { CaptureWalk } from "../../shared/capture-window";
import { cropOf, cutText, MOST_NEARBY, MOST_SELECTED } from "../../shared/pinned-note";
import type { PageRead } from "./bounds";
import type { Take } from "./session-note";

export type SessionNoteHeld = {
  sessionId: string;
  said: string;
  capture: StudioCapture;
  read: PageRead;
  /** The page's url as loaded. */
  address: string;
  take?: Take;
  walk: CaptureWalk | null;
  /** The base64 PNG the window cropped to the element, or null where none was taken. */
  crop: string | null;
};

const scenarioOf = (location: string): string | null => {
  try {
    const scenario = new URL(location, "http://page.invalid").searchParams.get("scenario");
    return scenario === null || scenario === "" ? null : scenario;
  } catch {
    return null;
  }
};

export function sessionNoteBody(held: SessionNoteHeld): SendSessionNote {
  const { capture, read, walk } = held;
  const scenario = scenarioOf(capture.location);
  const crop = cropOf(held.crop);
  return {
    session_id: held.sessionId,
    said: held.said.trim(),
    address: held.address,
    place: {
      ...(capture.component === undefined ? {} : { component: capture.component }),
      owners: capture.owners ?? [],
      selector: capture.selector,
      element: {
        tag: capture.element.tag,
        text: cutText(capture.element.text),
        ...(capture.element.label === undefined ? {} : { label: capture.element.label }),
      },
      nearby: read.nearby.slice(0, MOST_NEARBY),
      ...(read.stamp === undefined ? {} : { stamp: read.stamp }),
      location: capture.location,
      ...(capture.screen === undefined ? {} : { screen: capture.screen }),
      ...(capture.layer === undefined ? {} : { layer: capture.layer }),
    },
    state: {
      selected: read.selected.slice(0, MOST_SELECTED),
      ...(walk === null ? {} : { walk: { name: walk.name, step: walk.at, of: walk.of } }),
      ...(held.take === undefined ? {} : { take: held.take }),
      ...(scenario === null ? {} : { scenario }),
    },
    ...(crop === undefined ? {} : { crop }),
  };
}
