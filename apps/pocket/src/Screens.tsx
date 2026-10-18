// The phone app's screens (#1997): a plain responsive page.
// Everything it reads comes through `./data`, from the Gateway.

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Check, ChevronLeft, CircleDot, LoaderCircle, Plus, SquareTerminal, Unplug } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "@armada/components";

import { Refused, lastRefusal } from "./client";
import { DRAFTS, ago, answer, attempt, dispatch as dispatchSession, repositories, startLive, useSessions } from "./data";
import type { PhoneSession } from "./gateway";
import { paired } from "./device";
import { claim, codeFromInput, codeFromUrl, defaultName, waitForConfirm } from "./pair";
import { subscribe } from "./push";
import { go } from "./router";

type Tab = "Needs you" | "Running" | "Done";
const TABS: Tab[] = ["Needs you", "Running", "Done"];

function Draft({ id, ...rest }: { id: string } & React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  const [value, setValue] = useState(() => DRAFTS.get(id) ?? "");
  return (
    <textarea
      {...rest}
      value={value}
      onChange={(event) => {
        DRAFTS.set(id, event.target.value);
        setValue(event.target.value);
      }}
    />
  );
}

function Mark({ icon: Icon, tip, className }: { icon?: LucideIcon; tip: string; className?: string }) {
  if (Icon === undefined) return <span className="pk-mark" title={tip} />;
  return <Icon className={className === undefined ? "pk-mark" : `pk-mark ${className}`} aria-label={tip} role="img"><title>{tip}</title></Icon>;
}

function Band({ title, back, onBack, end }: { title: string; back?: string; onBack?: () => void; end?: ReactNode }) {
  return (
    <header className="pk-band">
      {onBack !== undefined && (
        <button className="pk-icon" aria-label={back ?? "Back"} onClick={onBack}><ChevronLeft /></button>
      )}
      <h1 className="pk-band__title">{title}</h1>
      {end}
    </header>
  );
}

function Pair() {
  const [phase, setPhase] = useState<"scanned" | "waiting">("scanned");
  const [name, setName] = useState(() => defaultName(navigator.userAgent));
  const [trouble, setTrouble] = useState(lastRefusal());
  const urlCode = codeFromUrl(window.location.search);
  const [link, setLink] = useState("");
  const code = urlCode !== "" ? urlCode : codeFromInput(link);
  const pair = async () => {
    setTrouble("");
    try {
      await claim(name, code);
    } catch (why) {
      setTrouble(why instanceof Refused ? why.message : "The phone's key could not be made.");
      return;
    }
    setPhase("waiting");
    if (await waitForConfirm()) go("/");
    else setPhase("scanned");
  };
  return (
    <section className="pk-screen" aria-label="Pair">
      <Band title="Pair" />
      <div className="pk-body pk-pair">
        <div className="pk-code" data-phase={phase} aria-hidden="true">
          {Array.from({ length: 169 }, (_, i) => <i key={i} data-on={((i * 7 + (i >> 3) * 5) % 3 === 0) || i % 13 === 0} />)}
        </div>
        {phase === "scanned" ? (
          <>
            {urlCode === "" && <input className="pk-line" aria-label="Pairing link" placeholder="Pairing link" value={link} autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={(event) => setLink(event.target.value)} />}
            <input className="pk-line" aria-label="Name" value={name} autoCapitalize="words" onChange={(event) => setName(event.target.value)} />
            {trouble !== "" && <p className="pk-fact">{trouble}</p>}
          </>
        ) : (
          <p className="pk-fact pk-waiting"><LoaderCircle className="pk-spin" aria-hidden="true" /> Waiting for Confirm on your Mac</p>
        )}
      </div>
      {phase === "scanned" && (
        <footer className="pk-bar">
          <Button variant="primary" className="pk-big" disabled={code === "" || name.trim() === ""} onClick={() => void pair()}>Pair</Button>
        </footer>
      )}
    </section>
  );
}

