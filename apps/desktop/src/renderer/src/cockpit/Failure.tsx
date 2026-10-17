// What went wrong on a Job, drawn on its call so the owner need not open the Job to see it: the tests
// a Check failed by name and the tail of its output, a ruling and the Judge's reason, how long a Drone
// has been quiet and what it last did, or what main or another Job has in the same files.

import { useLayoutEffect, useRef, useState } from "react";
import { Clock, X } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { CallIssueView } from "@armada/screens/src/draft/calls";

/** The output a card draws is its last lines, which scroll. */
const OUTPUT_LINES = 12;

/** Whether the issue carries anything to draw beyond its one line. */
export const hasFailure = (issue: CallIssueView): boolean =>
  issue.failing !== undefined || issue.output !== undefined || issue.ruling !== undefined || issue.quiet !== undefined || issue.clash !== undefined;

/** The line of the output that names a test: by its whole name, else without its crate, which the log leaves off. */
function lineOf(lines: readonly string[], test: string): number {
  const bare = test.split("::").slice(1).join("::");
  return lines.findIndex((one) => one.includes(test) || (bare !== "" && one.includes(bare)));
}

export function Failure({ issue }: { issue: CallIssueView }) {
  const { failing, output, ruling, quiet, clash } = issue;
  const lines = (output ?? []).slice(-OUTPUT_LINES);
  // The output opens at its last line, which is where the failure is.
  const tail = useRef<HTMLPreElement>(null);
  useLayoutEffect(() => {
    if (tail.current !== null) tail.current.scrollTop = tail.current.scrollHeight;
  }, [output]);
  // A test pressed marks the first line of the output that names it, and brings it into view. The mark stays until another is pressed.
  const [marked, setMarked] = useState<{ at: number; press: number }>();
  const show = (test: string) => {
    const at = lineOf(lines, test);
    if (at < 0) return;
    setMarked((was) => ({ at, press: (was?.press ?? 0) + 1 }));
    const line = tail.current?.children[at] as HTMLElement | undefined;
    if (tail.current !== null && line !== undefined) tail.current.scrollTop = line.offsetTop - tail.current.clientHeight / 2 + line.offsetHeight / 2;
  };
  return (
    <div className="armada-failure">
      {failing === undefined || failing.length === 0 ? null : (
        <ul className="armada-failure__tests" aria-label="Failing tests">
          {failing.map((one) => (
            <li key={one}>
              <button type="button" className="armada-failure__test" onClick={() => show(one)}>
                <Tooltip label="Failed">
                  <span role="img" aria-label="Failed" className="armada-failure__mark">
                    <X size={12} aria-hidden="true" />
                  </span>
                </Tooltip>
                <code>{one}</code>
              </button>
            </li>
          ))}
        </ul>
      )}
      {lines.length === 0 ? null : (
        <pre ref={tail} className="armada-failure__output" tabIndex={0} aria-label="Output">
          {lines.map((one, at) => (
            <span key={at} className="armada-failure__line" data-marked={marked?.at === at ? marked.press : undefined}>
              {one === "" ? " " : one}
            </span>
          ))}
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
