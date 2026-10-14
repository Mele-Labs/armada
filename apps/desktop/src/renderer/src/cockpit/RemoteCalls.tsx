// What another machine's Sessions wait on the owner for, on Your move, read-only. Two takes, chosen by
// the scenario (`OtherMachine.onCockpit`): a strip of single lines under the glass, or cards the size
// of the real ones in the deck. Neither is a call: they are not in the deck's list, so its keys never
// reach them, and neither draws a key or a button. Mock only, as the Cockpit's Dashboard is.

import { Eye } from "lucide-react";
import { AskCard, AskQuestion, MachineMark, QuietMark, since } from "@armada/components";
import type { OtherMachine } from "@armada/components";

import { age } from "../Dashboard";

export type RemoteWaiting = { key: string; machine: string; title: string; text: string; at: string; quiet?: string };

/** Every item the other machine's Sessions wait on, the one that began first leading. */
export function remoteWaitingOf(other: OtherMachine | undefined, now: number): RemoteWaiting[] {
  if (other === undefined) return [];
  const quiet = other.lastHeard === undefined ? undefined : `Last heard ${since(other.lastHeard, now) ?? ""} ago`;
  return other.sessions
    .filter((one) => one.state === "waiting")
    .flatMap((one) => (one.waiting ?? []).map((text) => ({ key: `${other.name}:${one.id}:${text}`, machine: other.name, title: one.title, text, at: one.since, ...(quiet === undefined ? {} : { quiet }) })))
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at));
}

/** One line to an item: the machine, what it waits on, the Session it is in, and how long. */
export function RemoteStrip({ items, now }: { items: readonly RemoteWaiting[]; now: number }) {
  if (items.length === 0) return null;
  return (
    <ul className="armada-remote-strip" aria-label="Waiting on the other machine">
      {items.map((one) => (
        <li key={one.key} className="armada-remote-strip__line" data-away={one.quiet === undefined ? undefined : ""}>
          <MachineMark name={one.machine} />
          {one.quiet === undefined ? null : <QuietMark said={one.quiet} />}
          <span className="armada-remote-strip__text">{one.text}</span>
          <span className="armada-remote-strip__title">{one.title}</span>
          <span className="armada-remote-strip__age">{age(one.at, now)}</span>
        </li>
      ))}
    </ul>
  );
}

/** The real card's shape without its keys: the band, the Session, what it waits on. Dashed, and dimmer once the machine is quiet. */
export function RemoteCards({ items, now }: { items: readonly RemoteWaiting[]; now: number }) {
  return (
    <>
      {items.map((one) => (
        <div key={one.key} className="armada-remote-card" data-away={one.quiet === undefined ? undefined : ""}>
          <AskCard
            icon={Eye}
            kind="Session question"
            where={
              <span className="armada-remote-card__marks">
                {one.quiet === undefined ? null : <QuietMark said={one.quiet} />}
                <MachineMark name={one.machine} />
              </span>
            }
            age={age(one.at, now)}
            title={one.title}
            label={`${one.machine}: ${one.title}`}
            hue="ask"
          >
            <AskQuestion>{one.text}</AskQuestion>
          </AskCard>
        </div>
      ))}
    </>
  );
}
