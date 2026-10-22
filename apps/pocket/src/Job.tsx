// A Job: its whole thread, read from the Mac that holds it, with every reply the phone makes sent to
// that Mac. What it waits on sits at the end of the thread; a message box sits under it.

import { useEffect, useState } from "react";
import { AskCards, Button, SessionThread, Textarea } from "@armada/components";
import type { HelmCallAnswer, QuestionAnswer, WaitingItem } from "@armada/protocol";
import { lapsedAnswerText } from "@armada/screens/src/sessions-wire";

import { holderOf, macOf, start, useFleet } from "./fleet";
import { actsOn, attempt, useJob, type Acts } from "./jobState";
import type { Machine } from "./machines";
import { filePath, go, jobPath, sketchPath } from "./router";
import { approvalOf, askedOf, asksOf, rowsOf, sessionOfThread } from "./thread";
import { Band, Trouble } from "./ui";

/** Runs a reply and keeps its refusal to show. */
function useReply(machine: Machine): [string, (act: () => Promise<void>) => Promise<boolean>] {
  const [says, setSays] = useState("");
  const run = async (act: () => Promise<void>) => {
    setSays("");
    const refused = await attempt(act, machine);
    if (refused !== null) setSays(refused);
    return refused === null;
  };
  return [says, run];
}

/** Something the agent itself asked: its choices, or the owner's own words. */
function WaitingAsk({ item, keep, answer }: { item: WaitingItem; keep: string; answer: (given: { choice?: number; text?: string }) => Promise<boolean> }) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label={item.text}>
      <p className="pk-question">{item.text}</p>
      {(item.options ?? []).length === 0 ? null : (
        <div className="pk-choices" role="group" aria-label="Options">
          {(item.options ?? []).map((option, choice) => (
            <Button key={option.label} variant="secondary" className="pk-choice" title={option.description} onClick={() => void answer({ choice })}>{option.label}</Button>
          ))}
        </div>
      )}
      <Textarea keep={keep} rows={2} aria-label="Answer" placeholder="Answer" value={words} onChange={(event) => setWords(event.target.value)} />
      <Button variant="primary" disabled={words.trim() === ""} onClick={() => void answer({ text: words.trim() }).then((went) => went && setWords(""))}>Answer</Button>
    </div>
  );
}

function PlanChanges({ keep, send }: { keep: string; send: (text: string) => Promise<boolean> }) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label="Changes to the plan">
      <Textarea keep={keep} rows={2} aria-label="Changes to the plan" placeholder="Changes to the plan" value={words} onChange={(event) => setWords(event.target.value)} />
      <Button variant="secondary" disabled={words.trim() === ""} onClick={() => void send(words.trim()).then((went) => went && setWords(""))}>Send changes</Button>
    </div>
  );
}

