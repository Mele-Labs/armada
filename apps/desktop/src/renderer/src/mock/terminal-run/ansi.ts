// What a terminal's screen reads as in plain text, for the tail the mock's Fleet tells a Session of a run
// (`plain::last_lines` in Fleet): the few escapes a prompt library sends, and the controls that move the
// cursor. Cells are kept so a prompt rewritten in place (`\r`, erase to the end of the line) reads as it does there.

type Style = { bold?: true; dim?: true; underline?: true; color?: "cyan" | "green" | "red" };
type Cell = { ch: string; style: Style };
type Screen = { lines: Cell[][]; cursor: { line: number; col: number } };

const ESC = "\x1b";

const without = (style: Style, ...keys: (keyof Style)[]): Style => Object.fromEntries(Object.entries(style).filter(([key]) => !keys.includes(key as keyof Style)));

/** The style a select-graphic-rendition code leaves a run in. */
function applied(code: number, style: Style): Style {
  if (code === 0) return {};
  if (code === 1) return { ...style, bold: true };
  if (code === 2) return { ...style, dim: true };
  if (code === 4) return { ...style, underline: true };
  if (code === 22) return without(style, "bold", "dim");
  if (code === 24) return without(style, "underline");
  if (code === 36) return { ...style, color: "cyan" };
  if (code === 32) return { ...style, color: "green" };
  if (code === 31) return { ...style, color: "red" };
  if (code === 39) return without(style, "color");
  return style;
}

function screenOf(out: string): Screen {
  const lines: Cell[][] = [[]];
  let line = 0;
  let col = 0;
  let style: Style = {};
  for (let at = 0; at < out.length; at += 1) {
    const ch = out[at]!;
    if (ch === ESC && out[at + 1] === "[") {
      let end = at + 2;
      while (end < out.length && !/[A-Za-z]/.test(out[end]!)) end += 1;
      const params = out.slice(at + 2, end);
      const final = out[end];
      if (final === "m") for (const code of params === "" ? ["0"] : params.split(";")) style = applied(Number(code), style);
      else if (final === "K") lines[line] = params === "2" ? [] : lines[line]!.slice(0, col);
      at = end;
    } else if (ch === "\r") col = 0;
    else if (ch === "\n") {
      line += 1;
      col = 0;
      lines[line] ??= [];
    } else if (ch === "\b") col = Math.max(0, col - 1);
    else {
      const cells = lines[line]!;
      while (cells.length < col) cells.push({ ch: " ", style: {} });
      cells[col] = { ch, style };
      col += 1;
    }
  }
  return { lines, cursor: { line, col } };
}

/** The last `count` lines of the screen as plain text, with no escapes and none of the blank lines at the end. */
export function tailOf(out: string, count: number): string {
  const text = screenOf(out).lines.map((cells) => cells.map((cell) => cell.ch).join("").trimEnd());
  while (text.length > 0 && text[text.length - 1] === "") text.pop();
  return text.slice(-count).join("\n");
}
