// What a shell action printed, in place of the flight controls' tiles until Close, as `HoldsReleasedBox` stands for a release. The line ran
// in the Job's slot through Fleet (`run_session_shell`), which answers once it has ended, so nothing is sent to the agent and nothing
// prints while it runs. The owner's ask of 18 Oct 2026.

import { Button } from "@armada/components";
import type { SessionShellRan } from "@armada/protocol";

/** The last bytes Fleet keeps of what a line printed, as it says so. */
const KEPT = "64 KiB";

/** The line, and how it ended: `ran` once it has, `said` where Fleet refused it or was not reached, neither while it runs. */
export type ShellRun = { line: string; ran?: SessionShellRan | undefined; said?: string | undefined };

// eslint-disable-next-line no-control-regex
const PLAIN = /\u001b\[[0-9;?]*[ -/]*[@-~]/g;

export function ShellBox({ run, onClose }: { run: ShellRun; onClose: () => void }) {
  const { ran, said } = run;
  const ended = ran !== undefined || said !== undefined;
  return (
    <div className="deck-controls__block deck-shell" data-subject="session" role="region" aria-label="Shell output" aria-busy={!ended || undefined}>
      <pre className="deck-shell__out" tabIndex={0} aria-label={run.line}>
        <span className="deck-shell__line">{`$ ${run.line}\n`}</span>
        {ran?.truncated === true ? <span className="deck-shell__code">{`truncated to the last ${KEPT}`}</span> : null}
        {ran === undefined ? null : ran.output.replace(PLAIN, "")}
        {ran?.timed_out === true ? (
          <span className="deck-shell__code" data-failed>
            timed out
          </span>
        ) : null}
        {ran?.exit === undefined ? null : (
          <span className="deck-shell__code" data-failed={ran.exit !== 0 || undefined}>{`exit ${ran.exit}`}</span>
        )}
        {said === undefined ? null : (
          <span className="deck-shell__code" data-failed>
            {said}
          </span>
        )}
      </pre>
      <Button variant="secondary" size="sm" onClick={onClose}>
        Close
      </Button>
    </div>
  );
}