function Thread({ machine, id, several }: { machine: Machine; id: string; several: boolean }) {
  const [read, reread] = useJob(machine, id);
  const [says, run] = useReply(machine);
  const [message, setMessage] = useState("");
  const [skipped, setSkipped] = useState<string | undefined>(undefined);
  const acts: Acts = actsOn(machine, reread);
  const home = () => go("/");
  if (read.thread === undefined) {
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={home} back="Home" />
        <div className="pk-body"><Trouble says={read.error ?? ""} /></div>
      </section>
    );
  }
  const session = sessionOfThread(read.thread);
  const keep = (what: string) => `pocket:${machine.origin}:${id}:${what}`;
  const asked = askedOf(session);
  const call = session.asked?.call;
  const plan = session.work?.plan;
  const approval = approvalOf(session, () => {
    if (plan?.version !== undefined) void run(() => acts.approvePlan({ session_id: id, version: plan.version! }));
  });
  const onAnswer = (answer?: string, answers?: QuestionAnswer[], folder?: string) =>
    call === undefined
      ? undefined
      : run(() => acts.answerAsk({ session_id: id, call, answer: (answer ?? "allow_once") as HelmCallAnswer, ...(answers === undefined ? {} : { answers }), ...(folder === undefined ? {} : { folder }) }));
  const lapsed =
    session.lapsed === undefined || session.lapsed.id === skipped
      ? undefined
      : {
          id: session.lapsed.id,
          questions: session.lapsed.questions,
          onAnswer: (answers: QuestionAnswer[]) => void run(() => acts.message({ session_id: id, text: lapsedAnswerText(answers) })),
          onSkip: () => setSkipped(session.lapsed?.id),
        };
  const planPending = approval !== undefined && !approval.approved && plan?.version !== undefined && approval.groups[0] !== undefined;
  const trailing = (
    <div className="pk-trailing">
      {asked === undefined && lapsed === undefined ? null : (
        <AskCards {...(asked === undefined ? {} : { asked, keep: keep(call ?? asked.command) })} onAnswer={onAnswer} {...(lapsed === undefined ? {} : { lapsed })} />
      )}
      {asksOf(session).map((item) => (
        <WaitingAsk key={item.id} item={item} keep={keep(`waiting:${item.id}`)} answer={(given) => run(() => acts.answerWaiting({ session_id: id, item_id: item.id, ...given }))} />
      ))}
      {planPending ? (
        <PlanChanges
          keep={keep("plan")}
          send={(text) => run(() => acts.planChanges({ session_id: id, version: plan!.version!, marks: [{ kind: "note", id: `phone-${Date.now()}`, group: approval!.groups[0]!.id, text }] }))}
        />
      ) : null}
      <Trouble says={says} />
    </div>
  );
  const send = () => void run(() => acts.message({ session_id: id, text: message.trim() })).then((went) => went && setMessage(""));
  return (
    <section className="pk-screen" aria-label={session.title ?? "Job"}>
      <Band title={session.title ?? ""} onBack={home} back="Home" end={several ? <span className="pk-band__mac">{machine.name}</span> : undefined} />
      <div className="pk-body pk-thread">
        <SessionThread
          sessionId={`${machine.origin} ${id}`}
          rows={rowsOf(session)}
          {...(approval === undefined ? {} : { approval })}
          working={session.turn.state === "working"}
          onAnswer={onAnswer}
          onOpenSession={(other) => go(jobPath(machine.origin, other))}
          onOpenFile={(path) => go(filePath(machine.origin, id, path))}
          onOpenSketch={(sketch) => go(sketchPath(machine.origin, id, sketch))}
          trailing={trailing}
        />
      </div>
      {session.dead !== undefined ? null : (
        <footer className="pk-bar pk-compose">
          <Textarea keep={keep("message")} rows={2} aria-label="Message" placeholder="Message" value={message} onChange={(event) => setMessage(event.target.value)} />
          <Button variant="primary" disabled={message.trim() === ""} onClick={send}>Send</Button>
        </footer>
      )}
    </section>
  );
}

/** Finds the Mac a Job is on, from the path or, for a push that did not say, by asking each Mac. */
export function Job({ origin, id }: { origin: string | undefined; id: string }) {
  const fleet = useFleet();
  const [found, setFound] = useState<string | undefined>(origin);
  const [lost, setLost] = useState(false);
  useEffect(() => {
    if (origin !== undefined) return;
    void holderOf(id).then((mac) => {
      if (mac === undefined) setLost(true);
      else go(jobPath(mac.machine.origin, id), true);
    });
  }, [origin, id]);
  useEffect(() => {
    void start();
    setFound(origin);
  }, [origin]);
  const mac = found === undefined ? undefined : macOf(found);
  if (mac === undefined) {
    const says = lost ? "No paired Mac holds that Job." : fleet.loaded && found !== undefined ? "This phone is not paired with that Mac." : "";
    return (
      <section className="pk-screen" aria-label="Job">
        <Band title="" onBack={() => go("/")} back="Home" />
        <div className="pk-body"><Trouble says={says} /></div>
      </section>
    );
  }
  return <Thread key={`${mac.machine.origin} ${id}`} machine={mac.machine} id={id} several={fleet.macs.length > 1} />;
}
