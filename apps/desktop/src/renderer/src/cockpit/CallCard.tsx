// The call in front: what needs the owner, over the glass, answerable without the pointer. A Session's
// call is a small view of the Session. Its answers
// carry the numbers that pick them, and two standing ones carry `b` and `g`. Enter sends, `l` puts it
// off for later, `d` dismisses it for good, `o` opens what it is about, `v` opens the walk a Session waits on. The keys themselves are
// `Cockpit`'s, on the window; this draws them and takes the press. While an act waits on Fleet the
// control pressed carries the loop and the rest of the card is held. Mock only, as the Dashboard is.

import { useEffect, useLayoutEffect, useRef, useState, type MutableRefObject, type ReactNode } from "react";
import { SquareTerminal } from "lucide-react";
import { AskAnswer, AskAnswers, AskCard, AskChoices, AskQuestion, Button, Kbd, RemoveAct, RemoveBox, RunnableBlock, SessionThread, Tooltip, actionOf, keyFor } from "@armada/components";

import type { BridgeState } from "../../../shared/bridge";
import { type Hosts, type Item } from "../Dashboard";
import { useSessions } from "../sessions-draft";
import { SessionBox } from "../sessions";
import { threadRowsOf } from "../session-thread-rows";
import { useAnswering, type Answer, type Answering } from "./answers";
import { CoveredKeycap, Keycap, useCover } from "./consoles";
import { sessionIdOf } from "./waiting";

/** What `Enter` does where a call has no answer of its own: it opens the thing. */
const OPENS = (item: Item) => (sessionIdOf(item.key) !== undefined ? "Open Job" : item.key.startsWith("pull:") ? "Open pull request" : "Open");

/** What the keys can say to the card beyond what an answer does. */
export type CardKeys = Answering & { change?: () => void };

/** The one word a loop's tooltip says: the control is out with Fleet. */
const OUT = "Waiting on Fleet";

/**
 * One answer: its number or its own key, the agent's line under it where it gave one, and what it tells the
 * agent where it says. `pending` is the one waiting on Fleet, which keeps its look and sweeps the bar; `held`
 * is every other while one is, which cannot be pressed.
 */
