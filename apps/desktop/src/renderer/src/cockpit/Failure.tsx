// What went wrong on a Job, drawn on its call so the owner need not open the Job to see it: the tests
// a Check failed by name and the tail of its output, a ruling and the Judge's reason, how long a Drone
// has been quiet and what it last did, or what main or another Job has in the same files.

import { useLayoutEffect, useRef } from "react";
import { Clock } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { CallIssueView } from "@armada/jobs/draft/calls";

/** The output a card draws is its last lines, which scroll. */
const OUTPUT_LINES = 12;

/** Whether the issue carries anything to draw beyond its one line. */
export const hasFailure = (issue: CallIssueView): boolean =>
  issue.failing !== undefined || issue.output !== undefined || issue.ruling !== undefined || issue.quiet !== undefined || issue.clash !== undefined;

export function Failure({ issue }: { issue: CallIssueView }) {
  const { failing, output, ruling, quiet, clash } = issue;
  // The output opens at its last line, which is where the failure is.
  const tail = useRef<HTMLPreElement>(null);
  useLayoutEffect(() => {
    if (tail.current !== null) tail.current.scrollTop = tail.current.scrollHeight;
  }, [output]);
  return (
    <div className="armada-failure">
      {failing === undefined || failing.length === 0 ? null : (
        <ul className="armada-failure__list" aria-label="Failing tests">
          {failing.map((one) => (
            <li key={one}>{one}</li>
          ))}
        </ul>
      )}
      {output === undefined || output.length === 0 ? null : (
        <pre ref={tail} className="armada-failure__output" tabIndex={0} aria-label="Output">
          {output.slice(-OUTPUT_LINES).join("\n")}
        </pre>
      )}
      {ruling === undefined ? null : (
        <>
          <dl className="armada-failure__facts">
            <div>
              <dt>Attempt</dt>
              <dd>{ruling.attempt}</dd>
            </div>
            <div>
              <dt>Trigger</dt>
              <dd>{ruling.trigger}</dd>
            </div>
          </dl>
          <p className="armada-failure__reason">{ruling.reason}</p>
        </>
      )}
      {quiet === undefined ? null : (
        <>
          <p className="armada-failure__quiet">
            <Tooltip label={`Quiet for ${quiet.minutes} minutes`}>
              <span role="img" aria-label={`Quiet for ${quiet.minutes} minutes`}>
                <Clock size={12} aria-hidden="true" />
                {quiet.minutes}m
              </span>
            </Tooltip>
          </p>
          <ol className="armada-failure__list" aria-label="Last actions">
            {quiet.last.map((one) => (
              <li key={one}>{one}</li>
            ))}
          </ol>
        </>
      )}
      {clash === undefined ? null : (
        <>
          <p className="armada-failure__reason">{clash.what}</p>
          <ul className="armada-failure__list" aria-label="Paths">
            {clash.paths.map((one) => (
              <li key={one}>{one}</li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
