// The call in front: what needs the owner, over the glass, answerable without the pointer. Its answers
// carry the numbers that pick them, Enter sends, `l` puts it off for later, `o` opens what it is
// about. The keys themselves are `Cockpit`'s, on the window; this draws them and takes the press.
// Mock only, as the Dashboard is.

import type { MutableRefObject } from "react";
import { Button, Kbd } from "@armada/components";

import type { BridgeState } from "../../../shared/bridge";
import { age, type Hosts, type Item } from "../Dashboard";
import { useAnswering, type Answering } from "./answers";

/** What `Enter` does where a call has no answer of its own: it opens the thing. */
const OPENS = (item: Item) => (item.key.startsWith("session:") ? "Open Session" : item.key.startsWith("pull:") ? "Open pull request" : "Open Job");

export function CallCard({
  item,
  now,
  state,
  hosts,
  finish,
  later,
  leaving,
  answering,
}: {
  item: Item;
  now: number;
  state: BridgeState;
  hosts: Hosts;
  /** The call is answered: it is cleared from the queue. */
  finish: () => void;
  /** Put it off: it goes to the waiting calls at the edge. */
  later: () => void;
  leaving: "later" | "sent" | undefined;
  /** Where the card hands its answers up to, for the keys. */
  answering: MutableRefObject<Answering | undefined>;
}) {
  const answer = useAnswering(item, hosts, state, finish);
  answering.current = answer;
  const Icon = item.icon;
  const asking = item.hue === "ask";
  const lone = answer.answers.length === 0;

  return (
    <section className="armada-callcard" data-hue={item.hue} data-leaving={leaving} aria-label={`${item.kind}: ${item.title}`}>
      <header className="armada-callcard__band">
        <Icon size={16} aria-hidden="true" />
        <span>{item.kind}</span>
        <span className="armada-callcard__where">{item.where}</span>
        <span className="armada-callcard__age">{age(item.at, now)}</span>
      </header>
      <div className="armada-callcard__body">
        <h2 className="armada-callcard__title">{item.title}</h2>
        {item.context === undefined || item.context.length === 0 ? null : (
          <section aria-label={item.contextHead ?? "Why it asks"}>
            <pre className="armada-callcard__why">{item.context.slice(0, 4).join("\n")}</pre>
          </section>
        )}
        <p className="armada-callcard__ask" data-quiet={(asking && !item.key.startsWith("session:")) || answer.decision !== undefined ? undefined : ""}>
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
          <div className="armada-callcard__answers" role="radiogroup" aria-label={answer.ask}>
            {answer.answers.map((one, index) => (
              <button
                key={one.id}
                type="button"
                role="radio"
                aria-checked={answer.picked === index}
                className="armada-callcard__answer"
                onClick={() => answer.pick(index)}
              >
                <Kbd>{index + 1}</Kbd>
                {one.label}
              </button>
            ))}
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