function Row({ icon, tip, title, where, age, onOpen, spin }: { icon?: LucideIcon; tip: string; title: string; where: string; age: string; onOpen?: () => void; spin?: boolean }) {
  const body = (
    <>
      <Mark icon={icon} tip={tip} className={spin === true ? "pk-live" : undefined} />
      <span className="pk-row__title">{title}</span>
      <span className="pk-row__meta">{where}</span>
      <span className="pk-row__age">{age}</span>
    </>
  );
  return onOpen === undefined ? <li className="pk-row" data-still>{body}</li> : <li><button className="pk-row" onClick={onOpen}>{body}</button></li>;
}

function sessionRow(session: PhoneSession) {
  const age = session.waiting_since === undefined ? "" : ago(session.waiting_since, Date.now());
  const title = session.title ?? "";
  const where = session.repository ?? "";
  if (session.ask === undefined) {
    return session.kind === "terminal" ? <Row key={session.id} icon={SquareTerminal} tip="Terminal job waiting" title={title} where={where} age={age} /> : null;
  }
  return <Row key={session.id} icon={SquareTerminal} tip="Job asks" title={title} where={where} age={age} onOpen={() => go(`/sessions/${session.id}`)} />;
}

function Tabs() {
  const [tab, setTab] = useState<Tab>("Needs you");
  const { needs, running, done, error } = useSessions();
  return (
    <section className="pk-screen" aria-label={tab}>
      <Band title={tab} end={<button className="pk-icon" aria-label="Dispatch" onClick={() => go("/dispatch")}><Plus /></button>} />
      <div className="pk-body" role="tabpanel" aria-label={tab}>
        {error !== undefined && <p className="pk-fact"><Unplug aria-hidden="true" /> {error}</p>}
        {tab === "Needs you" && (
          <ul className="pk-list">
            {needs.map(sessionRow)}
          </ul>
        )}
        {tab === "Running" && (
          <ul className="pk-list">
            {running.map((session) => (
              <Row key={session.id} icon={CircleDot} tip="Running" spin title={session.title ?? ""} where={session.repository ?? ""} age={ago(session.started_at, Date.now())} />
            ))}
          </ul>
        )}
        {tab === "Done" && (
          <ul className="pk-list">
            {done.map((session) => (
              <Row key={session.id} icon={Check} tip="Ended" title={session.title ?? ""} where={session.repository ?? ""} age={ago(session.ended_at ?? session.started_at, Date.now())} />
            ))}
          </ul>
        )}
      </div>
      <nav className="pk-tabs" role="tablist" aria-label="Armada">
        {TABS.map((name) => (
          <button key={name} role="tab" aria-selected={tab === name} className="pk-tab" onClick={() => setTab(name)}>{name}</button>
        ))}
      </nav>
    </section>
  );
}

/** Runs an act; on a 204 the screen goes back, on a refusal it shows the Gateway's sentence and keeps what was typed. */
function useRun(done: () => void): [string, (act: () => Promise<void>, draftIds?: string[]) => void] {
  const [trouble, setTrouble] = useState("");
  const run = (act: () => Promise<void>, draftIds: string[] = []) => {
    setTrouble("");
    void attempt(act, draftIds).then((says) => (says === null ? done() : setTrouble(says)));
  };
  return [trouble, run];
}

const Trouble = ({ says }: { says: string }) => (says === "" ? null : <p className="pk-fact">{says}</p>);

