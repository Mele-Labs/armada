// What a shell action printed, in place of the flight controls' tiles until Close, as `HoldsReleasedBox` stands for a release. The line ran
// in the Job's bay through the Session's own runner (`draft.run`), the one a thread's code block uses, so nothing is sent to the agent.
// The owner's ask of 18 Oct 2026.

import { useEffect, useRef } from "react";
import { Button } from "@armada/components";

/** What the run has printed so far, and how it ended: `undefined` while it runs, a code once it has, `null` where it was stopped. */
export type ShellRun = { line: string; output: string; code: number | null | undefined };

// eslint-disable-next-line no-control-regex
const PLAIN = /\u001b\[[0-9;?]*[ -/]*[@-~]/g;

export function ShellBox({ run, onClose }: { run: ShellRun; onClose: () => void }) {
  const end = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const out = end.current;
    if (out !== null) out.scrollTop = out.scrollHeight;
  }, [run.output]);
  const ended = run.code !== undefined;
  return (
    <div className="deck-controls__block deck-shell" data-subject="session" role="region" aria-label="Shell output" aria-busy={!ended || undefined}>
      <pre ref={end} className="deck-shell__out" tabIndex={0} aria-label={run.line}>
        <span className="deck-shell__line">{`$ ${run.line}\n`}</span>
        {run.output.replace(PLAIN, "")}
        {run.code === undefined || run.code === null ? null : <span className="deck-shell__code" data-failed={run.code !== 0 || undefined}>{`exit ${run.code}`}</span>}
      </pre>
      <Button variant="secondary" size="sm" onClick={onClose}>
        Close
      </Button>
    </div>
  );
}