function AnswerTile({ one, mark, picked, pending, held, onPick }: { one: Answer; mark: string; picked: boolean; pending: boolean; held: boolean; onPick: () => void }) {
  return (
    <AskAnswer
      one={{ label: one.label, ...(one.description === undefined ? {} : { description: one.description }), ...(one.recommended === true ? { recommended: true } : {}), ...(one.says === undefined ? {} : { says: one.says }), standing: one.key !== undefined }}
      mark={mark}
      picked={picked}
      pending={pending}
      held={held}
      pendingSays={OUT}
      onPick={onPick}
    />
  );
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

/** Who a pull request can be attached to: every live Session, a filter over them, j and k to move, Enter to pick. */
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
      <ul role="listbox" aria-label="Jobs" className="armada-callcard__candidates">
        {shown.map((one, index) => {
          return (
            <li key={`${one.kind}:${one.id}`} role="option" aria-selected={index === here} className="armada-callcard__candidate" onClick={() => attaching.choose(one)}>
              <Tooltip label="Job">
                <span role="img" aria-label="Job">
                  <SquareTerminal size={12} aria-hidden="true" />
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
  state,
  hosts,
  finish,
  later,
  dismiss,
  leaving,
  from,
  answering,
  queue,
}: {
  item: Item;
  now: number;
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
  /** The calls behind this one, drawn as a strip in the console on its left where the acts are in consoles. */
  queue?: ReactNode;
}) {
  const answer = useAnswering(item, hosts, state, finish, dismiss);
  const [text, setText] = useState(() => DRAFTS.get(item.key) ?? "");
  const change = (next: string) => (DRAFTS.set(item.key, next), setText(next));
  // A Sleep decision's text box is hidden until Change asks for it, and stays where words were left in it.
  const [changing, setChanging] = useState(() => text.trim() !== "");
  const focusReply = () => document.querySelector<HTMLElement>(".armada-callcard__reply textarea, .armada-callcard__composer [role=textbox]")?.focus();
  // A call that comes from a Session, and takes words, ends in that Session's own box rather than a bare one.
  const sessions = useSessions();
  const sleep = item.sleep;
  const home = sessions.find((one) => one.id === (sleep?.sessionId ?? sessionIdOf(item.key)));
  const composed = home !== undefined && home.dead === undefined && answer.reply !== undefined;
  const boxed = composed;
  // The acts are on the rail beside the card; the row under it is for the glass too narrow to hold the rail.
  // A Sleep decision's box is hidden until the owner asks for it; a Session's is always there.
  const openChange = () => (changing || composed ? focusReply() : setChanging(true));
  // Words in the box are what he just typed, so they win over a picked answer: Send and Enter send them.
  const held = answer.pending !== undefined;
  const words = !boxed && answer.reply !== undefined && text.trim() !== "";
  // Kept until Fleet takes it: a refusal leaves what was typed where it was.
  const sendWords = () => !held && answer.reply?.(text.trim(), () => DRAFTS.delete(item.key));
  const canSend = words ? !held : answer.canSend;
  const send = words ? () => void sendWords() : answer.send;
  // An item the agent stated is removed with a reason box first, and the agent is told; any other is dismissed outright.
  const [removing, setRemoving] = useState(false);
  useEffect(() => setRemoving(false), [item.key]);
  const removable = answer.remove !== undefined;
  const cover = useCover(() => answer.dismiss());
  answering.current = {
    ...answer,
    canSend,
    send,
    // t reaches the Session's own box too, which is always there.
    ...(sleep === undefined && !composed ? {} : { change: openChange }),
    // A key cannot dismiss for good by itself: the cover lifts first. Removing an item the agent stated opens its reason box instead.
    dismiss: removable ? () => setRemoving(true) : cover.press,
  };

  const sessionId = sessionIdOf(item.key);
  const session = sessions.find((one) => one.id === sessionId);
  // A pull request or red main a Session holds draws that Session's thread under the facts, so he sees what it is doing.
  const holder = item.ownedBy?.kind === "session" ? sessions.find((one) => one.id === item.ownedBy!.id) : undefined;
  const Icon = item.icon;
  const lone = answer.answers.length === 0;
  // An act is a button that does it; an option is a radio, picked and then sent, because a reply can go with it.
  const acts = answer.answers.filter((one) => one.once === true);
  const numbered = answer.answers.filter((one) => one.key === undefined && one.once !== true);
  const standing = answer.answers.filter((one) => one.key !== undefined);
  const actsOnly = acts.length > 0 && numbered.length === 0 && standing.length === 0;

  // Where the question came from: who asked.
  const origin = item.body.find(([term]) => term === "From" || term === "On")?.[1] ?? "";

  // The box's Send: the pick goes with the words where one is picked, else the words answer the call.
  const sendComposed = (words: string) => {
    if (held) return;
    const picked = answer.picked === undefined ? undefined : answer.answers[answer.picked];
    if (picked !== undefined && picked.once !== true) picked.run(words.trim() === "" ? undefined : words.trim());
    else if (words.trim() !== "") answer.reply?.(words.trim());
  };

  const changeLabel = sleep === undefined ? "Message" : sleep.corrected === undefined ? "Change" : "Change again";
  const openLabel = sleep === undefined ? OPENS(item) : "Open the Job";

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
        <Button variant="secondary" disabled={held} onClick={openChange}>
          {changeLabel}
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
          {openLabel}
          <Kbd>{keyFor("open")}</Kbd>
        </Button>
      )}
      <Button variant="ghost" disabled={held} onClick={later}>
        {actionOf("call_later").verb}
        <Kbd>{keyFor("call_later")}</Kbd>
      </Button>
      {removable ? null : (
        <Tooltip label={answer.pending === "dismiss" ? OUT : "Never show this again"}>
          <Button variant="ghost" pending={answer.pending === "dismiss"} disabled={held} onClick={answer.dismiss}>
            {actionOf("call_dismiss").verb}
            <Kbd>{keyFor("call_dismiss")}</Kbd>
          </Button>
        </Tooltip>
      )}
    </>
  );

  // The same acts as keycaps, for the consoles: the decision, where it goes next, and the one under a cover.
  const decideKeys = (
    <>
      {answer.attaching !== undefined
        ? null
        : acts.map((one) => {
            const at = answer.answers.indexOf(one);
            const first = actsOnly && answer.picked === at && !words;
            return (
              <Keycap key={one.id} label={one.label} keys={first ? "↵" : String(at + 1)} primary={first} pending={answer.pending === one.id} disabled={held && answer.pending !== one.id} {...(one.says === undefined ? {} : { says: one.says })} onPress={() => one.run()} />
            );
          })}
      {sleep === undefined ? null : <Keycap label={changeLabel} keys={keyFor("call_reply")} disabled={held} onPress={openChange} />}
      {actsOnly && !words ? null : <Keycap primary label={lone && !words ? openLabel : "Send"} keys="↵" disabled={!canSend} onPress={send} />}
    </>
  );
  const navKeys = (
    <>
      {answer.openWalk === undefined ? null : <Keycap label={actionOf("call_open_walk").verb} keys={keyFor("call_open_walk")} disabled={held} onPress={answer.openWalk} />}
      {lone || answer.open === undefined ? null : <Keycap label={openLabel} keys={keyFor("open")} disabled={held} onPress={answer.open} />}
      <Keycap label={actionOf("call_later").verb} keys={keyFor("call_later")} disabled={held} onPress={later} />
    </>
  );
  const coveredKey = removable ? (
    <Keycap
      label="Remove"
      keys={keyFor("call_dismiss")}
      pending={answer.pending === "dismiss"}
      disabled={held && answer.pending !== "dismiss"}
      says={answer.pending === "dismiss" ? OUT : "Remove, and tell the agent"}
      onPress={() => setRemoving(true)}
    />
  ) : (
    <CoveredKeycap
      label={actionOf("call_dismiss").verb}
      keys={keyFor("call_dismiss")}
      lifted={cover.lifted}
      pending={answer.pending === "dismiss"}
      disabled={held && answer.pending !== "dismiss"}
      says={answer.pending === "dismiss" ? OUT : "Never show this again"}
      onPress={cover.press}
    />
  );

  const card = (
    <AskCard icon={Icon} kind={item.kind} where={item.where} {...(item.at === undefined ? {} : { at: item.at })} now={now} title={item.title} hue={item.hue} leaving={leaving} from={from}>
        {session !== undefined ? (
          <div className="armada-callcard__thread" data-ran={answer.run !== undefined && answer.run.phase.state !== "idle" ? "" : undefined}>
            <SessionThread sessionId={session.id} rows={threadRowsOf(session)} onAnswer={() => {}} onOpenSession={hosts.onOpenSession} />
          </div>
        ) : (
          <>
            {answer.owner === undefined ? null : (
              <p className="armada-callcard__origin" data-owner role="group" aria-label="Owner">
                <Tooltip label="Job">
                  <span role="img" aria-label="Job">
                    <SquareTerminal size={12} aria-hidden="true" />
                  </span>
                </Tooltip>
                Job · {answer.owner.title}
              </p>
            )}
            {origin === "" ? null : <p className="armada-callcard__origin">{origin}</p>}
            {sleep !== undefined || item.context === undefined || item.context.length === 0 ? null : (
              <section aria-label={item.contextHead ?? "Why it asks"}>
                <pre className="armada-callcard__why">{item.context.slice(0, 3).join("\n")}</pre>
              </section>
            )}
            {holder === undefined ? null : (
              <div className="armada-callcard__thread" role="group" aria-label={`Job ${holder.title ?? holder.id}`}>
                <SessionThread sessionId={holder.id} rows={threadRowsOf(holder)} working={holder.turn.state === "working"} onAnswer={() => {}} onOpenSession={hosts.onOpenSession} />
              </div>
            )}
          </>
        )}
        <AskQuestion quiet={!(item.waiting !== undefined ? item.waiting.item.source !== "permission" : item.hue === "ask")}>
          {answer.ask}
        </AskQuestion>
        {answer.target === undefined ? null : (
          (() => {
            const target = (
              <Tooltip label={answer.target}>
                <code className="armada-callcard__target">{answer.target}</code>
              </Tooltip>
            );
            // A command he can run is the Session thread's own runnable block, driven by the rail's Run key too.
            return answer.run === undefined ? (
              target
            ) : (
              <div className="armada-callcard__run">
                <RunnableBlock command={answer.target} run={answer.run}>
                  {target}
                </RunnableBlock>
              </div>
            );
          })()
        )}
        {sleep === undefined || item.context === undefined ? null : (
          <section aria-label={item.contextHead ?? "Chose"}>
            <pre className="armada-callcard__why">{item.context.slice(0, 1).join("\n")}</pre>
          </section>
        )}
        {answer.attaching === undefined ? null : <AttachPicker attaching={answer.attaching} />}
        {numbered.length === 0 && standing.length === 0 || answer.attaching !== undefined ? null : (
          <AskChoices label={answer.ask}>
            <AskAnswers>
              {numbered.map((one, index) => (
                <AnswerTile key={one.id} one={one} mark={String(index + 1)} picked={answer.picked === answer.answers.indexOf(one)} pending={answer.pending === one.id} held={held} onPick={() => answer.pick(answer.answers.indexOf(one))} />
              ))}
            </AskAnswers>
            {standing.length === 0 ? null : (
              <AskAnswers standing>
                {standing.map((one) => (
                  <AnswerTile key={one.id} one={one} mark={one.key!} picked={answer.picked === answer.answers.indexOf(one)} pending={answer.pending === one.id} held={held} onPick={() => answer.pick(answer.answers.indexOf(one))} />
                ))}
              </AskAnswers>
            )}
          </AskChoices>
        )}
        {answer.remove === undefined ? null : (
          <div className="armada-callcard__remove">
            <RemoveAct name={item.fact} open={removing} onPress={() => setRemoving(!removing)} />
            {removing ? <RemoveBox name={item.fact} keep={`remove:${item.key}`} onRemove={answer.remove} onClose={() => setRemoving(false)} /> : null}
          </div>
        )}
        {boxed || answer.reply === undefined || (sleep !== undefined && !changing) ? null : (
          <Reply label={sleep === undefined ? "Type something" : "Change"} text={text} change={change} send={sendWords} pending={answer.pending === "reply"} held={held} {...(sleep === undefined ? {} : { focus: true, close: () => setChanging(false) })} />
        )}
        <div className="armada-callcard__acts" data-split={actsOnly || undefined} data-consoled>
          {actsOnly ? <div className="armada-callcard__decide">{decide}</div> : decide}
          {actsOnly ? <div className="armada-callcard__quiet">{quiet}</div> : quiet}
        </div>
        {!composed || home === undefined ? null : (
          <div className="armada-callcard__composer">
            <SessionBox session={home} onOpen={hosts.onOpenSession} onSend={sendComposed} />
          </div>
        )}
    </AskCard>
  );
  return (
    <>
      <aside className="armada-console armada-console--left" data-hue={item.hue}>
        {queue}
      </aside>
      {card}
      <aside className="armada-console armada-console--right" data-hue={item.hue}>
        <div className="armada-console__plate">
          {decideKeys}
          <hr className="armada-console__rule" />
          {navKeys}
          <hr className="armada-console__rule" />
          {coveredKey}
        </div>
      </aside>
    </>
  );
}
