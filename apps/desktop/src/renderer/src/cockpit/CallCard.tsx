// The call in front: what needs the owner, over the glass, answerable without the pointer. Above the
// question it says what the Job is and what he asked it for, where the question came from, and what
// is active now in one line; a Session's call is a small view of the Session instead. Its answers
// carry the numbers that pick them, and two standing ones carry `b` and `g`. Enter sends, `l` puts it
// off for later, `d` dismisses it for good, `o` opens what it is about, `v` opens the walk a Session waits on, `e` opens the whole request. The keys themselves are
// `Cockpit`'s, on the window; this draws them and takes the press. While an act waits on Fleet the
// control pressed carries the loop and the rest of the card is held. Mock only, as the Dashboard is.

import { useLayoutEffect, useRef, useState, type MutableRefObject } from "react";
import { Bot, Box, ChevronDown, ChevronUp, Cpu, Scale, ShieldCheck, SquareTerminal, Waypoints, Workflow, type LucideIcon } from "lucide-react";
import { Button, Kbd, SessionThread, Tooltip, actionOf, keyFor, type NowPanelProps } from "@armada/components";

import type { BridgeState } from "../../../shared/bridge";
import { age, type Hosts, type Item } from "../Dashboard";
import { useSessions } from "../sessions-draft";
import { SessionBox, threadRowsOf } from "../sessions";
import { useAnswering, type Answer, type Answering } from "./answers";
import { Failure, hasFailure } from "./Failure";
import { JobBox } from "./JobBox";
import { useJudgesRisk } from "./judges-risk";
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
export type CardKeys = Answering & { expand?: () => void; change?: () => void };

/** The one word a loop's tooltip says: the control is out with Fleet. */
const OUT = "Waiting on Fleet";

/**
 * One answer: its number or its own key, the agent's line under it where it gave one, and what it tells the
 * agent where it says. `pending` is the one waiting on Fleet, which keeps its look and sweeps the bar; `held`
 * is every other while one is, which cannot be pressed.
 */
