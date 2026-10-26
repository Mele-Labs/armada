import { useEffect, useState } from "react";
import { Check, Copy, Play, RotateCw, Square } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { XtermScreen } from "../../primitives/TerminalPane/XtermScreen";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A command's run as the host holds it: what it printed, and whether it still goes. */
export type PullRequestFixRun = {
  out: string;
  /** Which run `out` is of: a new run starts the screen over. */
  screen: number;
  running: boolean;
  write: (data: string) => void;
  onSize: (cols: number, rows: number) => void;
  onStop: () => void;
};

export type PullRequestFixProps = {
  /** The command the refusal names, as it would be typed. */
  command: string;
  onRun: () => void;
  /** Fleet's refusal to run it, said under it. */
  said?: string;
  run?: PullRequestFixRun;
  /** Offered once the run ends: reads the pull request again. */
  onAgain: () => void;
};

function CopyCommand({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);
  const said = copied ? "Copied" : "Copy command";
  return (
    <Tooltip label={said}>
      <Button variant="ghost" size="sm" iconOnly aria-label="Copy command" onClick={() => void navigator.clipboard.writeText(command).then(() => setCopied(true))}>
        {copied ? <Check size={16} strokeWidth={2} aria-hidden /> : <Copy size={16} strokeWidth={2} aria-hidden />}
      </Button>
    </Tooltip>
  );
}

/** The step a refusal names, beside the sentence that names it: run it here, with its output under it, as in a terminal. */
export function PullRequestFix({ command, onRun, said, run, onAgain }: PullRequestFixProps) {
  const ended = run !== undefined && !run.running;
  return (
    <div className="armada-pr-panel__fix">
      <div className="armada-pr-panel__acts">
        <code className="armada-pr-panel__command mono">{command}</code>
        <CopyCommand command={command} />
        {run?.running ? (
          <Button variant="secondary" size="sm" onClick={run.onStop}>
            <Square size={16} strokeWidth={2} aria-hidden />
            Stop
          </Button>
        ) : (
          <Button variant="primary" size="sm" onClick={onRun}>
            <Play size={16} strokeWidth={2} aria-hidden />
            {ended ? "Run again" : "Run"}
          </Button>
        )}
        {ended ? (
          <Button variant="secondary" size="sm" onClick={onAgain}>
            <RotateCw size={16} strokeWidth={2} aria-hidden />
            Open the pull request again
          </Button>
        ) : null}
      </div>
      {said === undefined || said === "" ? null : (
        <p className="armada-pr-panel__said" role="alert">
          {said}
        </p>
      )}
      {run === undefined ? null : (
        <div className="armada-pr-panel__run" {...(run.running ? { "data-keeps-esc": "" } : {})}>
          <XtermScreen key={run.screen} out={run.out} run={run.screen} running={run.running} write={run.write} onSize={run.onSize} />
        </div>
      )}
    </div>
  );
}
