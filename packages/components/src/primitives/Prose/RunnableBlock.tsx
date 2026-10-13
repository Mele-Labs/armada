import { createContext, useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown, CircleCheck, CircleX, Play, RotateCcw, Square, SquareTerminal } from "lucide-react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";

/** What a run says: a chunk of output, then how it ended. `code` is null where it was stopped. */
export type CodeRunEvent = { kind: "out"; text: string } | { kind: "end"; code: number | null };

/**
 * Runs a command where the Session works and reports back. The person's own act, so nothing
 * here reaches the agent, unless Bridge's runner settles a waiting item that asked for the command.
 * Bridge provides one per Session; absent, a block draws without Run.
 */
export type CodeRunner = (command: string, on: (event: CodeRunEvent) => void) => { stop: () => void };

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

export type CodePhase = { state: "idle" } | { state: "running" } | { state: "ended"; code: number | null };

/** One run's state: what it has printed and how it stands. `RunnableBlock` and a card that runs a command from its own key both draw it. */
export type CodeRun = {
  phase: CodePhase;
  out: string;
  open: boolean;
  setOpen: (open: boolean) => void;
  /** Starts the command, stopping one already going. */
  start: (command: string) => void;
  stop: () => void;
};

export function useCodeRun(runner: CodeRunner | null | undefined): CodeRun {
  const [phase, setPhase] = useState<CodePhase>({ state: "idle" });
  const [out, setOut] = useState("");
  const [open, setOpen] = useState(true);
  const handle = useRef<{ stop: () => void } | null>(null);
  useEffect(() => () => handle.current?.stop(), []);
  const start = (command: string) => {
    if (runner === null || runner === undefined) return;
    handle.current?.stop();
    setOut("");
    setOpen(true);
    setPhase({ state: "running" });
    handle.current = runner(command, (event) => {
      if (event.kind === "out") setOut((had) => had + event.text);
      else setPhase({ state: "ended", code: event.code });
    });
  };
  return { phase, out, open, setOpen, start, stop: () => handle.current?.stop() };
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
          <Tooltip label={phase.code === null ? "Stopped" : `Exit ${phase.code}`}>
            <span className="armada-prose__run-mark" data-result={phase.code === 0 ? "passed" : "failed"} role="img" aria-label={phase.code === 0 ? "Passed" : "Failed"}>
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
          {out}
        </pre>
      ) : null}
    </div>
  );
}

/** `run` is a run held elsewhere, which a call card's own key drives too; left out, the block holds its own. */
export function RunnableBlock({ command, runner, run: held, children }: { command: string; runner?: CodeRunner; run?: CodeRun; children: ReactNode }) {
  const own = useCodeRun(runner);
  const run = held ?? own;
  const { phase } = run;
  const running = phase.state === "running";

  return (
    <div className="armada-prose__run" data-state={phase.state} role="group" aria-label={command.split("\n")[0]}>
      <div className="armada-prose__run-block">
        {children}
        <span className="armada-prose__run-acts">
          {running ? (
            <Tooltip label="Stop">
              <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={run.stop}>
                <Square size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : phase.state === "idle" ? (
            <Tooltip label="Run">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run" onClick={() => run.start(command)}>
                <Play size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <Tooltip label="Run again">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" onClick={() => run.start(command)}>
                <RotateCcw size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </span>
      </div>
      <RunOutput run={run} />
    </div>
  );
}
