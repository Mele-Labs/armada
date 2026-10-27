import { createContext, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { Check, ChevronDown, CircleCheck, CircleX, Copy, Play, RotateCcw, Square, SquareTerminal } from "lucide-react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";
import { endedSaid } from "../TerminalPane/TerminalPane";
import { DockedMark, InlineTerminal, ProseTerminal } from "./TerminalRun";
import { plainOf } from "./plain";
import { begin, change, entryFor, newEntry, subscribeTo, type RunEntry } from "./runs";

/** What a run says: a chunk of the terminal's bytes, escapes intact, then how it ended: with an exit code, or with none, where `stopped` says whether Stop ended it or a signal did. */
export type CodeRunEvent = { kind: "out"; text: string } | { kind: "end"; code: number } | { kind: "end"; code: null; stopped: boolean };

/** A terminal's size in cells. */
export type TerminalSize = { cols: number; rows: number };

/**
 * Runs a command where the Session works and reports back. The person's own act, so nothing
 * here reaches the agent, unless Bridge's runner settles a waiting item that asked for the command.
 * Bridge provides one per Session; absent, a block draws without Run. The run is on a terminal:
 * `size` is the screen it starts at, `write` sends it keys and `resize` tells it the screen changed.
 */
export type CodeRunner = (
  command: string,
  on: (event: CodeRunEvent) => void,
  size?: TerminalSize,
) => { stop: () => void; write?: (data: string) => void; resize?: (cols: number, rows: number) => void };

export const ProseRunner = createContext<CodeRunner | null>(null);

const SHELLS = new Set(["sh", "bash", "zsh", "shell", "console"]);

const COMMON = new Set([
  "cd", "ls", "git", "gh", "pnpm", "npm", "npx", "yarn", "cargo", "armada", "make", "docker", "kubectl", "brew", "node",
  "python", "python3", "pip", "curl", "cat", "echo", "grep", "rg", "rm", "mv", "cp", "mkdir", "touch", "chmod", "sudo",
  "tail", "head", "find", "sed", "awk", "xargs", "kill", "ps", "sh", "bash", "zsh", "source", "export",
]);
/** A first word that opens a line of code, not of a command. `export` stays: it is shell too. */
const CODE_WORDS = new Set([
  "const", "let", "var", "function", "def", "class", "import", "from", "return", "fn", "pub", "use", "struct", "enum",
  "impl", "interface", "package", "public", "private", "type", "async", "await",
]);

/**
 * Whether a block with no language reads as shell. Conservative: it is a run button on the person's
 * machine, so a doubt is no button. Every non-empty line must open like a command, after an optional
 * `$ `: a path with arguments or plainly executable, an assignment, or a lower-case word that is a
 * common command or carries a flag, path or quote. A line after `\`, `&&`, `||` or `|` is a
 * continuation and is let through. Code, JSON, prose (a capital, or a letter then `.`/`?`/`!` at the
 * end) and diagrams (box-drawing, arrows) fail.
 */
export function readsAsShell(source: string): boolean {
  if (/[\u2190-\u21ff\u2500-\u257f\u25a0-\u25ff]|=>|->|<-/.test(source)) return false;
  const lines = source.split("\n").filter((line) => line.trim() !== "");
  if (lines.length === 0) return false;
  let continued = false;
  for (const raw of lines) {
    const line = raw.trim().replace(/^\$ /, "");
    const was = continued;
    continued = /(\\|&&|\|\||\|)$/.test(line);
    if (line.startsWith("#")) continue;
    if (/(^|[^\\]);$|[{},]$|^[{}[\]"]/.test(line)) return false;
    if (/[A-Za-z0-9)]\.$|[?!]$/.test(line)) return false;
    if (was) continue;
    if (/^\S+ = /.test(line)) return false;
    const [first = "", ...rest] = line.split(/\s+/);
    if (/^[A-Za-z_][A-Za-z0-9_]*=/.test(first)) continue;
    // A path opens a command with arguments, or on its own is one that is plainly run: a list of files is not.
    if (/^(\.{0,2}\/|~\/|\/|[\w.@-]+\/)[\w./@~-]+$/.test(first)) {
      if (rest.length > 0 || /^\.\/|^(scripts|bin)\/|\.sh$/.test(first)) continue;
      return false;
    }
    if (!/^[a-z][a-z0-9._+-]*$/.test(first) || CODE_WORDS.has(first)) return false;
    if (COMMON.has(first)) continue;
    if (!rest.some((word) => /^-|[/.~=$"'*@]/.test(word))) return false;
  }
  return !continued;
}

/**
 * The command a fenced block holds, or null where it is not a shell block. A `console` block loses
 * its `$ ` prompts. A block with no language is one where `readsAsShell`; any other language is not.
 */
export function commandOf(language: string | undefined, source: string): string | null {
  const body = source.replace(/\n$/, "");
  if (language === undefined) return readsAsShell(body) ? stripPrompts(body) : null;
  if (!SHELLS.has(language)) return null;
  return language === "console" ? stripPrompts(body) : body;
}

const stripPrompts = (body: string) =>
  body
    .split("\n")
    .map((line) => line.replace(/^\$ ?/, ""))
    .join("\n");

export type CodePhase = { state: "idle" } | { state: "running" } | { state: "ended"; code: number | null; stopped?: boolean };

/** One run's state: what it has printed and how it stands. `RunnableBlock` and a card that runs a command from its own key both draw it. */
export type CodeRun = {
  phase: CodePhase;
  /** Everything it has printed, as the terminal's bytes. */
  out: string;
  /** How many runs this has started: what `out` is of. */
  runs: number;
  open: boolean;
  setOpen: (open: boolean) => void;
  /** Starts the command, stopping one already going, on a terminal of `size`. */
  start: (command: string, size?: TerminalSize) => void;
  stop: () => void;
  /** Keys for the run's terminal. */
  write: (data: string) => void;
  /** The terminal's screen changed size. */
  resize: (cols: number, rows: number) => void;
  /** Puts a run that has ended away: back to idle, with nothing printed. */
  close: () => void;
};

/**
 * With a `key`, the run is held outside the card by it: an unmount does not stop it, a card that mounts again
 * under the same key finds it, and only Stop or a new start ends it. Without one nothing could find the run
 * again, so the card's unmount stops it.
 */
export function useCodeRun(runner: CodeRunner | null | undefined, key?: string): CodeRun {
  const own = useRef<RunEntry | null>(null);
  const entry = key === undefined ? (own.current ??= newEntry()) : entryFor(key);
  const { phase, out, runs, open } = useSyncExternalStore(useMemo(() => subscribeTo(entry), [entry]), () => entry.view);
  useEffect(() => (key === undefined ? () => entry.handle?.stop() : undefined), [entry, key]);
  return {
    phase,
    out,
    runs,
    open,
    setOpen: (next) => change(entry, { open: next }),
    start: (command, size) => {
      if (runner !== null && runner !== undefined) begin(entry, runner, command, size);
    },
    stop: () => entry.handle?.stop(),
    write: (data) => entry.handle?.write?.(data),
    resize: (cols, rows) => entry.handle?.resize?.(cols, rows),
    close: () => change(entry, { phase: { state: "idle" }, out: "" }),
  };
}

/** The panel under a run: its mark (running, or the exit code), a fold, and the output scrolled to its end. */
export function RunOutput({ run }: { run: CodeRun }) {
  const { phase, out, open, setOpen } = run;
  const log = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const one = log.current;
    if (one !== null) one.scrollTop = one.scrollHeight;
  }, [out]);
  if (phase.state === "idle") return null;
  return (
    <div className="armada-prose__run-panel" role="group" aria-label="Run">
      <div className="armada-prose__run-head">
        {phase.state === "running" ? (
          <Tooltip label="Running">
            <span className="armada-prose__run-mark" data-result="running" role="img" aria-label="Running">
              <SquareTerminal size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : (
          <Tooltip label={endedSaid(phase)}>
            <span className="armada-prose__run-mark" data-result={phase.code === 0 ? "passed" : "failed"} role="img" aria-label={phase.code === 0 ? "Passed" : phase.code === null && phase.stopped !== false ? "Stopped" : "Failed"}>
              {phase.code === 0 ? <CircleCheck size={16} strokeWidth={2} aria-hidden /> : <CircleX size={16} strokeWidth={2} aria-hidden />}
            </span>
          </Tooltip>
        )}
        <Tooltip label={open ? "Collapse output" : "Show output"}>
          <Button variant="ghost" size="sm" iconOnly aria-label={open ? "Collapse output" : "Show output"} aria-expanded={open} onClick={() => setOpen(!open)}>
            <ChevronDown size={16} strokeWidth={2} aria-hidden className="armada-prose__run-chevron" data-open={open ? "" : undefined} />
          </Button>
        </Tooltip>
      </div>
      {open ? (
        <pre ref={log} className="armada-prose__run-out" role="log" aria-label="Output" tabIndex={0}>
          {plainOf(out)}
        </pre>
      ) : null}
    </div>
  );
}

/** How long Copy shows it has copied before it returns. */
const COPIED_FOR = 1500;

/** Writes a fenced block's source to the clipboard. The check is the confirmation, so a refused write leaves Copy as it was. `label` names the act where "Copy" alone is ambiguous. */
export function CopyAct({ text, label: named = "Copy" }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const label = copied ? "Copied" : named;
  return (
    <Tooltip label={label}>
      <Button
        variant="ghost"
        size="sm"
        iconOnly
        aria-label={label}
        onClick={() => {
          void navigator.clipboard.writeText(text).then(
            () => {
              setCopied(true);
              clearTimeout(timer.current);
              timer.current = setTimeout(() => setCopied(false), COPIED_FOR);
            },
            () => undefined,
          );
        }}
      >
        {copied ? <Check size={16} strokeWidth={2} aria-hidden /> : <Copy size={16} strokeWidth={2} aria-hidden />}
      </Button>
    </Tooltip>
  );
}

/** `copyLabel` names the copy key where one panel holds several blocks. `run` is a run held elsewhere, which a call card's own key drives too; left out, the block holds its own, kept past the block by `keep` where that is given. `copy` is the source a fenced block copies, beside Run. */
export function RunnableBlock({ command, runner, run: held, keep, copy, copyLabel, children }: { command: string; runner?: CodeRunner; run?: CodeRun; keep?: string; copy?: string; copyLabel?: string; children: ReactNode }) {
  const own = useCodeRun(runner, keep);
  const terminal = useContext(ProseTerminal);
  const run = held ?? own;
  // The view's pane holds one run, and a block draws it under itself only while it is a run of this command that a block started.
  const dock = held === undefined && terminal !== null ? terminal.dock : null;
  const block = useId();
  useLayoutEffect(() => dock?.register(block, command), [dock?.register, block, command]);
  const docked = dock !== null && dock.command === command && dock.claims(block) ? dock.run : null;
  const { phase } = dock === null ? run : (docked ?? { phase: { state: "idle" } as const });
  const running = phase.state === "running";
  const start = () => (dock === null ? run.start(command) : dock.start(command, block));
  const stop = dock === null ? run.stop : dock.run.stop;

  return (
    <div className="armada-prose__run" data-state={phase.state} data-docked={docked === null ? undefined : ""} role="group" aria-label={command.split("\n")[0]}>
      <div className="armada-prose__run-block">
        {children}
        {dock !== null && docked !== null && !dock.shown ? <DockedMark dock={dock} phase={phase} /> : null}
        <span className="armada-prose__run-acts">
          {copy === undefined ? null : <CopyAct text={copy} {...(copyLabel === undefined ? {} : { label: copyLabel })} />}
          {running ? (
            <Tooltip label="Stop">
              <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={stop}>
                <Square size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : phase.state === "idle" ? (
            <Tooltip label="Run">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run" onClick={start}>
                <Play size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <Tooltip label="Run again">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" onClick={start}>
                <RotateCcw size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </span>
      </div>
      {dock === null ? <RunOutput run={run} /> : docked === null ? null : <InlineTerminal dock={dock} />}
    </div>
  );
}
