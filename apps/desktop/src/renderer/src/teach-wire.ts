// The Teach panel's words and Fleet's: what a press on something becomes in `read_teaching` and `save_teaching`, and what a proposal
// is drawn as in the panel. **The panel shows what the wire carries and no more**, and so does the mock's Fleet, which answers as Fleet does.

import { NOTICE_NAMES, TEACH_SOURCE } from "@armada/components";
import type { SessionNotice, TeachReach, TeachReading } from "@armada/components";
import type { FleetDoes, ReadTeaching, SaveTeaching, TeachingProposal, TeachingReach, TeachingSource } from "@armada/protocol";
import { LAYOUT, parseLayout } from "@armada/shell";
import type { LayoutRegion } from "@armada/shell";

import type { Preview } from "./teach";
import { BRIDGE, foundSaid } from "./teach";
import type { PanelItem } from "./shared-panel/store";

type Pressed = Extract<PanelItem, { kind: "teach" }>;

/** The most of a thing's text a source carries: Fleet refuses more than 200 characters. */
const MOST_TEXT = 200;

const reachOf = (reach: TeachReach): TeachingReach => (reach === "all" ? "everywhere" : "repository");

/** What Teach was pressed on. A title bar has no text of its own, so the source names it. */
export function sourceOf(item: Pressed): TeachingSource {
  const text = item.text.trim() === "" ? TEACH_SOURCE[item.source] : item.text;
  return { kind: item.source, text: text.slice(0, MOST_TEXT), ...(item.ids === undefined || item.ids.length === 0 ? {} : { ids: item.ids }) };
}

/** Where it was taught, which Fleet keeps with the entry as its evidence. */
const citeOf = (item: Pressed): string => `Teach, ${TEACH_SOURCE[item.source].toLowerCase()}`;

/** Only a teaching for one repository names it: one for every repository is keyed to none, so an address it finds is each one's own. */
const placeOf = (item: Pressed, reach: TeachReach) => ({
  ...(reach === "repository" && item.root !== undefined ? { repository: item.root } : {}),
  // Bridge's own interface was taught in no Session.
  ...(item.sessionId === BRIDGE ? {} : { session_id: item.sessionId }),
});

/** `read_teaching`'s body for what the person wrote about it. */
export function readBody(item: Pressed, said: string, reach: TeachReach): ReadTeaching {
  return {
    ...(item.source === "words" ? { words: item.text.slice(0, MOST_TEXT) } : {}),
    said,
    reach: reachOf(reach),
    source: sourceOf(item),
    ...placeOf(item, reach),
  };
}

/** The first token of a tweak: the panel shows one colour, so one colour is what is kept. */
const firstTweak = (proposal: TeachingProposal) => proposal.tweak?.[0];

/** `save_teaching`'s body: the proposal as it was shown, and where it was taught. */
export function saveBody(item: Pressed, proposal: TeachingProposal, reach: TeachReach): SaveTeaching {
  const tweak = firstTweak(proposal);
  const { tweak: _all, ...rest } = proposal;
  return { ...rest, ...(tweak === undefined ? {} : { tweak: [tweak] }), reach: reachOf(reach), cite: citeOf(item), source: sourceOf(item), ...placeOf(item, reach) };
}

export const DOES: Record<FleetDoes, string> = { tell: "Tell", quiet: "Quiet", batch: "Batch", call: "Call" };

/** A notice's word as the menu names it, with a capital. */
export function noticeName(word: string): string {
  const name = (NOTICE_NAMES as Record<string, string | undefined>)[word as SessionNotice] ?? word;
  return `${name.charAt(0).toUpperCase()}${name.slice(1)}`;
}

/** Where a rule reaches, from the place Fleet wrote: `job <id>`, `repo <manifest>` or `everywhere`. */
export function scopeOf(where: string, repository: string): string {
  return where === "everywhere" ? "Every repository" : where.startsWith("job ") ? "This Job" : repository;
}

/** The first region a layout hides something in, and what it hides there, as the registry names them. */
export function layoutOf(name: string, text: string): { hides: string; where: string } {
  const regions = parseLayout(text).file.regions;
  const region = (Object.keys(regions) as LayoutRegion[]).find((one) => (regions[one]?.hidden ?? []).length > 0);
  if (region === undefined) return { hides: name, where: "Layout" };
  const spec = LAYOUT[region];
  const hidden = regions[region]?.hidden ?? [];
  return { hides: hidden.map((id) => spec.entries.find((one) => one.id === id)?.label ?? id).join(", "), where: spec.label };
}

/** What the panel shows back of a proposal. */
export function readingOf(proposal: TeachingProposal, repository: string): TeachReading {
  const { note, words, fleet, layout } = proposal;
  const tweak = firstTweak(proposal);
  return {
    ...(note === undefined ? {} : { agents: note.text }),
    ...(words === undefined
      ? {}
      : { bridge: { name: words.name, pattern: words.pattern, example: words.example, ...(words.address === undefined ? {} : { address: words.address.template.replaceAll("{match}", words.example), found: foundSaid(words.address.found) }) } }),
    ...(fleet === undefined ? {} : { fleet: { notice: noticeName(fleet.notice), does: DOES[fleet.does], scope: scopeOf(fleet.where, repository) } }),
    ...(tweak === undefined ? {} : { tweak: { colour: tweak.note ?? tweak.token, token: tweak.token, was: tweak.was, now: tweak.now, places: tweak.used_in ?? [] } }),
    ...(layout === undefined ? {} : { layout: layoutOf(layout.name, layout.layout) }),
  };
}

/** What the proposal draws behind the panel before it is kept: a link where an address came with the words, a colour. */
export function previewOf(proposal: TeachingProposal, reach: TeachReach, manifestId: string | undefined): Preview | undefined {
  const tweak = firstTweak(proposal);
  const words = proposal.words;
  const drawn = words?.address === undefined ? undefined : { pattern: words.pattern, address: words.address.template, from: foundSaid(words.address.found) };
  if (drawn === undefined && tweak === undefined) return undefined;
  return {
    reach,
    ...(manifestId === undefined ? {} : { manifestId }),
    ...(drawn === undefined ? {} : { link: drawn }),
    ...(tweak === undefined ? {} : { tweak: { token: tweak.token, now: tweak.now } }),
  };
}
