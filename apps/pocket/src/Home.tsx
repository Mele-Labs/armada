// Home: every paired Mac's live Jobs in one list. Needs you first, oldest wait first, then Running.
// Where more than one Mac is paired, each row names its Mac. A Mac that forgot the phone, or is
// not answering, is a row of its own at the top, and the rest of the list carries on without it.

import { Laptop, Plus } from "lucide-react";
import { Button, SessionMark } from "@armada/components";

import { homeOf, useFleet, type Item, type Mac } from "./fleet";
import { go, jobPath, pairPath } from "./router";
import { Band, IconButton, Row, Section } from "./ui";

function MacRow({ mac }: { mac: Mac }) {
  const name = mac.machine.name === "" ? new URL(mac.machine.origin).hostname : mac.machine.name;
  return (
    <li className="deck-list__item">
      <div className="deck-row" data-tone="bad">
        <span className="deck-row__title">{name}</span>
        <span className="deck-row__marks"><SessionMark state="quiet" said={mac.paired ? "Not answering" : "Not paired"} /></span>
        <span className="deck-row__says">{mac.paired ? mac.error : "Not paired"}</span>
        <span className="deck-row__said">
          {mac.paired ? null : <Button variant="secondary" size="sm" onClick={() => go(pairPath(mac.machine.origin))}>Pair again</Button>}
        </span>
      </div>
    </li>
  );
}

export function Home() {
  const fleet = useFleet();
  const { several, needs, running, troubled } = homeOf(fleet);
  const row = (item: Item, state: "waiting" | "working" | "idle", said: string, at: string) => {
    const key = `${item.mac.machine.origin} ${item.session.id}`;
    const open = () => go(jobPath(item.mac.machine.origin, item.session.id));
    const mac = several ? item.mac.machine.name : undefined;
    return <Row key={key} state={state} said={said} title={item.session.title ?? ""} says={item.session.repository ?? ""} {...(mac === undefined ? {} : { mac })} at={at} onOpen={open} />;
  };
  return (
    <section className="pk-screen" aria-label="Home">
      <Band
        title="Armada"
        end={
          <>
            <IconButton label="Pair a Mac" onClick={() => go(pairPath())}><Laptop size={16} aria-hidden="true" /></IconButton>
            <IconButton label="Dispatch" onClick={() => go("/dispatch")}><Plus size={16} aria-hidden="true" /></IconButton>
          </>
        }
      />
      <div className="pk-body pk-home">
        {troubled.length === 0 ? null : (
          <section className="pk-section" aria-label="Macs">
            <ul className="deck-list__rows">{troubled.map((mac) => <MacRow key={mac.machine.origin} mac={mac} />)}</ul>
          </section>
        )}
        {needs.length === 0 ? null : <Section label="Needs you">{needs.map((item) => row(item, "waiting", "Needs you", item.session.waiting_since ?? item.session.started_at))}</Section>}
        {running.length === 0 ? null : <Section label="Running">{running.map((item) => (item.session.working ? row(item, "working", "Working", item.session.started_at) : row(item, "idle", "Idle", item.session.started_at)))}</Section>}
      </div>
    </section>
  );
}
