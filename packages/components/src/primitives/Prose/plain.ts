// A terminal's bytes as the plain text a `<pre>` shows: escape sequences dropped, a line break's carriage
// return with it, and a line a program redrew in place with a carriage return left as its last drawing.

const ESC = String.fromCharCode(27);
const BEL = String.fromCharCode(7);
const SEQUENCE = new RegExp(`${ESC}(?:\\[[0-?]*[ -/]*[@-~]|\\][^${BEL}${ESC}]*(?:${BEL}|${ESC}\\\\)|[ -/][ -~]|[@-Z\\\\^_=>])|${BEL}`, "g");

export function plainOf(bytes: string): string {
  return bytes
    .replace(SEQUENCE, "")
    .replaceAll(ESC, "")
    .replaceAll("\r\n", "\n")
    .split("\n")
    .map((line) => line.slice(line.lastIndexOf("\r") + 1))
    .join("\n");
}
