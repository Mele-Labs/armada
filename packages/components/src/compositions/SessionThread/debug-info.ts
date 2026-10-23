// Where a message's debug facts begin. A message that carries a Job's facts puts them under this heading, so the agent reads plain
// text and the thread draws the facts folded. A message without it is drawn whole.

/** The heading line, as the agent reads it. */
export const DEBUG_INFO_HEADING = "## Debug info";

/** The words before the heading, and what follows it, which is `undefined` where there is no heading. */
export function splitDebugInfo(text: string): { said: string; debug: string | undefined } {
  const at = text.split("\n").findIndex((line) => line.trimEnd() === DEBUG_INFO_HEADING);
  if (at < 0) return { said: text, debug: undefined };
  const lines = text.split("\n");
  return { said: lines.slice(0, at).join("\n").trimEnd(), debug: lines.slice(at + 1).join("\n").trim() };
}
