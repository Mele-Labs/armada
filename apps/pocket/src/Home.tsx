// Home: every paired Mac's Jobs in one list. Needs you first, oldest wait first, then Running, then
// Done. Where more than one Mac is paired, each row names its Mac. A Mac that forgot the phone, or is
// not answering, is a row of its own at the top, and the rest of the list carries on without it.

import { Laptop, Plus } from "lucide-react";
import { Button, SessionMark } from "@armada/components";

import { ago, homeOf, useFleet, type Item, type Mac } from "./fleet";
import { go, jobPath, pairPath } from "./router";
import { Band, Row, Section } from "./ui";

function MacRow({ mac }: { mac: Mac }) {
  const name = mac.machine.name === "" ? new URL(mac.machine.origin).hostname : mac.machine.name;
  return (
    <li className="pk-row pk-mac">
      <span className="pk-row__mark"><SessionMark state="quiet" said={mac.paired ? "Not answering" : "Not paired"} /></span>
      <span className="pk-row__title">{name}</span>
      <span className="pk-row__meta">{mac.paired ? mac.error : "Not paired"}</span>
      {mac.paired ? null : (
        <Button variant="secondary" size="sm" className="pk-row__act" onClick={() => go(pairPath(mac.machine.origin))}>Pair again</Button>
      )}
    </li>
  );
}

export function Home() {
  const fleet = useFleet();
  const { several, needs, running, done, troubled } = homeOf(fleet);
  const macName = (item: Item) => (several ? item.mac.machine.name : undefined);
  const open = (item: Item) => () => go(jobPath(item.mac.machine.origin, item.session.id));
  const row = (item: Item, state: "waiting" | "working" | "idle" | "ended", said: string, at: string) => {
    const mac = macName(item);
    return (
      <Row
        key={`${item.mac.machine.origin} ${item.session.id}`}
        state={state}
        said={said}
        title={item.session.title ?? ""}
        where={item.session.repository ?? ""}
        {...(mac === undefined ? {} : { mac })}
        age={ago(at, fleet.at)}
        onOpen={open(item)}
      />
    );
  };
  const quiet = needs.length + running.length + done.length === 0;
  return (
    <section className="pk-screen" aria-label="Home">
      <Band
        title="Armada"
        end={
          <>
            <button className="pk-icon" aria-label="Pair a Mac" onClick={() => go(pairPath())}><Laptop /></button>
            <button className="pk-icon" aria-label="Dispatch" onClick={() => go("/dispatch")}><Plus /></button>
          </>
        }
      />
      <div className="pk-body pk-home">
        {troubled.length === 0 ? null : (
          <section className="pk-section" aria-label="Macs">
            <ul className="pk-list">{troubled.map((mac) => <MacRow key={mac.machine.origin} mac={mac} />)}</ul>
          </section>
        )}
        {needs.length === 0 ? null : <Section label="Needs you">{needs.map((item) => row(item, "waiting", "Needs you", item.session.waiting_since ?? item.session.started_at))}</Section>}
        {running.length === 0 ? null : <Section label="Running">{running.map((item) => (item.session.working ? row(item, "working", "Working", item.session.started_at) : row(item, "idle", "Idle", item.session.started_at)))}</Section>}
        {done.length === 0 ? null : <Section label="Done">{done.map((item) => row(item, "ended", "Ended", item.session.ended_at ?? item.session.started_at))}</Section>}
        {quiet && fleet.loaded && troubled.length === 0 ? <p className="pk-fact pk-dim">Nothing needs you.</p> : null}
      </div>
    </section>
  );
}
