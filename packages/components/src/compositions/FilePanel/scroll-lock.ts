/** A block of the preview: the source line it starts on and where it sits in the scrolled content. The last is the end of the text. */
export type Mark = { line: number; top: number };

/** Where `at` falls between the marks, as the mark before it and how far it is toward the next, 0 to 1. */
function between(marks: readonly Mark[], at: number, key: "line" | "top"): { from: Mark; next: Mark | undefined; share: number } | undefined {
  const first = marks[0];
  if (first === undefined) return undefined;
  let index = 0;
  while (index + 1 < marks.length && marks[index + 1]![key] <= at) index += 1;
  const from = marks[index]!;
  const next = marks[index + 1];
  const span = next === undefined ? 0 : next[key] - from[key];
  return { from, next, share: span <= 0 ? 0 : Math.min(1, Math.max(0, (at - from[key]) / span)) };
}

/** Where in the preview a source line is: a line inside a block is in proportion between it and the next. */
export function topForLine(marks: readonly Mark[], line: number): number {
  const found = between(marks, line, "line");
  if (found === undefined) return 0;
  return found.next === undefined ? found.from.top : found.from.top + found.share * (found.next.top - found.from.top);
}

/** The source line at a place in the preview, the other way. */
export function lineForTop(marks: readonly Mark[], top: number): number {
  const found = between(marks, top, "top");
  if (found === undefined) return 1;
  return found.next === undefined ? found.from.line : found.from.line + found.share * (found.next.line - found.from.line);
}