function AnswerTile({ one, mark, picked, pending, held, onPick }: { one: Answer; mark: string; picked: boolean; pending: boolean; held: boolean; onPick: () => void }) {
  const tile = (
    <button
      type="button"
      role="radio"
      aria-checked={picked}
      className="armada-callcard__answer"
      data-standing={one.key === undefined ? undefined : ""}
      data-pending={pending || undefined}
      aria-busy={pending || undefined}
      disabled={held && !pending}
      onClick={onPick}
    >
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
  const said = pending ? OUT : one.says;
  return said === undefined ? tile : <Tooltip label={said}>{tile}</Tooltip>;
}

/** What a reply in words has been given and not yet sent, kept by call so leaving the card keeps it. */
const DRAFTS = new Map<string, string>();

/** A reply in words: the Session's own "type something". Enter sends it, as Send does. */
function Reply({ label, text, change, send, pending, held, focus, close }: { label: string; text: string; change: (next: string) => void; send: () => void; pending: boolean; held: boolean; focus?: boolean; close?: () => void }) {
  return (
    <label className="armada-callcard__reply" data-pending={pending || undefined}>
      <Kbd>{keyFor("call_reply")}</Kbd>
      <textarea
        aria-label={label}
        rows={1}
        value={text}
        readOnly={held}
        autoFocus={focus}
        aria-busy={pending || undefined}
        onChange={(event) => change(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && text.trim() !== "") {
            event.preventDefault();
            event.nativeEvent.stopPropagation();
            if (held) return;
            send();
          } else if (event.key === "Escape") {
            event.currentTarget.blur();
            close?.();
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
      <ul role="listbox" aria-label="Jobs and Sessions" className="armada-callcard__candidates">
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
  /** Clears the call for good, once it is dismissed: on Fleet by the card, or here where Fleet raised it. */
  dismiss: () => void;
  leaving: "later" | "sent" | undefined;
  /** It was behind another card until now, and slides up from there. */
  from: "stack" | undefined;
  /** Where the card hands its answers up to, for the keys. */
  answering: MutableRefObject<CardKeys | undefined>;
}) {
  const answer = useAnswering(item, hosts, state, finish, dismiss);
  const [text, setText] = useState(() => DRAFTS.get(item.key) ?? "");
  const change = (next: string) => (DRAFTS.set(item.key, next), setText(next));
  // A Sleep decision's text box is hidden until Change asks for it, and stays where words were left in it.
  const [changing, setChanging] = useState(() => text.trim() !== "");
  const focusReply = () => document.querySelector<HTMLElement>(".armada-callcard__reply textarea, .armada-callcard__composer [role=textbox]")?.focus();
  const [whole, setWhole] = useState(false);
  const [long, setLong] = useState(false);
  const request = useRef<HTMLParagraphElement>(null);
  useLayoutEffect(() => {
    const one = request.current;
    if (one !== null && !whole) setLong(one.scrollHeight > one.clientHeight + 1);
  }, [item.request, whole]);
  // A call that comes from a Session, and takes words, ends in that Session's own box rather than a bare one.
  const sessions = useSessions();
  const sleep = item.sleep;
  const home = sessions.find((one) => one.id === (sleep?.sessionId ?? sessionIdOf(item.key)));
  const composed = home !== undefined && home.dead === undefined && answer.reply !== undefined;
  // A Job's call that says what went wrong ends in a box whose words go to its Drone.
  const jobBoxed = item.issue !== undefined && answer.reply !== undefined;
  const boxed = composed || jobBoxed;
  // Words in the box are what he just typed, so they win over a picked answer: Send and Enter send them.
  const held = answer.pending !== undefined;
  const words = !boxed && answer.reply !== undefined && text.trim() !== "";
  // Kept until Fleet takes it: a refusal leaves what was typed where it was.
  const sendWords = () => !held && answer.reply?.(text.trim(), () => DRAFTS.delete(item.key));
  const canSend = words ? !held : answer.canSend;
  const send = words ? () => void sendWords() : answer.send;
  answering.current = {
    ...answer,
    canSend,
    send,
    ...(long ? { expand: () => setWhole((was) => !was) } : {}),
    ...(sleep === undefined ? {} : { change: () => (changing || composed ? focusReply() : setChanging(true)) }),
  };

  const sessionId = sessionIdOf(item.key);
  const session = sessions.find((one) => one.id === sessionId);
  // A Job's call that asks a person, where a risky refusal is what brought it: the Judges' reason.
  const risk = useJudgesRisk(item.job, item.hue === "ask" && item.decisions === undefined);
  const Icon = item.icon;
  const lone = answer.answers.length === 0;
  // An act is a button that does it; an option is a radio, picked and then sent, because a reply can go with it.
  const acts = answer.answers.filter((one) => one.once === true);
  const numbered = answer.answers.filter((one) => one.key === undefined && one.once !== true);
  const standing = answer.answers.filter((one) => one.key !== undefined);
  const actsOnly = acts.length > 0 && numbered.length === 0 && standing.length === 0;

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

  // The box's Send: the pick goes with the words where one is picked, else the words answer the call.
  const sendComposed = (words: string) => {
    if (held) return;
    const picked = answer.picked === undefined ? undefined : answer.answers[answer.picked];
    if (picked !== undefined && picked.once !== true) picked.run(words.trim() === "" ? undefined : words.trim());
    else if (words.trim() !== "") answer.reply?.(words.trim());
  };

  // A card made only of acts is split: the decision on the left, a quiet group of ghosts on the right.
  // The first act is the primary and Enter runs it, until there are words in the box, which Enter sends.
  const decide = (
    <>
      {answer.attaching !== undefined
        ? null
        : acts.map((one) => {
            const at = answer.answers.indexOf(one);
            const first = actsOnly && answer.picked === at && !words;
            const button = (
              <Button key={one.id} variant={first ? "primary" : "secondary"} pending={answer.pending === one.id} disabled={held && answer.pending !== one.id} onClick={() => one.run()}>
                {one.label}
                <Kbd>{first ? "↵" : String(at + 1)}</Kbd>
              </Button>
            );
            return one.says === undefined ? button : <Tooltip key={one.id} label={one.says}>{button}</Tooltip>;
          })}
      {sleep === undefined ? null : (
        <Button variant="secondary" disabled={held} onClick={() => (changing || composed ? focusReply() : setChanging(true))}>
          {sleep.corrected === undefined ? "Change" : "Change again"}
          <Kbd>{keyFor("call_reply")}</Kbd>
        </Button>
      )}
      {actsOnly && !words ? null : (
        <Button variant="primary" disabled={!canSend} onClick={send}>
          {lone && !words ? OPENS(item) : "Send"}
          <Kbd>↵</Kbd>
        </Button>
      )}
    </>
  );
  const quiet = (
    <>
      {answer.openWalk === undefined ? null : (
        <Button variant="ghost" disabled={held} onClick={answer.openWalk}>
          {actionOf("call_open_walk").verb}
          <Kbd>{keyFor("call_open_walk")}</Kbd>
        </Button>
      )}
      {lone || answer.open === undefined ? null : (
        <Button variant="ghost" disabled={held} onClick={answer.open}>
          {sleep === undefined ? OPENS(item) : sleep.sessionId !== undefined ? "Open the Session" : "Open the Job"}
          <Kbd>{keyFor("open")}</Kbd>
        </Button>
      )}
      <Button variant="ghost" disabled={held} onClick={later}>
        {actionOf("call_later").verb}
        <Kbd>{keyFor("call_later")}</Kbd>
      </Button>
      <Tooltip label={answer.pending === "dismiss" ? OUT : "Never show this again"}>
        <Button variant="ghost" pending={answer.pending === "dismiss"} disabled={held} onClick={answer.dismiss}>
          {actionOf("call_dismiss").verb}
          <Kbd>{keyFor("call_dismiss")}</Kbd>
        </Button>
      </Tooltip>
    </>
  );

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
            {risk === undefined ? null : (
              <section aria-label="Why the Judges brought it to you">
                <p className="armada-callcard__why">{risk}</p>
              </section>
            )}
            {item.issue !== undefined && hasFailure(item.issue) ? <Failure issue={item.issue} /> : null}
            {sleep !== undefined || (item.issue !== undefined && hasFailure(item.issue)) || item.context === undefined || item.context.length === 0 ? null : (
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
        {sleep === undefined || item.context === undefined ? null : (
          <section aria-label={item.contextHead ?? "Chose"}>
            <pre className="armada-callcard__why">{item.context.slice(0, 1).join("\n")}</pre>
          </section>
        )}
        {answer.attaching === undefined ? null : <AttachPicker attaching={answer.attaching} />}
        {numbered.length === 0 && standing.length === 0 || answer.attaching !== undefined ? null : (
          <div role="radiogroup" aria-label={answer.ask} className="armada-callcard__choices">
            <div className="armada-callcard__answers">
              {numbered.map((one, index) => (
                <AnswerTile key={one.id} one={one} mark={String(index + 1)} picked={answer.picked === answer.answers.indexOf(one)} pending={answer.pending === one.id} held={held} onPick={() => answer.pick(answer.answers.indexOf(one))} />
              ))}
            </div>
            {standing.length === 0 ? null : (
              <div className="armada-callcard__answers" data-standing>
                {standing.map((one) => (
                  <AnswerTile key={one.id} one={one} mark={one.key!} picked={answer.picked === answer.answers.indexOf(one)} pending={answer.pending === one.id} held={held} onPick={() => answer.pick(answer.answers.indexOf(one))} />
                ))}
              </div>
            )}
          </div>
        )}
        {boxed || answer.reply === undefined || (sleep !== undefined && !changing) ? null : (
          <Reply label={sleep === undefined ? "Type something" : "Change"} text={text} change={change} send={sendWords} pending={answer.pending === "reply"} held={held} {...(sleep === undefined ? {} : { focus: true, close: () => setChanging(false) })} />
        )}
        <div className="armada-callcard__acts" data-split={actsOnly || undefined}>
          {actsOnly ? <div className="armada-callcard__decide">{decide}</div> : decide}
          {actsOnly ? <div className="armada-callcard__quiet">{quiet}</div> : quiet}
        </div>
        {!jobBoxed ? null : (
          <div className="armada-callcard__composer">
            <JobBox draftKey={`job-${item.key}`} onSend={sendComposed} />
          </div>
        )}
        {!composed || home === undefined ? null : (
          <div className="armada-callcard__composer">
            <SessionBox session={home} onOpen={hosts.onOpenSession} onSend={sendComposed} />
          </div>
        )}
      </div>
    </section>
  );
}
