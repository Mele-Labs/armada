// A terminal's output as plain text for a `<pre>`: escape sequences and carriage returns dropped.

const ESC = String.fromCharCode(27);
const BEL = String.fromCharCode(7);
const SEQUENCE = new RegExp(
  `${ESC}(?:\\[[0-?]*[ -/]*[@-~]|\\][^${BEL}${ESC}]*(?:${BEL}|${ESC}\\\\)|[ -/][ -~]|[@-Z\\\\^_=>])|[\\r${BEL}]`,
  "g",
);
/** More than this after an ESC is not a sequence still arriving. */
const LONGEST = 256;

/**
 * A stripper for chunks of one run. **A sequence that a chunk ends inside is held for the next**,
 * so a colour code split across two reads does not print as `[31m`.
 */
export function ansiStripper(): (chunk: string) => string {
  let held = "";
  return (chunk) => {
    const plain = (held + chunk).replace(SEQUENCE, "");
    const open = plain.lastIndexOf(ESC);
    if (open === -1 || plain.length - open > LONGEST) {
      held = "";
      return plain.replaceAll(ESC, "");
    }
    held = plain.slice(open);
    return plain.slice(0, open);
  };
}
