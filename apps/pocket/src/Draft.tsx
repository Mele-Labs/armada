// A draft Job: one held on its start card, which runs nothing until Send. Its brief is the first message the agent
// will read, edited here; its workflow, model and effort are read as they stand. Send begins it with the brief as
// edited, Discard closes it.

import { useEffect, useState } from "react";
import { Button, Textarea } from "@armada/components";
import { readKept } from "@armada/components/src/keep";
import type { SessionThread as WireThread } from "@armada/protocol";

import { macOf, start, useFleet } from "./fleet";
import { actsOn, attempt, useJob } from "./jobState";
import type { Machine } from "./machines";
import { go, jobPath } from "./router";
import { Band, Figure, Trouble } from "./ui";

function Card({ machine, id, thread, reread }: { machine: Machine; id: string; thread: WireThread; reread: () => void }) {
  const record = thread.session;
  const keep = `pocket:${machine.origin}:${id}:brief`;
  const [brief, setBrief] = useState(() => readKept<string>(keep) ?? record.hosted?.start?.message ?? "");
  const [says, setSays] = useState("");
  const acts = actsOn(machine, reread);
  const act = async (run: () => Promise<void>, then: string) => {
    setSays("");
    const refused = await attempt(run, machine);
    if (refused !== null) return setSays(refused);
    setBrief("");
    go(then, true);
  };
  const facts: readonly [string, string | undefined][] = [
    ["Workflow", record.workflow],
    ["Model", record.hosted?.model],
    ["Effort", record.hosted?.effort],
  ];
  return (
    <section className="pk-screen" aria-label={record.title ?? "Draft"}>
      <Band title="Draft" onBack={() => go("/")} back="Home" />
      <div className="pk-body pk-draft">
        <h2 className="pk-draft__title">{record.title}</h2>
        <Textarea keep={keep} grow rows={8} aria-label="Brief" placeholder="Brief" value={brief} autoCapitalize="sentences" onChange={(event) => setBrief(event.target.value)} />
        <div className="pk-draft__facts">
          {facts.flatMap(([name, value]) => (value === undefined ? [] : [<Figure key={name} name={name} loose>{value}</Figure>]))}
        </div>
        <Trouble says={says} />
      </div>
      <footer className="pk-bar pk-bar--pair">
        <Button variant="ghost" className="pk-big" onClick={() => void act(() => acts.close({ session_id: id }), "/")}>Discard</Button>
        <Button
          variant="primary"
          className="pk-big"
          disabled={brief.trim() === ""}
          onClick={() => void act(() => acts.begin({ session_id: id, message: brief.trim() }), jobPath(machine.origin, id))}
        >
          Send
        </Button>
      </footer>
    </section>
  );
}

export function Draft({ origin, id }: { origin: string; id: string }) {
  const fleet = useFleet();
  useEffect(() => void start(), []);
  const mac = macOf(origin);
  const [read, reread] = useJob(mac?.machine, id);
  const held = read.thread?.session.hosted?.start !== undefined;
  // A Job that has begun, here or on the Mac, opens as itself.
  useEffect(() => {
    if (read.thread !== undefined && !held) go(jobPath(origin, id), true);
  }, [read.thread, held, origin, id]);
  if (mac === undefined || read.thread === undefined || !held) {
    const says = mac === undefined && fleet.loaded ? "This phone is not paired with that Mac." : (read.error ?? "");
    return (
      <section className="pk-screen" aria-label="Draft">
        <Band title="Draft" onBack={() => go("/")} back="Home" />
        <div className="pk-body"><Trouble says={says} /></div>
      </section>
    );
  }
  return <Card machine={mac.machine} id={id} thread={read.thread} reread={reread} />;
}
