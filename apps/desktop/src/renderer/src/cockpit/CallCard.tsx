// The call in front: what needs the owner, over the glass, answerable without the pointer. Above the
// question it says what the Job is and what he asked it for, where the question came from, and what
// is active now in one line; a Session's call is a small view of the Session instead. Its answers
// carry the numbers that pick them, and two standing ones carry `b` and `g`. Enter sends, `l` puts it
// off for later, `d` dismisses it for good, `o` opens what it is about, `e` opens the whole request. The keys themselves are
// `Cockpit`'s, on the window; this draws them and takes the press. Mock only, as the Dashboard is.

import { useLayoutEffect, useRef, useState, type MutableRefObject } from "react";
import { Bot, Box, ChevronDown, ChevronUp, Cpu, Scale, ShieldCheck, SquareTerminal, Waypoints, Workflow, type LucideIcon } from "lucide-react";
import { Button, Kbd, SessionThread, Tooltip, actionOf, keyFor, type NowPanelProps } from "@armada/components";

import type { BridgeState } from "../../../shared/bridge";
import { age, type Hosts, type Item } from "../Dashboard";
import { useSessions } from "../sessions-draft";
import { threadRowsOf } from "../sessions";
import { useAnswering, type Answer, type Answering } from "./answers";
import { sessionIdOf } from "./waiting";

/** What `Enter` does where a call has no answer of its own: it opens the thing. */
const OPENS = (item: Item) => (sessionIdOf(item.key) !== undefined ? "Open Session" : item.key.startsWith("pull:") ? "Open pull request" : "Open Job");

/** What runs, by its kind, and why nothing runs, by its: the marks the Now panel draws. */
const KIND: Record<"drone" | "check" | "judge", { Glyph: LucideIcon; said: string }> = {
  drone: { Glyph: Bot, said: "Drone" },
  check: { Glyph: ShieldCheck, said: "Check" },
  judge: { Glyph: Scale, said: "Judge" },
};
const WAIT: Record<"resource" | "job" | "transition" | "step", { Glyph: LucideIcon; said: string }> = {
  resource: { Glyph: Cpu, said: "Resource" },
  job: { Glyph: Box, said: "Job" },
  transition: { Glyph: Waypoints, said: "Transition" },
  step: { Glyph: Workflow, said: "Step" },
};

/** What the keys can say to the card beyond what an answer does. */
export type CardKeys = Answering & { expand?: () => void };

/** One answer: its number or its own key, the agent's line under it where it gave one, and what it tells the agent where it says. */
function AnswerTile({ one, mark, picked, onPick }: { one: Answer; mark: string; picked: boolean; onPick: () => void }) {
  const tile = (
    <button type="button" role="radio" aria-checked={picked} className="armada-callcard__answer" data-standing={one.key === undefined ? undefined : ""} onClick={onPick}>
      <Kbd>{mark}</Kbd>
      <span className="armada-callcard__option">
        <span className="armada-callcard__label">
          {one.label}
          {one.recommended === true ? <span className="armada-callcard__recommended">Recommended</span> : null}
        </span>
        {one.description === undefined ? null : <span className="armada-callcard__description">{one.description}</span>}
      </span>
    </button>
  );
  return one.says === undefined ? tile : <Tooltip label={one.says}>{tile}</Tooltip>;
}

/** What a reply in words has been given and not yet sent, kept by call so leaving the card keeps it. */
const DRAFTS = new Map<string, string>();

/** A reply in words: the Session's own "type something". Enter sends it. */
function Reply({ itemKey, send }: { itemKey: string; send: (text: string) => void }) {
  const [text, setText] = useState(DRAFTS.get(itemKey) ?? "");
  const change = (next: string) => (DRAFTS.set(itemKey, next), setText(next));
  return (
    <label className="armada-callcard__reply">
      <Kbd>{keyFor("call_reply")}</Kbd>
      <textarea
        aria-label="Type something"
        rows={1}
        value={text}
        onChange={(event) => change(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && text.trim() !== "") {
            event.preventDefault();
            event.nativeEvent.stopPropagation();
            DRAFTS.delete(itemKey);
            send(text.trim());
          } else if (event.key === "Escape") {
            event.currentTarget.blur();
          }
        }}
      />
    </label>
  );
}

