// The dispatch row's chips, read from what is typed: a marker and a name (`@armada`, `+Docs refresh`) sets a chip as Tab and its
// list do. Pure, so the palette and its host read a token the same way.

/** One choice a chip offers. `detail` trails it in the chip's list, muted: `last used`. */
export type PaletteChipOption = { id: string; label: string; detail?: string };

/** What a chip needs to be read from the words. */
type Marked = { id: string; marker?: string; options: readonly PaletteChipOption[] };

/** A marker starts a token only at the start or after a space, so `a@b` stays words. */
const opens = (query: string, at: number): boolean => at === 0 || /\s/.test(query[at - 1] ?? "");

/** The option named just past a marker: the longest label the words go on with, ending at a space or the end. */
function named(chip: Marked, query: string, from: number): { option: PaletteChipOption; end: number } | undefined {
  const rest = query.slice(from).toLowerCase();
  let best: { option: PaletteChipOption; end: number } | undefined;
  for (const option of chip.options) {
    const label = option.label.toLowerCase();
    const next = rest[label.length];
    if (label === "" || !rest.startsWith(label) || (next !== undefined && !/\s/.test(next))) continue;
    if (best === undefined || label.length > best.end - from) best = { option, end: from + label.length };
  }
  return best;
}

/** The tokens in the words: each chip's choice where a token names it (the last one wins), and the words without the tokens. */
export function readTokens(query: string, chips: readonly Marked[]): { words: string; chosen: Record<string, string> } {
  const chosen: Record<string, string> = {};
  const cut: [number, number][] = [];
  for (let at = 0; at < query.length; at += 1) {
    if (!opens(query, at)) continue;
    const chip = chips.find((one) => one.marker !== undefined && one.marker !== "" && query.startsWith(one.marker, at));
    const found = chip === undefined ? undefined : named(chip, query, at + chip.marker!.length);
    if (chip === undefined || found === undefined) continue;
    chosen[chip.id] = found.option.id;
    cut.push([at, found.end]);
    at = found.end;
  }
  const words = cut.reduceRight((text, [from, to]) => text.slice(0, from) + text.slice(to), query).replace(/\s+/g, " ").trim();
  return { words, chosen };
}

/** The words with the tokens of one chip taken out, for a choice made from its list instead. */
export function withoutTokens(query: string, chip: Marked): string {
  if (chip.marker === undefined) return query;
  const cut: [number, number][] = [];
  for (let at = 0; at < query.length; at += 1) {
    const found = opens(query, at) && query.startsWith(chip.marker, at) ? named(chip, query, at + chip.marker.length) : undefined;
    if (found === undefined) continue;
    cut.push([at, found.end]);
    at = found.end;
  }
  return cut.reduceRight((text, [from, to]) => text.slice(0, from) + text.slice(to), query).replace(/\s+/g, " ").trimStart();
}

/**
 * The token being typed at the end of the words, and the options it could still become. None where the last token already
 * names one, or no option begins with what was typed.
 */
export function completing<Chip extends Marked>(query: string, chips: readonly Chip[]): { chip: Chip; from: number; options: PaletteChipOption[] } | undefined {
  let best: { chip: Chip; from: number; options: PaletteChipOption[] } | undefined;
  let latest = -1;
  for (const chip of chips) {
    if (chip.marker === undefined || chip.marker === "") continue;
    const at = query.lastIndexOf(chip.marker);
    if (at < 0 || at < latest || !opens(query, at)) continue;
    const from = at + chip.marker.length;
    const typed = query.slice(from).toLowerCase();
    const options = chip.options.filter((one) => one.label.toLowerCase().startsWith(typed) && one.label.toLowerCase() !== typed);
    if (options.length === 0) continue;
    best = { chip, from, options };
    latest = at;
  }
  return best;
}
