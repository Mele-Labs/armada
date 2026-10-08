import { useLayoutEffect, useRef, useState } from "react";
import { AppWindow, Box, ChevronRight, GitBranch, KeyRound, Layers, SquareTerminal, Wrench } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Prose } from "../../primitives/Prose/Prose";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Session's conversation, drawn on Helm's own thread rows
 * (`HelmThread.css`) with the two things Helm's thread has no room for.
 *
 * **A message another Session wrote is set apart**: its own band, with the
 * Session mark and the sender's title, over the words, and the band is a press
 * that opens that Session. It is never folded into the agent's voice, which
 * the owner found too easy to mistake it for (6 Oct 2026).
 *
 * **The first write is a row of its own**, filled in the running hue, because
 * it is the one moment a Session stops being blank: the slot leased and the
 * branch cut, in the thread where it happened.
 */
export type SessionThreadRow =
  | {
      id: string;
      at: string;
      kind: "message";
      from: "you" | "agent";
      text: string;
      /** What was sent with it. Pictures are drawn; other files are chips. */
      files?: readonly { id: string; name: string; src?: string }[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly { kind: "session" | "job" | "pull_request" | "branch"; id: string; title: string }[];
    }
  | { id: string; at: string; kind: "message"; from: "session"; sender: { id: string; title: string }; text: string }
  /** A tool call, mono. */
  | { id: string; at: string; kind: "tool"; text: string }
  /** A command the person ran in the terminal, as typed, mono. */
  | { id: string; at: string; kind: "command"; text: string }
  /** The summary the CLI wrote where it compacted the conversation. Not the person's words: a quiet row that opens to its text. */
  | { id: string; at: string; kind: "compaction"; text: string }
  | { id: string; at: string; kind: "lease"; slot: number; branch: string }
  /** The Session showed a page in a window: a quiet row, a press that opens the window again. */
  | { id: string; at: string; kind: "window"; title: string; url: string }
  | {
      id: string;
      at: string;
      kind: "handoff";
      job: { number: number; title: string };
      slot: number;
      branch: string;
      /** Absent where the Job stopped on no step. */
      step?: { id: string; label: string };
      attempts: number;
      refusals: readonly string[];
      /** Files written that the plan did not cover, and paths it named that nothing changed under. */
      plan: { outside: readonly string[]; unwritten: readonly string[] };
      /** Absent where no Drone said what it was stuck on: a Job taken over from an escalation or a gate has none. */
      narrative?: { trying_to: string; blocked_by: string; tried: readonly string[] };
    };

/** One answer an ask will take: what the press hands back, the word on it and what it commits to. */
export type AskOffer = { id: string; label: string; means: string };

/** One question the agent put to the person, with the options it offered. */
export type AskedQuestion = {
  question: string;
  header: string;
  multi_select: boolean;
  options: readonly { label: string; description: string }[];
};

/** What was chosen for one question: option labels, and the person's own words where they chose Other. */
export type QuestionAnswer = { question: string; chosen: string[] };

export type SessionThreadProps = {
  rows: readonly SessionThreadRow[];
  /**
   * The permission the agent is held on. **`offers` are the answers Fleet will take**, drawn in its
   * order; absent, the card offers Allow once and Deny and names neither.
   */
  asked?: { command: string; offers?: readonly AskOffer[]; questions?: readonly AskedQuestion[] };
  onAnswer: (answer?: string, answers?: QuestionAnswer[]) => void;
  /** Opens the Session a message came from. */
  onOpenSession: (sessionId: string) => void;
  /** Opens a window the Session showed, again. Absent where windows are not served, and its row is not a press. */
  onOpenWindow?: (url: string) => void;
  /** Which Session this is. A change of it opens the thread at its newest row again. */
  sessionId?: string;
};

const TAG_KIND = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch" } as const;

function Sent({ row }: { row: Extract<SessionThreadRow, { from: "you" | "agent" }> }) {
  const files = row.files ?? [];
  const sketches = row.sketches ?? [];
  const tags = row.tags ?? [];
  if (files.length + sketches.length + tags.length === 0) return null;
  return (
    <div className="armada-session-sent">
      {files.map((file) =>
        file.src === undefined ? (
          <AttachmentChip key={file.id} filename={file.name} />
        ) : (
          <img key={file.id} className="armada-session-sent__picture" src={file.src} alt={file.name} />
        ),
      )}
      {sketches.map((one) => (
        <AttachmentChip key={one.id} filename={one.title} from="Sketch" />
      ))}
      {tags.map((one) => (
        <AttachmentChip key={`${one.kind}${one.id}`} filename={one.title} from={TAG_KIND[one.kind]} />
      ))}
    </div>
  );
}

/** What Fleet knew when a Job's Drone stopped, as the structured fields the handoff bundle has. A field with nothing to say is left out. */
function Handoff({ row }: { row: Extract<SessionThreadRow, { kind: "handoff" }> }) {
  const { step, narrative } = row;
  const planned = row.plan.outside.length + row.plan.unwritten.length > 0;
  return (
    <li className="armada-session-handoff" role="region" aria-label={`Handed over: Job ${row.job.number}`}>
      <div className="armada-session-handoff__head">
        <span className="armada-session-lease__eyebrow">Handed over</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Job ${row.job.number}`}>
            <Box size={12} strokeWidth={2} aria-hidden />
            {row.job.number}
          </span>
          <span className="armada-session-lease__chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
            <KeyRound size={12} strokeWidth={2} aria-hidden />
            {row.slot}
          </span>
          <span className="armada-session-lease__chip" role="img" aria-label={`Branch ${row.branch}`}>
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            {row.branch}
          </span>
        </span>
        <span className="armada-session-lease__at">{row.at}</span>
      </div>
      <dl className="armada-session-handoff__fields">
        {step === undefined ? null : (
          <>
            <dt>Stopped on</dt>
            <dd>
              <code>{step.id}</code> {step.label}
            </dd>
            <dt>Attempts</dt>
            <dd>
              <code>{row.attempts}</code>
            </dd>
          </>
        )}
        {row.refusals.length === 0 ? null : (
          <>
            <dt>Judge refused</dt>
            <dd>
              <ul>
                {row.refusals.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </dd>
          </>
        )}
        {!planned ? null : (
          <>
            <dt>Plan against diff</dt>
            <dd>
              <ul>
                {row.plan.outside.map((one) => (
                  <li key={one}>
                    <code>{one}</code> written, not declared
                  </li>
                ))}
                {row.plan.unwritten.map((one) => (
                  <li key={one}>
                    <code>{one}</code> declared, not written
                  </li>
                ))}
              </ul>
            </dd>
          </>
        )}
        {narrative === undefined ? null : (
          <>
            <dt>Trying to</dt>
            <dd>{narrative.trying_to}</dd>
            <dt>Blocked by</dt>
            <dd>{narrative.blocked_by}</dd>
            <dt>Tried</dt>
            <dd>
              <ol>
                {narrative.tried.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ol>
            </dd>
          </>
        )}
      </dl>
    </li>
  );
}

/** What the thread draws: a row, or the tool calls that ran one after another, folded into one. */
type Item = { kind: "row"; row: Exclude<SessionThreadRow, { kind: "tool" }> } | { kind: "calls"; id: string; rows: readonly Extract<SessionThreadRow, { kind: "tool" }>[] };

function itemsOf(rows: readonly SessionThreadRow[]): Item[] {
  const items: Item[] = [];
  for (const row of rows) {
    const last = items[items.length - 1];
    if (row.kind !== "tool") items.push({ kind: "row", row });
    else if (last?.kind === "calls") last.rows = [...last.rows, row];
    else items.push({ kind: "calls", id: row.id, rows: [row] });
  }
  return items;
}

/** The tools a group holds, each named once, in the order they were first called. */
function namesOf(rows: readonly { text: string }[]): string[] {
  return [...new Set(rows.map((one) => one.text.split(/\s/, 1)[0] ?? ""))].filter((name) => name !== "");
}

/**
 * **Calls that ran one after another are one row, closed.** It names the tools it holds and draws no
 * count; pressing it shows each call. A lone call is a group of one.
 */
function Calls({ rows }: { rows: readonly { id: string; text: string }[] }) {
  const names = namesOf(rows);
  return (
    <li className="armada-session-fold armada-session-fold--calls">
      <details>
        <summary className="armada-session-fold__head" aria-label={`Tool calls: ${names.join(", ")}`}>
          <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
          <Wrench size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-fold__names">{names.join(", ")}</span>
        </summary>
        <ol className="armada-session-fold__calls">
          {rows.map((one) => (
            <li key={one.id} className="armada-session-command">
              <code className="armada-session-command__text">{one.text}</code>
            </li>
          ))}
        </ol>
      </details>
    </li>
  );
}

function Row({
  row,
  onOpenSession,
  onOpenWindow,
}: {
  row: Exclude<SessionThreadRow, { kind: "tool" }>;
  onOpenSession: (id: string) => void;
  onOpenWindow?: (url: string) => void;
}) {
  if (row.kind === "window") {
    return (
      <li className="armada-session-fold">
        <button type="button" className="armada-session-fold__head armada-session-fold__head--press" aria-label={`Open window ${row.title}`} onClick={() => onOpenWindow?.(row.url)}>
          <AppWindow size={12} strokeWidth={2} aria-hidden />
          {row.title}
        </button>
      </li>
    );
  }
  if (row.kind === "handoff") return <Handoff row={row} />;
  if (row.kind === "lease") {
    return (
      <li className="armada-session-lease" role="region" aria-label="Leased on first write">
        <span className="armada-session-lease__eyebrow">First write</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
            <KeyRound size={12} strokeWidth={2} aria-hidden />
            {row.slot}
          </span>
          <span className="armada-session-lease__chip" role="img" aria-label={`Branch ${row.branch}`}>
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            {row.branch}
          </span>
        </span>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "command") {
    return (
      <li className="armada-session-command">
        <code className="armada-session-command__text">{row.text}</code>
      </li>
    );
  }
  if (row.kind === "compaction") {
    return (
      <li className="armada-session-fold">
        <details>
          <summary className="armada-session-fold__head">
            <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
            <Layers size={12} strokeWidth={2} aria-hidden />
            Conversation compacted
          </summary>
          <div className="armada-session-fold__body">
            <Prose text={row.text} />
          </div>
        </details>
      </li>
    );
  }
  if (row.from === "session") {
    const { sender } = row;
    return (
      <li className="armada-session-from" role="region" aria-label={`Message from ${sender.title}`}>
        <button type="button" className="armada-session-from__band" aria-label={`Open Session ${sender.title}`} onClick={() => onOpenSession(sender.id)}>
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-from__id">{sender.id}</span>
          <span className="armada-session-from__title">{sender.title}</span>
          <span className="armada-session-from__at">{row.at}</span>
        </button>
        <div className="armada-session-from__body">
          <Prose text={row.text} />
        </div>
      </li>
    );
  }
  return (
    <li className="armada-helm-thread__row" data-actor={row.from === "you" ? "you" : "helm"} data-from={row.from}>
      <div className="armada-helm-thread__head">
        <span className="armada-helm-thread__who">{row.from === "you" ? "You" : "Agent"}</span>
        <span className="armada-helm-thread__at">{row.at}</span>
      </div>
      <div className="armada-helm-thread__message">
        <Prose text={row.text} />
      </div>
      <Sent row={row} />
    </li>
  );
}

/**
 * The agent's questions as a form. **Each question takes an option or, last, the person's own words**
 * (*Other*); a multi-select takes any number. Answering is held until every question has something
 * in it, which is what Fleet checks too.
 */
function Questions({
  questions,
  onAnswer,
  canSkip,
}: {
  questions: readonly AskedQuestion[];
  onAnswer: (answer?: string, answers?: QuestionAnswer[]) => void;
  canSkip: boolean;
}) {
  const [picked, setPicked] = useState<readonly (readonly string[])[]>(() => questions.map(() => []));
  const [using, setUsing] = useState<readonly boolean[]>(() => questions.map(() => false));
  const [words, setWords] = useState<readonly string[]>(() => questions.map(() => ""));
  const at = <T,>(list: readonly T[], index: number, value: T): T[] => list.map((one, i) => (i === index ? value : one));
  const chosenFor = (index: number): string[] => {
    const typed = words[index]?.trim() ?? "";
    return [...(picked[index] ?? []), ...(using[index] === true && typed !== "" ? [typed] : [])];
  };
  const complete = questions.every((_, index) => chosenFor(index).length > 0);
  return (
    <div className="armada-session-questions">
      {questions.map((one, index) => (
        <fieldset key={one.question} className="armada-session-questions__one">
          <legend className="armada-session-questions__text">
            {one.header === "" ? null : <span className="armada-session-questions__header">{one.header}</span>}
            {one.question}
          </legend>
          <RadioGroup>
            {one.options.map((option) => {
              const on = (picked[index] ?? []).includes(option.label);
              const choose = () =>
                setPicked(
                  at(
                    picked,
                    index,
                    one.multi_select ? (on ? (picked[index] ?? []).filter((x) => x !== option.label) : [...(picked[index] ?? []), option.label]) : [option.label],
                  ),
                );
              const control = one.multi_select ? (
                <Checkbox checked={on} onChange={choose}>
                  {option.label}
                </Checkbox>
              ) : (
                <Radio
                  name={`question-${index}`}
                  checked={on}
                  onChange={() => {
                    choose();
                    setUsing(at(using, index, false));
                  }}
                >
                  {option.label}
                </Radio>
              );
              return option.description === "" ? (
                <div key={option.label}>{control}</div>
              ) : (
                <Tooltip key={option.label} label={option.description}>
                  <div>{control}</div>
                </Tooltip>
              );
            })}
            {one.multi_select ? (
              <Checkbox checked={using[index] === true} onChange={() => setUsing(at(using, index, using[index] !== true))}>
                Other
              </Checkbox>
            ) : (
              <Radio
                name={`question-${index}`}
                checked={using[index] === true}
                onChange={() => {
                  setUsing(at(using, index, true));
                  setPicked(at(picked, index, []));
                }}
              >
                Other
              </Radio>
            )}
          </RadioGroup>
          {using[index] === true ? (
            <Input
              aria-label="Other"
              value={words[index] ?? ""}
              onChange={(event) => setWords(at(words, index, event.target.value))}
            />
          ) : null}
        </fieldset>
      ))}
      <div className="armada-session-thread__answers" role="group" aria-label="Answers">
        <Button
          size="sm"
          variant="secondary"
          disabled={!complete}
          onClick={() =>
            onAnswer(
              "allow_once",
              questions.map((one, index) => ({ question: one.question, chosen: chosenFor(index) })),
            )
          }
        >
          Answer
        </Button>
        {canSkip ? (
          <Button size="sm" variant="ghost" onClick={() => onAnswer("refuse")}>
            Skip
          </Button>
        ) : null}
      </div>
    </div>
  );
}

/** How far from the end still counts as being at it, so a rounding or a half row does not unpin. */
const NEAR_END = 24;

export function SessionThread({ rows, asked, onAnswer, onOpenSession, onOpenWindow, sessionId }: SessionThreadProps) {
  const scroller = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);
  // **Opens at the newest row, with no animation, and stays there while the person is at the end.**
  // A person who scrolled up to read is not moved by a row that arrives.
  useLayoutEffect(() => {
    pinned.current = true;
  }, [sessionId]);
  useLayoutEffect(() => {
    const one = scroller.current;
    if (one !== null && pinned.current) one.scrollTop = one.scrollHeight;
  }, [rows, sessionId]);
  return (
    <div className="armada-session-thread">
      <div
        ref={scroller}
        className="armada-session-thread__rows"
        role="region"
        aria-label="Thread"
        onScroll={(event) => {
          const one = event.currentTarget;
          pinned.current = one.scrollHeight - one.scrollTop - one.clientHeight <= NEAR_END;
        }}
      >
        <ol className="armada-helm-thread__rows">
          {itemsOf(rows).map((item) =>
            item.kind === "calls" ? (
              <Calls key={item.id} rows={item.rows} />
            ) : (
              <Row key={item.row.id} row={item.row} onOpenSession={onOpenSession} onOpenWindow={onOpenWindow} />
            ),
          )}
        </ol>
      </div>
      {asked === undefined ? null : (
        <Card flat className="armada-session-thread__ask" role="article" aria-label="Waiting on you">
          <span className="armada-session-thread__eyebrow">{(asked.questions?.length ?? 0) > 0 ? "Question" : "Permission"}</span>
          {(asked.questions?.length ?? 0) > 0 ? null : <p className="armada-session-thread__command">{asked.command}</p>}
          {asked.questions !== undefined && asked.questions.length > 0 ? (
            <Questions
              key={asked.command}
              questions={asked.questions}
              onAnswer={onAnswer}
              canSkip={asked.offers === undefined || asked.offers.some((offer) => offer.id === "refuse")}
            />
          ) : (
          <div className="armada-session-thread__answers" role="group" aria-label="Answers">
            {asked.offers === undefined ? (
              <>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Allow once
                </Button>
                <Button size="sm" variant="secondary" onClick={() => onAnswer()}>
                  Deny
                </Button>
              </>
            ) : (
              asked.offers.map((offer) => (
                <Tooltip key={offer.id} label={offer.means}>
                  <Button size="sm" variant="secondary" onClick={() => onAnswer(offer.id)}>
                    {offer.label}
                  </Button>
                </Tooltip>
              ))
            )}
          </div>
          )}
        </Card>
      )}
    </div>
  );
}
