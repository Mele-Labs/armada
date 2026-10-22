// Dispatch: a line of text and a repository, sent to the Mac that serves that repository. With more
// than one Mac paired, each repository names its Mac.

import { useEffect, useState } from "react";
import { Button, Textarea } from "@armada/components";

import { call, getJson, refusal } from "./client";
import { loadLists, start, useFleet } from "./fleet";
import type { DispatchBody, RepositoriesBody } from "./gateway";
import type { Machine } from "./machines";
import { go } from "./router";
import { Band, Trouble } from "./ui";

type Choice = { machine: Machine; repository: string };

export function Dispatch() {
  const fleet = useFleet();
  const [choices, setChoices] = useState<Choice[]>([]);
  const [picked, setPicked] = useState<Choice | undefined>(undefined);
  const [line, setLine] = useState("");
  const [trouble, setTrouble] = useState("");
  const paired = fleet.macs.filter((mac) => mac.paired);
  const several = fleet.macs.length > 1;
  useEffect(() => void start(), []);
  useEffect(() => {
    let gone = false;
    void Promise.all(
      paired.map((mac) => getJson<RepositoriesBody>(mac.machine, "/api/repositories").then((names) => names.map((repository) => ({ machine: mac.machine, repository })), () => [])),
    ).then((each) => {
      if (gone) return;
      const all = each.flat();
      setChoices(all);
      setPicked((was) => was ?? all[0]);
    });
    return () => {
      gone = true;
    };
  }, [paired.map((mac) => mac.machine.origin).join(" ")]);
  const send = async () => {
    if (picked === undefined) return;
    setTrouble("");
    const body: DispatchBody = { text: line.trim(), repository: picked.repository };
    try {
      await call(picked.machine, "/api/sessions", { method: "POST", body });
    } catch (why) {
      setTrouble(refusal(why, picked.machine));
      return;
    }
    setLine("");
    void loadLists(picked.machine.origin);
    go("/");
  };
  return (
    <section className="pk-screen" aria-label="Dispatch">
      <Band title="Dispatch" onBack={() => go("/")} back="Home" />
      <div className="pk-body">
        <Textarea keep="pocket:dispatch" rows={3} aria-label="Job" placeholder="Job" value={line} enterKeyHint="send" autoCapitalize="sentences" onChange={(event) => setLine(event.target.value)} />
        <div className="pk-choices" role="radiogroup" aria-label="Repository">
          {choices.map((choice) => (
            <button
              key={`${choice.machine.origin} ${choice.repository}`}
              role="radio"
              aria-checked={picked === choice}
              className="pk-choice"
              onClick={() => setPicked(choice)}
            >
              {choice.repository}
              {several ? <span className="pk-row__mac">{choice.machine.name}</span> : null}
            </button>
          ))}
        </div>
        <Trouble says={trouble} />
      </div>
      <footer className="pk-bar">
        <Button variant="primary" className="pk-big" disabled={line.trim() === "" || picked === undefined} onClick={() => void send()}>Dispatch</Button>
      </footer>
    </section>
  );
}
