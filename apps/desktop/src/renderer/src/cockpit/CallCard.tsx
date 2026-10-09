// The call in front: what needs the owner, over the glass, answerable without the pointer. Above the
// question it says what the Job is and what he asked it for, where the question came from, and what
// is active now in one line; a Session's call is a small view of the Session instead. Its answers
// carry the numbers that pick them, and two standing ones carry `b` and `g`. Enter sends, `l` puts it
// off for later, `o` opens what it is about, `e` opens the whole request. The keys themselves are
// `Cockpit`'s, on the window; this draws them and takes the press. Mock only, as the Dashboard is.

import { useLayoutEffect, useRef, useState, type MutableRefObject } from "react";
import { Bot, Box, ChevronDown, ChevronUp, Cpu, Scale, ShieldCheck, Waypoints, Workflow, type LucideIcon } from "lucide-react";
import { Button, Kbd, SessionThread, Tooltip, type NowPanelProps } from "@armada/components";

import type { BridgeState } from "../../../shared/bridge";
import { age, type Hosts, type Item } from "../Dashboard";
import { useSessions } from "../sessions-draft";
import { threadRowsOf } from "../sessions";
import { useAnswering, type Answer, type Answering } from "./answers";

/** What `Enter` does where a call has no answer of its own: it opens the thing. */
const OPENS = (item: Item) => (item.key.startsWith("session:") ? "Open Session" : item.key.startsWith("pull:") ? "Open pull request" : "Open Job");

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

/** One answer: its number or its own key, and what it tells the agent where it says. */
function AnswerTile({ one, mark, picked, onPick }: { one: Answer; mark: string; picked: boolean; onPick: () => void }) {
  const tile = (
    <button type="button" role="radio" aria-checked={picked} className="armada-callcard__answer" data-standing={one.key === undefined ? undefined : ""} onClick={onPick}>
      <Kbd>{mark}</Kbd>
      {one.label}
    </button>
  );
  return one.says === undefined ? tile : <Tooltip label={one.says}>{tile}</Tooltip>;
}

export function CallCard({
  item,
  now,
  nowing,
  state,
  hosts,
  finish,
  later,
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

  const sessionId = item.key.startsWith("session:") ? item.key.slice("session:".length) : undefined;
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
                    <Kbd>e</Kbd>
                  </Button>
                )}
              </div>
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
        <p className="armada-callcard__ask" data-quiet={(item.hue === "ask" && sessionId === undefined) || answer.decision !== undefined ? undefined : ""}>
          {answer.ask}
          {answer.decision === undefined ? null : (
            <span className="armada-callcard__pips" aria-hidden="true">
              {Array.from({ length: answer.decision.of }, (_, index) => (
                <span key={index} className="armada-callcard__pip" data-here={index === answer.decision!.at || undefined} />
              ))}
            </span>
          )}
        </p>
        {lone ? null : (
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
        <div className="armada-callcard__acts">
          <Button variant="primary" disabled={!answer.canSend} onClick={answer.send}>
            {lone ? OPENS(item) : "Send"}
            <Kbd>↵</Kbd>
          </Button>
          {lone || answer.open === undefined ? null : (
            <Button variant="ghost" onClick={answer.open}>
              {OPENS(item)}
              <Kbd>o</Kbd>
            </Button>
          )}
          <Button variant="ghost" onClick={later}>
            Later
            <Kbd>l</Kbd>
          </Button>
        </div>
      </div>
    </section>
  );
}
