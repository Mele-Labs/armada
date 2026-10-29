import { ChevronRight } from "lucide-react";
import type { ToolSubagent } from "@armada/protocol";

import { useNowWhile } from "../../clock";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { elapsedWords } from "../SessionRetroRow/SessionRetroRow";

const MARK: Record<ToolSubagent["state"], { activity: StepActivity; said: string }> = {
  running: { activity: "running", said: "Running" },
  finished: { activity: "advanced", said: "Finished" },
  cut_off: { activity: "killed", said: "Cut off" },
};

/** How long it ran: to now while it runs, to `ended` once settled, and nothing where either end will not parse. */
function ranFor(subagent: ToolSubagent, now: number): string | undefined {
  const from = Date.parse(subagent.since);
  const to = subagent.state === "running" ? now : subagent.ended === undefined ? Number.NaN : Date.parse(subagent.ended);
  return Number.isNaN(from) || Number.isNaN(to) ? undefined : elapsedWords(to - from);
}

/**
 * The subagent an `Agent` call started, in place of its one tool line: its type, its task, its own last call and how
 * long it has run. **Its calls are never rows of the thread**, which would put its words in the agent's mouth. A press
 * opens its own thread.
 */
export function SubagentRow({ subagent, onOpen }: { subagent: ToolSubagent; onOpen?: (id: string) => void }) {
  const running = subagent.state === "running";
  const now = useNowWhile(running);
  const ran = ranFor(subagent, now);
  const mark = MARK[subagent.state];
  const body = (
    <>
      <span className="armada-session-subagent__head">
        <StepActivityMark activity={mark.activity} label={mark.said} says={mark.said} />
        <span className="armada-session-subagent__eyebrow">Subagent</span>
        {subagent.agent_type === undefined ? null : <code className="armada-session-subagent__type">{subagent.agent_type}</code>}
        <span className="armada-session-subagent__task">{subagent.task}</span>
        {ran === undefined ? null : <span className="armada-session-subagent__ran">{ran}</span>}
        {onOpen === undefined ? null : <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-subagent__go" />}
      </span>
      {subagent.latest === undefined ? null : <code className="armada-session-subagent__latest">{subagent.latest}</code>}
    </>
  );
  return (
    <li className="armada-session-subagent" data-state={subagent.state}>
      {onOpen === undefined ? (
        <div className="armada-session-subagent__press">{body}</div>
      ) : (
        <button type="button" className="armada-session-subagent__press" aria-label={`Subagent ${subagent.task}, ${mark.said.toLowerCase()}`} onClick={() => onOpen(subagent.id)}>
          {body}
        </button>
      )}
    </li>
  );
}
