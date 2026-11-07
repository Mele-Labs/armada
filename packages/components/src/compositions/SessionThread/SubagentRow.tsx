import { Bot, ChevronRight } from "lucide-react";
import type { ReactNode } from "react";
import type { ToolSubagent } from "@armada/protocol";

import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";
import { elapsedWords } from "../SessionRetroRow/SessionRetroRow";

const MARK: Record<
  ToolSubagent["state"],
  {
    activity: StepActivity;
    said: string;
  }
> = {
  running: {
    activity: "running",
    said: "Running",
  },
  finished: {
    activity: "advanced",
    said: "Finished",
  },
  cut_off: {
    activity: "killed",
    said: "Cut off",
  },
};

/** What its status mark says on hover: the state, and the kind of subagent where the call named one. The mark is what says it is a subagent. */
export function saidOf(subagent: ToolSubagent): string {
  const said = `Subagent ${MARK[subagent.state].said.toLowerCase()}`;
  return subagent.agent_type === undefined ? said : `${said} · ${subagent.agent_type}`;
}

/** How long it ran: to now while it runs, to `ended` once settled, and nothing where either end will not parse. */
function ranFor(subagent: ToolSubagent, now: number): string | undefined {
  const from = Date.parse(subagent.since);
  const to =
    subagent.state === "running"
      ? now
      : subagent.ended === undefined
        ? Number.NaN
        : Date.parse(subagent.ended);
  return Number.isNaN(from) || Number.isNaN(to) ? undefined : elapsedWords(to - from);
}

/**
 * The subagent an `Agent` call started, in place of its one tool line: while it runs, the handoff, its live view being the
 * pinned `SubagentsCard`; once it returns, how it ended and how long it took. **Its calls are never rows of the thread**,
 * which would put its words in the agent's mouth. A press opens its own thread.
 */
export function SubagentRow({
  subagent,
  onOpen,
}: {
  subagent: ToolSubagent;
  onOpen?: (id: string) => void;
}) {
  const running = subagent.state === "running";
  const mark = MARK[subagent.state];
  const press = (body: ReactNode) =>
    onOpen === undefined ? (
      <div className="armada-session-subagent__press">{body}</div>
    ) : (
      <button
        type="button"
        className="armada-session-subagent__press"
        aria-label={`Subagent ${subagent.task}, ${mark.said.toLowerCase()}`}
        onClick={() => onOpen(subagent.id)}
      >
        {body}
      </button>
    );
  // While it runs, the pinned card is its live view: here it is the handoff, one quiet line as a call is.
  if (running)
    return (
      <li className="armada-session-subagent" data-state={subagent.state} data-handoff="">
        {press(
          <>
            <Bot size={14} strokeWidth={2} aria-hidden className="armada-session-subagent__bot" />
            <span className="armada-session-subagent__handoff">
              Started a subagent: {subagent.task}
            </span>
          </>,
        )}
      </li>
    );
  // Once it returns, its result: how it ended and how long it took. A press opens its thread.
  const ran = ranFor(subagent, Date.now());
  return (
    <li className="armada-session-subagent" data-state={subagent.state}>
      {press(
        <span className="armada-session-subagent__head">
          <StepActivityMark activity={mark.activity} label={mark.said} says={saidOf(subagent)} />
          <span className="armada-session-subagent__task">{subagent.task}</span>
          {ran === undefined ? null : <span className="armada-session-subagent__ran">{ran}</span>}
          {onOpen === undefined ? null : (
            <ChevronRight
              size={12}
              strokeWidth={2}
              aria-hidden
              className="armada-session-subagent__go"
            />
          )}
        </span>,
      )}
    </li>
  );
}
