// Home: every paired Mac's live Jobs in one list. Drafts first, then Needs you, oldest wait first, then Running.
// Over it, the plan's two limits as the newest Mac's reading has them.
// Where more than one Mac is paired, each row names its Mac. A Mac that forgot the phone, or is
// not answering, is a row of its own at the top, and the rest of the list carries on without it.

import { useEffect, useState } from "react";
import { Laptop, Plus } from "lucide-react";
import { Button, SessionMark } from "@armada/components";

import { getJson } from "./client";
import { homeOf, useFleet, type Item, type Mac } from "./fleet";
import type { PlanUsageBody } from "./gateway";
import { draftPath, go, jobPath, pairPath } from "./router";
import { Band, Figure, IconButton, Row, Section } from "./ui";

const LIMITS = [
  ["five_hour", "5h", "5-hour limit"],
  ["seven_day", "wk", "Weekly limit"],
] as const;

/** The newest reading any paired Mac holds, read again whenever the Macs' lists change. */
function usePlanUsage(macs: readonly Mac[], at: number): PlanUsageBody | undefined {
  const [usage, setUsage] = useState<PlanUsageBody | undefined>(undefined);
  const paired = macs.filter((mac) => mac.paired);
  useEffect(() => {
    let gone = false;
    void Promise.all(paired.map((mac) => getJson<PlanUsageBody>(mac.machine, "/api/plan_usage").catch(() => undefined))).then((each) => {
      if (gone) return;
      const read = each.filter((one): one is PlanUsageBody => one !== undefined && one.read_at !== undefined);
      setUsage(read.sort((a, b) => Date.parse(b.read_at!) - Date.parse(a.read_at!))[0]);
    });
    return () => {
      gone = true;
    };
  }, [paired.map((mac) => mac.machine.origin).join(" "), at]);
  return usage;
}

function Usage({ usage }: { usage: PlanUsageBody | undefined }) {
  const drawn = LIMITS.flatMap(([kind, short, name]) => {
    const one = usage?.windows.find((window) => window.kind === kind);
    return one === undefined ? [] : [{ short, name, percent: Math.round(one.percent_used) }];
  });
  if (drawn.length === 0) return null;
  return (
    <div className="pk-usage" role="group" aria-label="Plan limits">
      {drawn.map((one) => (
        <span key={one.short} className="pk-usage__limit">
          <span className="pk-usage__name" aria-hidden="true">{one.short}</span>
          <Figure name={one.name}>{`${one.percent}%`}</Figure>
        </span>
      ))}
    </div>
  );
}

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
  const { several, drafts, needs, running, troubled } = homeOf(fleet);
  const usage = usePlanUsage(fleet.macs, fleet.at);
  const row = (item: Item, state: "waiting" | "working" | "idle" | "draft", said: string, at: string) => {
    const { session } = item;
    const key = `${item.mac.machine.origin} ${session.id}`;
    const open = () => go((session.held ? draftPath : jobPath)(item.mac.machine.origin, session.id));
    const mac = several ? item.mac.machine.name : undefined;
    return (
      <Row
        key={key}
        state={state}
        said={said}
        title={session.title ?? ""}
        says={session.repository ?? ""}
        {...(mac === undefined ? {} : { mac })}
        {...(session.from === undefined ? {} : { from: session.from.title })}
        {...(session.context_tokens === undefined ? {} : { context: session.context_tokens })}
        at={at}
        onOpen={open}
      />
    );
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
      <Usage usage={usage} />
      <div className="pk-body pk-home">
        {troubled.length === 0 ? null : (
          <section className="pk-section" aria-label="Macs">
            <ul className="deck-list__rows">{troubled.map((mac) => <MacRow key={mac.machine.origin} mac={mac} />)}</ul>
          </section>
        )}
        {drafts.length === 0 ? null : <Section label="Drafts">{drafts.map((item) => row(item, "draft", "Draft", item.session.started_at))}</Section>}
        {needs.length === 0 ? null : <Section label="Needs you">{needs.map((item) => row(item, "waiting", "Needs you", item.session.waiting_since ?? item.session.started_at))}</Section>}
        {running.length === 0 ? null : <Section label="Running">{running.map((item) => (item.session.working ? row(item, "working", "Working", item.session.started_at) : row(item, "idle", "Idle", item.session.started_at)))}</Section>}
      </div>
    </section>
  );
}
