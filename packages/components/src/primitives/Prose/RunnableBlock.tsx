import { createContext, useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown, CircleCheck, CircleX, Play, RotateCcw, Square, SquareTerminal } from "lucide-react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";

/** What a run says: a chunk of output, then how it ended. `code` is null where it was stopped. */
export type CodeRunEvent = { kind: "out"; text: string } | { kind: "end"; code: number | null };

/**
 * Runs a command where the Session works and reports back. The person's own act, so nothing
 * here reaches the agent. Bridge provides one per Session; absent, a block draws without Run.
 */
export type CodeRunner = (command: string, on: (event: CodeRunEvent) => void) => { stop: () => void };

export const ProseRunner = createContext<CodeRunner | null>(null);

const SHELLS = new Set(["sh", "bash", "zsh", "shell", "console"]);

/** The command a fenced block holds, or null where it is not a shell block. A `console` block loses its `$ ` prompts. */
export function commandOf(language: string | undefined, source: string): string | null {
  if (language === undefined || !SHELLS.has(language)) return null;
  const body = source.replace(/\n$/, "");
  if (language !== "console") return body;
  return body
    .split("\n")
    .map((line) => line.replace(/^\$ ?/, ""))
    .join("\n");
}

type Phase = { state: "idle" } | { state: "running" } | { state: "ended"; code: number | null };

export function RunnableBlock({ command, runner, children }: { command: string; runner: CodeRunner; children: ReactNode }) {
  const [phase, setPhase] = useState<Phase>({ state: "idle" });
  const [out, setOut] = useState("");
  const [open, setOpen] = useState(true);
  const handle = useRef<{ stop: () => void } | null>(null);
  const log = useRef<HTMLPreElement>(null);
  useEffect(() => () => handle.current?.stop(), []);
  useEffect(() => {
    const one = log.current;
    if (one !== null) one.scrollTop = one.scrollHeight;
  }, [out]);

  const run = () => {
    handle.current?.stop();
    setOut("");
    setOpen(true);
    setPhase({ state: "running" });
    handle.current = runner(command, (event) => {
      if (event.kind === "out") setOut((had) => had + event.text);
      else setPhase({ state: "ended", code: event.code });
    });
  };
  const running = phase.state === "running";

  return (
    <div className="armada-prose__run" data-state={phase.state} role="group" aria-label={command.split("\n")[0]}>
      <div className="armada-prose__run-block">
        {children}
        <span className="armada-prose__run-acts">
          {running ? (
            <Tooltip label="Stop">
              <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={() => handle.current?.stop()}>
                <Square size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : phase.state === "idle" ? (
            <Tooltip label="Run">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run" onClick={run}>
                <Play size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <Tooltip label="Run again">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" onClick={run}>
                <RotateCcw size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </span>
      </div>
      {phase.state === "idle" ? null : (
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
      )}
    </div>
  );
}
