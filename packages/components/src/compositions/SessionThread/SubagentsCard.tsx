import { useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { ToolSubagent } from "@armada/protocol";

import { useNowWhile } from "../../clock";
import { WorkingMark } from "../../primitives/WorkingMark/WorkingMark";
import { elapsedWords } from "../SessionRetroRow/SessionRetroRow";
import { StepActivityMark } from "../StepActivityMark/StepActivityMark";
import { saidOf } from "./SubagentRow";
import { SessionStop } from "./SessionStop";

/** How long one has run, or nothing where its start will not parse. */
function ranFor(subagent: ToolSubagent, now: number): string | undefined {
  const from = Date.parse(subagent.since);
  return Number.isNaN(from) ? undefined : elapsedWords(now - from);
}

/**
 * The subagents still running, held at the foot of the thread while any is: the one live mark, folded to how many there are,
 * opened to each one's status, task, last call and time. A press on one opens its own thread. Gone once the last returns.
 */
export function SubagentsCard({
  running,
  onOpen,
  onStop,
}: {
  running: readonly ToolSubagent[];
  onOpen?: (id: string) => void;
  /** Ends the turn, and the subagents with it: a Stop beside the live mark, as the working line has. */
  onStop?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const now = useNowWhile(open);
  const said = running.length === 1 ? "1 subagent running" : `${running.length} subagents running`;
  // It opens upward from the foot, as Needs you does: up while folded, down to fold it again.
  const Chevron = open ? ChevronDown : ChevronUp;
  return (
    <section className="armada-subagents" aria-label="Subagents running">
      <div className="armada-subagents__head">
        <button
          type="button"
          className="armada-subagents__toggle"
          aria-label={said}
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <span className="armada-subagents__mark" role="img" aria-label="Running">
            <WorkingMark size={14} />
          </span>
          <span className="armada-subagents__said">{said}</span>
          <Chevron size={14} strokeWidth={2} aria-hidden className="armada-subagents__chevron" />
        </button>
        {onStop === undefined ? null : <SessionStop onStop={onStop} />}
      </div>
      {open ? (
        <ul className="armada-subagents__rows">
          {running.map((one) => {
            const ran = ranFor(one, now);
            const body = (
              <>
                <StepActivityMark activity="running" label="Running" says={saidOf(one)} />
                <span className="armada-subagents__line">
                  <span className="armada-subagents__task">{one.task}</span>
                  {ran === undefined ? null : <span className="armada-subagents__ran">{ran}</span>}
                </span>
                {one.latest === undefined ? null : (
                  <span className="armada-subagents__latest">{one.latest}</span>
                )}
              </>
            );
            return (
              <li key={one.id}>
                {onOpen === undefined ? (
                  <div className="armada-subagents__row">{body}</div>
                ) : (
                  <button
                    type="button"
                    className="armada-subagents__row"
                    aria-label={`Subagent ${one.task}`}
                    onClick={() => onOpen(one.id)}
                  >
                    {body}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
