import { useEffect, useLayoutEffect, useRef } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal, type ITheme } from "@xterm/xterm";

export type TerminalScreenProps = {
  /** Everything the run has printed, as the bytes it printed them. It only grows within a run. */
  out: string;
  /** Which run `out` is of: a change starts the screen over. */
  run: number;
  running: boolean;
  /** Keys, as a terminal sends them. */
  write: (data: string) => void;
  /** The screen's size in cells, told after each fit: the first is the size a run starts at. */
  onSize: (cols: number, rows: number) => void;
};

/** A token's colour as the browser computes it, which is what xterm is given. */
function colorOf(host: HTMLElement, token: string): string {
  const probe = document.createElement("span");
  probe.style.color = `var(${token})`;
  host.appendChild(probe);
  const value = getComputedStyle(probe).color;
  probe.remove();
  return value;
}

/** The screen's colours from the tokens. The tokens hold no terminal palette, so the four hues a run takes are the run's own and the rest are xterm's. */
function themeOf(host: HTMLElement): ITheme {
  const color = (token: string) => colorOf(host, token);
  return {
    background: color("--bg-sunken"),
    foreground: color("--fg-default"),
    cursor: color("--fg-default"),
    cursorAccent: color("--bg-sunken"),
    selectionBackground: color("--accent-muted"),
    black: color("--bg-raised"),
    brightBlack: color("--fg-subtle"),
    red: color("--run-failed"),
    brightRed: color("--run-failed"),
    green: color("--run-passed"),
    brightGreen: color("--run-passed"),
    blue: color("--accent"),
    brightBlue: color("--accent-hover"),
    cyan: color("--run-running"),
    brightCyan: color("--run-running"),
  };
}

const HIDE_CURSOR = "\x1b[?25l";

/**
 * **The terminal's screen, drawn by xterm.js.** It takes focus as it opens and on a press, sends what is
 * typed to `write`, tells `onSize` its size in cells whenever it fits the pane, and scrolls to the end
 * as output arrives. xterm keeps the keys it takes from the window's shortcuts, and a Cmd chord is not
 * one it takes. When the run ends it lets go of focus.
 */
export function XtermScreen({ out, run, running, write, onSize }: TerminalScreenProps) {
  const host = useRef<HTMLDivElement>(null);
  const screen = useRef<Terminal | null>(null);
  const shown = useRef({ run, length: 0, live: false });
  // Writes of history still being parsed. A query in history was answered when it was printed, so its reply now would be typed into the shell.
  const redrawing = useRef(0);
  const latest = useRef({ write, onSize, running });
  latest.current = { write, onSize, running };

  useLayoutEffect(() => {
    const element = host.current;
    if (element === null) return;
    const style = getComputedStyle(element);
    const size = parseFloat(style.getPropertyValue("--text-xs"));
    const line = parseFloat(style.getPropertyValue("--leading-xs"));
    const family = style.getPropertyValue("--font-mono");
    const terminal = new Terminal({
      fontFamily: family,
      fontSize: size,
      lineHeight: line / size,
      fontWeightBold: style.getPropertyValue("--weight-heading").trim() as "600",
      theme: themeOf(element),
      cursorBlink: true,
      cursorInactiveStyle: "outline",
      scrollback: 10_000,
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(element);
    screen.current = terminal;

    let over = false;
    let told: { cols: number; rows: number } | null = null;
    const refit = () => {
      if (over || element.clientWidth === 0 || element.clientHeight === 0) return;
      fit.fit();
      if (told?.cols === terminal.cols && told.rows === terminal.rows) return;
      told = { cols: terminal.cols, rows: terminal.rows };
      latest.current.onSize(terminal.cols, terminal.rows);
    };
    refit();
    const watching = new ResizeObserver(refit);
    watching.observe(element);
    // A face that loads after the cell was measured leaves the grid the width of the one before it.
    void document.fonts?.load(`${size}px ${family}`).then(() => {
      if (over) return;
      terminal.options.fontFamily = `${family}, monospace`;
      refit();
    });

    const data = terminal.onData((typed) => {
      if (latest.current.running && redrawing.current === 0) latest.current.write(typed);
    });
    const retheme = new MutationObserver(() => {
      terminal.options.theme = themeOf(element);
    });
    retheme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    if (latest.current.running) terminal.focus();

    return () => {
      over = true;
      retheme.disconnect();
      watching.disconnect();
      data.dispose();
      terminal.dispose();
      screen.current = null;
      shown.current = { run: -1, length: 0, live: false };
    };
    // The screen is made once; what changes after is told to it below.
  }, []);

  useEffect(() => {
    const terminal = screen.current;
    if (terminal === null) return;
    // What a screen is first given, or given again after it starts over on the same run, is history; a new run's bytes are not.
    if (shown.current.run !== run || out.length < shown.current.length) {
      terminal.reset();
      shown.current = { run, length: 0, live: shown.current.run !== run && shown.current.run !== -1 };
    }
    if (out.length > shown.current.length) {
      const chunk = out.slice(shown.current.length);
      if (shown.current.live) terminal.write(chunk);
      else {
        redrawing.current += 1;
        terminal.write(chunk, () => {
          redrawing.current -= 1;
        });
      }
      shown.current.length = out.length;
    }
    shown.current.live = true;
  }, [out, run]);

  useEffect(() => {
    const terminal = screen.current;
    if (terminal === null) return;
    terminal.options.disableStdin = !running;
    terminal.options.cursorBlink = running;
    if (running) terminal.focus();
    else {
      terminal.write(HIDE_CURSOR);
      // Keys are the window's again once nothing is listening for them.
      terminal.blur();
    }
  }, [running, run]);

  return <div ref={host} className="armada-xterm" onClick={() => screen.current?.focus()} />;
}