function AskScreen({ session, back, done }: { session: PhoneSession; back: () => void; done: () => void }) {
  const ask = session.ask!;
  const questions = ask.questions ?? [];
  const [picked, setPicked] = useState<Record<number, string[]>>({});
  const [trouble, run] = useRun(done);
  const draftId = (i: number) => (i === 0 ? `ask-${session.id}` : `ask-${session.id}-${i}`);
  const choose = (i: number, option: string, multi: boolean) =>
    setPicked((was) => {
      const now = was[i] ?? [];
      return { ...was, [i]: multi ? (now.includes(option) ? now.filter((one) => one !== option) : [...now, option]) : [option] };
    });
  const permission = questions.length === 0;
  const send = () => run(() => answer({
    session_id: session.id,
    ask_id: ask.ask_id,
    answer: questions.map((q, i) => {
      const own = (DRAFTS.get(draftId(i)) ?? "").trim();
      return { question: q.question, chosen: own !== "" ? [own] : (picked[i] ?? []) };
    }),
  }), questions.map((_, i) => draftId(i)));
  const decide = (decision: "allow_once" | "refuse") => run(() => answer({ session_id: session.id, ask_id: ask.ask_id, answer: decision }));
  return (
    <section className="pk-screen" aria-label={session.title ?? ""}>
      <Band title={session.title ?? ""} onBack={back} back="Back to Needs you" />
      <div className="pk-body">
        {permission && <p className="pk-question">{ask.detail ?? ask.tool}</p>}
        {questions.map((q, i) => (
          <div key={i}>
            <p className="pk-question">{q.question}</p>
            <div className="pk-choices" role={q.multi_select ? "group" : "radiogroup"} aria-label="Answer">
              {q.options.map((option) => (
                <button key={option.label} role={q.multi_select ? "checkbox" : "radio"} aria-checked={(picked[i] ?? []).includes(option.label)} className="pk-choice" onClick={() => choose(i, option.label, q.multi_select)}>{option.label}</button>
              ))}
            </div>
            <Draft id={draftId(i)} className="pk-dictate" rows={3} placeholder="Other" aria-label="Other answer" enterKeyHint="send" autoCapitalize="sentences" autoCorrect="on" spellCheck />
          </div>
        ))}
        <Trouble says={trouble} />
      </div>
      <footer className="pk-bar">
        {permission ? (
          <div className="pk-pair-acts">
            {ask.offers.includes("refuse") && <Button variant="secondary" className="pk-big" onClick={() => decide("refuse")}>Refuse</Button>}
            {ask.offers.includes("allow_once") && <Button variant="primary" className="pk-big" onClick={() => decide("allow_once")}>Allow once</Button>}
          </div>
        ) : (
          <Button variant="primary" className="pk-big" onClick={send}>Answer</Button>
        )}
      </footer>
    </section>
  );
}

function DispatchScreen({ back, done }: { back: () => void; done: () => void }) {
  const [repos, setRepos] = useState<string[]>([]);
  const [repo, setRepo] = useState<string | undefined>(undefined);
  const [line, setLine] = useState(() => DRAFTS.get("dispatch") ?? "");
  const [trouble, run] = useRun(done);
  useEffect(() => {
    repositories().then((names) => { setRepos(names); setRepo((was) => was ?? names[0]); }, (why) => run(() => Promise.reject(why)));
  }, []);
  return (
    <section className="pk-screen" aria-label="Dispatch">
      <Band title="Dispatch" onBack={back} back="Back" />
      <div className="pk-body">
        <input
          className="pk-line" aria-label="Job" placeholder="Job" value={line} enterKeyHint="send"
          autoCapitalize="sentences" autoCorrect="on"
          onChange={(event) => { DRAFTS.set("dispatch", event.target.value); setLine(event.target.value); }}
        />
        <div className="pk-choices" role="radiogroup" aria-label="Repository">
          {repos.map((name) => (
            <button key={name} role="radio" aria-checked={repo === name} className="pk-choice" onClick={() => setRepo(name)}>{name}</button>
          ))}
        </div>
        <Trouble says={trouble} />
      </div>
      <footer className="pk-bar">
        <Button variant="primary" className="pk-big" disabled={line.trim() === "" || repo === undefined} onClick={() => run(() => dispatchSession({ text: line.trim(), repository: repo! }), ["dispatch"])}>Dispatch</Button>
      </footer>
    </section>
  );
}

export function App({ path }: { path: string }) {
  const home = () => go("/");
  const { needs } = useSessions();
  const asked = path.startsWith("/sessions/") ? needs.find((one) => one.id === path.slice("/sessions/".length) && one.ask !== undefined) : undefined;
  useEffect(() => {
    if (path === "/pair") return;
    void paired().then((device) => {
      if (device === undefined) go("/pair");
      else {
        void startLive();
        void subscribe().catch(() => undefined);
      }
    });
  }, [path === "/pair"]);
  return (
    <main className="pk-app" aria-label="Armada">
      {path === "/pair" && <Pair />}
      {path === "/" && <Tabs />}
      {asked !== undefined && <AskScreen key={asked.id} session={asked} back={home} done={home} />}
      {path === "/dispatch" && <DispatchScreen back={home} done={home} />}
    </main>
  );
}
