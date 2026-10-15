// The terminal's screen, drawn by hand from the bytes the scripted program printed. Mock-only: the real
// build replaces this one component with xterm.js and nothing around it changes (`TerminalPane` takes the
// body as children, and the runner carries `write`). Keys go to the program as a terminal sends them, it
// takes focus when it opens, and a block caret stands where the next character will go.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import type { TerminalScreenProps } from "@armada/components";

import { screenOf, type Cell, type Style } from "./ansi";
import { KEY } from "./wrangler";
import "./fake-screen.css";

/** The bytes a terminal sends for a key, or null where it sends none. */
function dataOf(event: KeyboardEvent): string | null {
  if (event.metaKey || event.altKey) return null;
  if (event.ctrlKey) return event.key.toLowerCase() === "c" ? KEY.interrupt : null;
  if (event.key === "Enter") return KEY.enter;
  if (event.key === "Backspace") return KEY.backspace;
  return event.key.length === 1 ? event.key : null;
}

type Run = { text: string; style: Style; caret: boolean };

/** A line as runs of one style, with the caret's cell a run of its own. */
function runsOf(cells: readonly Cell[], caretAt: number | null): Run[] {
  const all = [...cells];
  if (caretAt !== null) while (all.length <= caretAt) all.push({ ch: " ", style: {} });
  const runs: Run[] = [];
  all.forEach((cell, at) => {
    const caret = at === caretAt;
    const last = runs[runs.length - 1];
    if (last !== undefined && !caret && !last.caret && JSON.stringify(last.style) === JSON.stringify(cell.style)) last.text += cell.ch;
    else runs.push({ text: cell.ch, style: cell.style, caret });
  });
  return runs;
}

export function FakeScreen({ out, running, write }: TerminalScreenProps) {
  const body = useRef<HTMLDivElement>(null);
  const [focused, setFocused] = useState(false);
  const screen = useMemo(() => screenOf(out), [out]);
  // It takes focus as it opens, and again when a run is started in it.
  useEffect(() => {
    if (running) body.current?.focus({ preventScroll: true });
  }, [running]);
  useLayoutEffect(() => {
    const one = body.current;
    if (one !== null) one.scrollTop = one.scrollHeight;
  }, [out]);

  const pressed = (event: KeyboardEvent<HTMLDivElement>) => {
    const data = running ? dataOf(event) : null;
    if (data === null) return;
    // A key typed into the terminal is the program's, and no shortcut of the window's.
    event.preventDefault();
    event.stopPropagation();
    write(data);
  };

  return (
    <div
      ref={body}
      className="mock-terminal"
      role="textbox"
      aria-multiline
      aria-readonly={!running || undefined}
      aria-label="Terminal input"
      tabIndex={0}
      data-focused={focused || undefined}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      onKeyDown={pressed}
    >
      {screen.lines.map((cells, at) => (
        <div key={at} className="mock-terminal__line">
          {runsOf(cells, running && at === screen.cursor.line ? screen.cursor.col : null).map((run, index) => (
            <span
              key={index}
              className={run.caret ? "mock-terminal__caret" : "mock-terminal__run"}
              data-color={run.style.color}
              data-bold={run.style.bold}
              data-dim={run.style.dim}
              data-underline={run.style.underline}
            >
              {run.text}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}