/** Who a pull request can be attached to: every live Job and Session, a filter over them, j and k to move, Enter to pick. */
function AttachPicker({ attaching }: { attaching: NonNullable<Answering["attaching"]> }) {
  const [filter, setFilter] = useState("");
  const [at, setAt] = useState(0);
  const shown = attaching.candidates.filter((one) => one.title.toLowerCase().includes(filter.toLowerCase()));
  const here = Math.min(at, Math.max(0, shown.length - 1));
  const field = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => field.current?.focus(), []);
  const move = (by: number) => setAt(Math.min(shown.length - 1, Math.max(0, here + by)));
  return (
    <div className="armada-callcard__picker">
      <input
        ref={field}
        aria-label="Attach to"
        value={filter}
        onChange={(event) => (setFilter(event.target.value), setAt(0))}
        onKeyDown={(event) => {
          const empty = filter === "";
          if (event.key === "ArrowDown" || (empty && event.key === "j")) move(1);
          else if (event.key === "ArrowUp" || (empty && event.key === "k")) move(-1);
          else if (event.key === "Enter" && shown[here] !== undefined) attaching.choose(shown[here]!);
          else if (event.key === "Escape") attaching.close();
          else return;
          event.preventDefault();
          event.nativeEvent.stopPropagation();
        }}
      />
      <ul role="listbox" aria-label="Sessions" className="armada-callcard__candidates">
        {shown.map((one, index) => {
          const Glyph = one.kind === "session" ? SquareTerminal : Workflow;
          return (
            <li key={`${one.kind}:${one.id}`} role="option" aria-selected={index === here} className="armada-callcard__candidate" onClick={() => attaching.choose(one)}>
              <Tooltip label={one.kind === "session" ? "Session" : "Job"}>
                <span role="img" aria-label={one.kind === "session" ? "Session" : "Job"}>
                  <Glyph size={12} aria-hidden="true" />
                </span>
              </Tooltip>
              {one.title}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export function CallCard({
  item,
  now,
  nowing,
  state,
  hosts,
  finish,
  later,
  dismiss,
  leaving,
  from,
  answering,
}: {
  item: Item;
  now: number;
  /** What the Job runs now and waits on, where it is a Job: one line of it is drawn. */
  nowing: Omit<NowPanelProps, "onHide"> | undefined;
  state: BridgeState;
  hosts: Hosts;
  /** The call is answered: it is cleared from the queue. */
  finish: () => void;
  /** Put it off: it goes to the back of the stack. */
  later: () => void;
  /** Removes the call for good. */
  dismiss: () => void;
  leaving: "later" | "sent" | undefined;
  /** It was behind another card until now, and slides up from there. */
  from: "stack" | undefined;
  /** Where the card hands its answers up to, for the keys. */
  answering: MutableRefObject<CardKeys | undefined>;
}) {
  const answer = useAnswering(item, hosts, state, finish);
  const [whole, setWhole] = useState(false);
  const [long, setLong] = useState(false);
  const request = useRef<HTMLParagraphElement>(null);
  useLayoutEffect(() => {
    const one = request.current;
    if (one !== null && !whole) setLong(one.scrollHeight > one.clientHeight + 1);
  }, [item.request, whole]);
  answering.current = { ...answer, ...(long ? { expand: () => setWhole((was) => !was) } : {}) };

  const sessionId = sessionIdOf(item.key);
  const session = useSessions().find((one) => one.id === sessionId);
  const Icon = item.icon;
  const lone = answer.answers.length === 0;
  const numbered = answer.answers.filter((one) => one.key === undefined);
  const standing = answer.answers.filter((one) => one.key !== undefined);

  // What is active now, in one line: its kind's mark and its own words.
  const active = nowing?.running?.[0];
  const waiting = nowing?.waiting?.[0];
  const marked = active !== undefined ? KIND[active.of] : waiting !== undefined ? WAIT[waiting.kind] : undefined;
  const doing = active !== undefined ? (active.line ?? active.name) : waiting?.text;

  // Where the question came from: who asked, and at which step.
  const steps = state.holds.workflows.find((one) => one.id === item.job?.workflow_id)?.steps ?? [];
  const step = steps.find((one) => one.step_id === item.job?.current_step_id)?.label;
  const asker = item.body.find(([term]) => term === "From" || term === "On")?.[1] ?? active?.name;
  const origin = [asker, step].filter((one) => one !== undefined && one !== "").join(" · ");
  const OwnerGlyph = answer.owner?.kind === "session" ? SquareTerminal : Workflow;

  return (
    <section className="armada-callcard" data-hue={item.hue} data-leaving={leaving} data-from={from} aria-label={`${item.kind}: ${item.title}`}>
      <header className="armada-callcard__band">
        <Icon size={16} aria-hidden="true" />
        <span>{item.kind}</span>
        <span className="armada-callcard__where">{item.where}</span>
        <span className="armada-callcard__age">{age(item.at, now)}</span>
      </header>
      <div className="armada-callcard__body">
        <h2 className="armada-callcard__title">{item.title}</h2>
        {session !== undefined ? (
          <div className="armada-callcard__thread">
            <SessionThread sessionId={session.id} rows={threadRowsOf(session)} onAnswer={() => {}} onOpenSession={hosts.onOpenSession} />
          </div>
        ) : (
          <>
            {item.request === undefined ? null : (
              <div className="armada-callcard__asked">
                <p ref={request} className="armada-callcard__request" data-whole={whole || undefined}>
                  {item.request}
                </p>
                {!long && !whole ? null : (
                  <Button variant="ghost" size="sm" onClick={() => setWhole(!whole)}>
                    {whole ? <ChevronUp size={12} aria-hidden="true" /> : <ChevronDown size={12} aria-hidden="true" />}
                    {whole ? "Less" : "All of it"}
                    <Kbd>{keyFor("call_expand")}</Kbd>
                  </Button>
                )}
              </div>
            )}
            {answer.owner === undefined ? null : (
              <p className="armada-callcard__origin" data-owner role="group" aria-label="Owner">
                <Tooltip label={answer.owner.kind === "session" ? "Session" : "Job"}>
                  <span role="img" aria-label={answer.owner.kind === "session" ? "Session" : "Job"}>
                    <OwnerGlyph size={12} aria-hidden="true" />
                  </span>
                </Tooltip>
                {answer.owner.kind === "session" ? "Session" : "Job"} · {answer.owner.title}
              </p>
            )}
            {origin === "" ? null : <p className="armada-callcard__origin">{origin}</p>}
            {marked === undefined || doing === undefined ? null : (
              <p className="armada-callcard__doing">
                <Tooltip label={marked.said}>
                  <span role="img" aria-label={marked.said}>
                    <marked.Glyph size={12} aria-hidden="true" />
                  </span>
                </Tooltip>
                {doing}
              </p>
            )}
            {item.context === undefined || item.context.length === 0 ? null : (
              <section aria-label={item.contextHead ?? "Why it asks"}>
                <pre className="armada-callcard__why">{item.context.slice(0, 3).join("\n")}</pre>
              </section>
            )}
          </>
        )}
        <p className="armada-callcard__ask" data-quiet={(item.waiting !== undefined ? item.waiting.item.source !== "permission" : item.hue === "ask") || answer.decision !== undefined ? undefined : ""}>
          {answer.ask}
          {answer.decision === undefined ? null : (
            <span className="armada-callcard__pips" aria-hidden="true">
              {Array.from({ length: answer.decision.of }, (_, index) => (
                <span key={index} className="armada-callcard__pip" data-here={index === answer.decision!.at || undefined} />
              ))}
            </span>
          )}
        </p>
        {answer.attaching === undefined ? null : <AttachPicker attaching={answer.attaching} />}
        {lone || answer.attaching !== undefined ? null : (
          <div role="radiogroup" aria-label={answer.ask} className="armada-callcard__choices">
            <div className="armada-callcard__answers">
              {numbered.map((one, index) => (
                <AnswerTile key={one.id} one={one} mark={String(index + 1)} picked={answer.picked === answer.answers.indexOf(one)} onPick={() => answer.pick(answer.answers.indexOf(one))} />
              ))}
            </div>
            {standing.length === 0 ? null : (
              <div className="armada-callcard__answers" data-standing>
                {standing.map((one) => (
                  <AnswerTile key={one.id} one={one} mark={one.key!} picked={answer.picked === answer.answers.indexOf(one)} onPick={() => answer.pick(answer.answers.indexOf(one))} />
                ))}
              </div>
            )}
          </div>
        )}
        {answer.reply === undefined ? null : <Reply itemKey={item.key} send={answer.reply} />}
        <div className="armada-callcard__acts">
          <Button variant="primary" disabled={!answer.canSend} onClick={answer.send}>
            {lone ? OPENS(item) : "Send"}
            <Kbd>↵</Kbd>
          </Button>
          {lone || answer.open === undefined ? null : (
            <Button variant="ghost" onClick={answer.open}>
              {OPENS(item)}
              <Kbd>{keyFor("open")}</Kbd>
            </Button>
          )}
          <Button variant="ghost" onClick={later}>
            {actionOf("call_later").verb}
            <Kbd>{keyFor("call_later")}</Kbd>
          </Button>
          <Tooltip label="Never show this again">
            <Button variant="ghost" onClick={dismiss}>
              {actionOf("call_dismiss").verb}
              <Kbd>{keyFor("call_dismiss")}</Kbd>
            </Button>
          </Tooltip>
        </div>
      </div>
    </section>
  );
}
