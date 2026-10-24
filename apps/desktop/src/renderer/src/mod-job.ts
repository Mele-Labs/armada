// Mod: a Job for modding Armada itself, started from a right-click menu and carrying what was pointed at.
// The first message is the pointed-at text the annotation layer sends Fleet with a note, without the note.

import { askToOpenSessionInCockpit } from "./open-session";
import { askToTell } from "./tell";
import { capture } from "./annotate/capture";
import { refusal } from "./annotate/send";
import type { Annotation } from "../../shared/annotations";

/** A Mod Job's title starts with this. The mock Fleet marks a Job started under such a title as a Mod Job. */
export const MOD_TITLE = "Mod ";

const NAME_WIDTH = 40;

/** What the element is called: its label or its words when they are short, else the place around it. */
function nameOf(note: Annotation): string {
  const said = (note.element.label ?? "").trim() || note.element.text.trim();
  if (said !== "" && said.length <= NAME_WIDTH) return said;
  return note.nearby?.[0] ?? note.component ?? note.element.tag;
}

/** `named` is what the caller already knows the element as: a Job row's title, which its own text only repeats. */
export const modTitleOf = (note: Annotation, named?: string): string => `${MOD_TITLE}${named ?? nameOf(note)}`;

/** The lines Fleet's note text opens with: what was pointed at, the page, and the places around it. */
export function pointedAt(note: Annotation): string {
  const named = (note.element.label ?? "").trim() || note.element.text.trim();
  const lines = [`Pointed at: ${note.element.tag}${named === "" ? "" : ` "${named}"`}`, `Page: ${note.location}`];
  const chain = [note.component, ...note.owners].filter((one): one is string => one !== null && one !== "");
  const inside = [...(note.nearby === undefined || note.nearby.length === 0 ? [] : [note.nearby.join(" ← ")]), ...(chain.length === 0 ? [] : [`component chain ${chain.join(" ← ")}`])];
  if (inside.length > 0) lines.push(`Inside: ${inside.join("; ")}`);
  return lines.join("\n");
}

/** Start a Mod Job on `element` and open it on the Cockpit. A refusal is told as a toast. */
export async function startModJob(element: Element, named?: string): Promise<void> {
  const fleet = window.armada;
  if (fleet === undefined) return;
  const note = capture(element, new Date());
  const title = modTitleOf(note, named);
  const started = await fleet.startSession(title);
  if (!started.ok) return askToTell(`Mod Job not started: ${refusal(started.outcome)}`);
  const sent = await fleet.sendSessionMessage({ session_id: started.value.id, text: pointedAt(note) });
  if (!sent.ok) askToTell(`Mod Job not told what was pointed at: ${refusal(sent.outcome)}`);
  askToOpenSessionInCockpit(started.value.id);
}
