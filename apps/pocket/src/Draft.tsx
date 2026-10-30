// A draft Job: one held on its start card, which runs nothing until Send. Its brief is the first message the agent
// will read, edited here; its workflow, model and effort are read as they stand. Where its workflow asks how the work
// lands, the three landings are offered, Fleet's pick chosen. Send begins it with the brief as edited and the landing,
// Discard closes it.

import { useEffect, useState } from "react";
import { Quote, Repeat } from "lucide-react";
import { Button, Radio, RadioGroup, Textarea, Tooltip } from "@armada/components";
import { readKept } from "@armada/components/src/keep";
import type { DeliveryChoice, SessionThread as WireThread } from "@armada/protocol";

import { macOf, start, useFleet } from "./fleet";
import { actsOn, attempt, useJob } from "./jobState";
import type { Machine } from "./machines";
import { go, jobPath } from "./router";
import { Band, Figure, Trouble } from "./ui";

/** How a Job's work lands, in the order and words of Bridge's start card (`StartPicks.tsx`). */
const LANDS: readonly { choice: DeliveryChoice; label: string }[] = [
  { choice: "auto_land", label: "Auto land" },
  { choice: "open_pr", label: "Open PR only" },
  { choice: "keep_local", label: "Keep local" },
];

function Card({ machine, id, thread, reread }: { machine: Machine; id: string; thread: WireThread; reread: () => void }) {
  const record = thread.session;
  const keep = `pocket:${machine.origin}:${id}:brief`;
  const [brief, setBrief] = useState(() => readKept<string>(keep) ?? record.hosted?.start?.message ?? "");
  const start = record.hosted?.start;
  const picked = start?.picked;
  const [landing, setLanding] = useState<DeliveryChoice>(picked?.choice ?? "auto_land");
  const lands = start?.asks_delivery === true;
  // Marked while in use, only where Fleet took it from his usual or his words, as Bridge marks it.
  const standing = picked !== undefined && landing === picked.choice && (picked.from === "usual" || picked.from === "named") ? picked.from : undefined;
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
        {lands ? (
          <RadioGroup label="Lands">
            {LANDS.map((one) => (
              <Radio key={one.choice} name={`${keep}:lands`} value={one.choice} checked={landing === one.choice} onChange={() => setLanding(one.choice)}>
                <span className="pk-draft__land">
                  {one.label}
                  {standing === undefined || one.choice !== landing ? null : (
                    <Tooltip label={standing === "named" ? "Named in your request" : "Your usual"}>
                      <span role="img" aria-label={standing === "named" ? "Named in your request" : "Your usual"}>
                        {standing === "named" ? <Quote size={12} strokeWidth={2} aria-hidden="true" /> : <Repeat size={12} strokeWidth={2} aria-hidden="true" />}
                      </span>
                    </Tooltip>
                  )}
                </span>
              </Radio>
            ))}
          </RadioGroup>
        ) : null}
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
          onClick={() => void act(() => acts.begin({ session_id: id, message: brief.trim(), ...(lands ? { delivery: landing } : {}) }), jobPath(machine.origin, id))}
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
