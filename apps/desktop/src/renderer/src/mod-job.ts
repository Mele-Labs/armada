// Mod: a Job for modding Armada itself, started from a right-click menu and carrying what was pointed at.
// Fleet starts it and sends its first message, the Mod brief and the note, as a Report's note is built.

import { askToOpenSessionInCockpit } from "./open-session";
import { askToTell } from "./tell";
import { capture } from "./annotate/capture";
import { refusal } from "./annotate/send";
import type { Annotation } from "../../shared/annotations";
import { annotationNote } from "../../shared/pinned-note";

const NAME_WIDTH = 40;

/** What the element is called: its label or its words when they are short, else the place around it. */
function nameOf(note: Annotation): string {
  const said = (note.element.label ?? "").trim() || note.element.text.trim();
  if (said !== "" && said.length <= NAME_WIDTH) return said;
  return note.nearby?.[0] ?? note.component ?? note.element.tag;
}

/** Start a Mod Job on `element` and open it on the Cockpit. A refusal is told as a toast. */
export async function startModJob(element: Element, named?: string): Promise<void> {
  const fleet = window.armada;
  if (fleet === undefined) return;
  const note = capture(element, new Date());
  const started = await fleet.startModSession({ title: named ?? nameOf(note), note: annotationNote(note, window.location.href, null) });
  if (!started.ok) return askToTell(`Mod Job not started: ${refusal(started.outcome)}`);
  if (started.value.send_failed !== undefined) askToTell(`Mod Job not told what was pointed at: ${started.value.send_failed}`);
  askToOpenSessionInCockpit(started.value.session_id);
}
