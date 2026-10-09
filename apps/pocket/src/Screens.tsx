// The phone app's screens (#1997): a plain responsive page, fixture data only.
// Everything it reads comes through `./data`, which the Gateway client replaces (#1998).

import { useState } from "react";
import type { ReactNode } from "react";
import {
  Check, ChevronLeft, CircleDot, Eye, GitPullRequest, LoaderCircle, OctagonAlert, Plus, RefreshCw,
  Split, SquareTerminal, Unplug, UserCheck, X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button, HoldButton } from "@armada/components";

import { APPROVAL, BLOCKED, DONE, HOSTED_ASK, MAC, REPOSITORIES, REVIEW, RUNNING, TERMINAL_WAITING, jobById } from "./data";
import type { PocketJob } from "./data";
import { Link, go } from "./router";

type Tab = "Needs you" | "Running" | "Done";
const TABS: Tab[] = ["Needs you", "Running", "Done"];

/** Glyphs and verbs are `crates/core-model/domain/enum-verbs.toml`'s. */
const REASON: Record<NonNullable<PocketJob["reason"]>, { icon?: LucideIcon; says: string }> = {
  stalled: { icon: OctagonAlert, says: "Stalled" },
  thrashing: { icon: RefreshCw, says: "Churning" },
  rate_cap: { icon: Split, says: "Hit the sub-dispatch cap" },
  interrupted: { icon: Unplug, says: "Interrupted" },
};

type Act = "approve" | "redirect" | "restart" | "redispatch" | "kill";
/** What a state allows. `approve_dispatch` means something only on a Job awaiting dispatch approval. */
const ACTS: Record<NonNullable<PocketJob["reason"]> | "approval", Set<Act>> = {
  approval: new Set(["approve"]),
  stalled: new Set(["redirect", "restart", "redispatch", "kill"]),
  thrashing: new Set(["redirect", "restart", "redispatch", "kill"]),
  rate_cap: new Set(["restart", "kill"]),
  interrupted: new Set(["restart", "redispatch", "kill"]),
};

/** Drafts live outside the screens, so leaving one and coming back finds what was typed. */
const DRAFTS = new Map<string, string>();

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

const drafted = (id: string) => (DRAFTS.get(id) ?? "").trim() !== "";

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
  return (
    <section className="pk-screen" aria-label="Pair">
      <Band title="Pair" />
      <div className="pk-body pk-pair">
        <div className="pk-code" data-phase={phase} aria-hidden="true">
          {Array.from({ length: 169 }, (_, i) => <i key={i} data-on={((i * 7 + (i >> 3) * 5) % 3 === 0) || i % 13 === 0} />)}
        </div>
        {phase === "scanned" ? (
          <p className="pk-fact"><Check /> Code scanned <span className="pk-dim">{MAC}</span></p>
        ) : (
          <p className="pk-fact pk-waiting"><LoaderCircle className="pk-spin" aria-hidden="true" /> Waiting for Confirm on your Mac</p>
        )}
      </div>
      <footer className="pk-bar">
        {phase === "scanned" ? (
          <Button variant="primary" className="pk-big" onClick={() => setPhase("waiting")}>Pair</Button>
        ) : (
          <Link to="/" className="pk-big pk-continue">Continue</Link>
        )}
      </footer>
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

function Tabs({ gone, dispatched }: { gone: Set<string>; dispatched: string[] }) {
  const [tab, setTab] = useState<Tab>("Needs you");
  const left = (job: PocketJob) => !gone.has(job.id);
  return (
    <section className="pk-screen" aria-label={tab}>
      <Band title={tab} end={<button className="pk-icon" aria-label="Dispatch" onClick={() => go("/dispatch")}><Plus /></button>} />
      <div className="pk-body" role="tabpanel" aria-label={tab}>
        {tab === "Needs you" && (
          <ul className="pk-list">
            {BLOCKED.filter(left).map((job) => (
              <Row key={job.id} icon={REASON[job.reason!].icon} tip={REASON[job.reason!].says} title={job.title} where={job.repository} age={job.quiet ?? job.age} onOpen={() => go(`/jobs/${job.id}`)} />
            ))}
            {left(APPROVAL) && <Row icon={UserCheck} tip="Approval" title={APPROVAL.title} where={APPROVAL.repository} age={APPROVAL.age} onOpen={() => go(`/jobs/${APPROVAL.id}`)} />}
            {left(REVIEW) && <Row icon={Eye} tip="Review" title={REVIEW.title} where={REVIEW.repository} age={REVIEW.age} onOpen={() => go(`/jobs/${REVIEW.id}`)} />}
            {!gone.has(HOSTED_ASK.id) && <Row icon={SquareTerminal} tip="Session asks" title={HOSTED_ASK.title} where={HOSTED_ASK.repository} age={HOSTED_ASK.age} onOpen={() => go(`/sessions/${HOSTED_ASK.id}`)} />}
            <Row icon={SquareTerminal} tip="Terminal session waiting" title={TERMINAL_WAITING.title} where={TERMINAL_WAITING.repository} age={TERMINAL_WAITING.age} />
          </ul>
        )}
        {tab === "Running" && (
          <ul className="pk-list">
            {dispatched.map((title) => <Row key={title} icon={CircleDot} tip="Running" spin title={title} where={DRAFT_REPO.current} age="now" />)}
            {RUNNING.map((job) => (
              <Row key={job.id} icon={CircleDot} tip="Running" spin title={job.title} where={`${job.repository}  ${job.step!.at}/${job.step!.of} ${job.step!.name}`} age={job.age} />
            ))}
          </ul>
        )}
        {tab === "Done" && (
          <ul className="pk-list">
            {DONE.map((job) => (
              <Row key={job.id} icon={job.status === "completed_success" ? Check : X} tip={job.status === "completed_success" ? "Landed" : "Failed"} title={job.title} where={job.pr === undefined ? job.repository : `${job.repository}  #${job.pr.number}`} age={job.age} />
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

const DRAFT_REPO = { current: REPOSITORIES[0]! };

function Facts({ job }: { job: PocketJob }) {
  const reason = job.reason === undefined ? undefined : REASON[job.reason];
  return (
    <dl className="pk-facts">
      <dt>Repository</dt><dd>{job.repository}</dd>
      <dt>Status</dt><dd>{reason === undefined ? job.status.replace(/_/g, " ") : <><Mark icon={reason.icon} tip={reason.says} /> {reason.says}</>}</dd>
      {job.step !== undefined && <><dt>Step</dt><dd>{job.step.at} of {job.step.of}, {job.step.name}</dd></>}
      <dt>Age</dt><dd>{job.age}</dd>
      {job.quiet !== undefined && <><dt>Quiet</dt><dd>{job.quiet}</dd></>}
      {job.verdict !== undefined && <><dt>Judge</dt><dd>{job.verdict.line}</dd></>}
      {job.checks !== undefined && job.checks.length > 0 && (
        <><dt>Checks</dt><dd><ul className="pk-checks">{job.checks.map((check) => (
          <li key={check.name} data-passed={check.passed}><Mark icon={check.passed ? Check : X} tip={check.passed ? "Passed" : "Failed"} /> {check.name}</li>
        ))}</ul></dd></>
      )}
      {job.pr !== undefined && <><dt>Pull request</dt><dd><a className="pk-link" href={`https://${job.pr.url}`}><GitPullRequest aria-hidden="true" /> #{job.pr.number}</a></dd></>}
      {job.lastAction !== undefined && <><dt>Last action</dt><dd>{job.lastAction}</dd></>}
    </dl>
  );
}

function HoldToKill({ onKilled }: { onKilled: () => void }) {
  return (
    <HoldButton className="pk-big" size="default" askLabel="Kill" description="Kill. Hold until the fill completes, then confirm." onCommit={onKilled} onAsk={onKilled}>
      Hold to kill
    </HoldButton>
  );
}

function JobScreen({ job, back, done }: { job: PocketJob; back: () => void; done: () => void }) {
  const [redirecting, setRedirecting] = useState(false);
  const draftId = `redirect-${job.id}`;
  const acts = ACTS[job.reason ?? "approval"];
  return (
    <section className="pk-screen" aria-label={job.title}>
      <Band title={job.title} onBack={back} back="Back to Needs you" />
      <div className="pk-body"><Facts job={job} /></div>
      {redirecting ? (
        <footer className="pk-bar pk-bar--sheet" role="dialog" aria-label="Redirect">
          <Draft id={draftId} className="pk-dictate" rows={4} placeholder="Redirect" aria-label="Redirect" enterKeyHint="send" autoCapitalize="sentences" autoCorrect="on" spellCheck />
          <div className="pk-pair-acts">
            <Button variant="secondary" className="pk-big" onClick={() =>setRedirecting(false)}>Close</Button>
            <Button variant="primary" className="pk-big" onClick={() =>{ DRAFTS.delete(draftId); done(); }}>Send</Button>
          </div>
        </footer>
      ) : (
        <footer className="pk-bar">
          {acts.has("approve") && <Button variant="primary" className="pk-big" onClick={done}>Approve</Button>}
          {acts.has("redirect") && (
            <Button variant="secondary" className="pk-big" onClick={() =>setRedirecting(true)}>
       Redirect{drafted(draftId) && <span className="pk-dot" role="img" aria-label="Draft kept" />}</Button>
          )}
          {(acts.has("restart") || acts.has("redispatch")) && (
            <div className="pk-pair-acts">
              {acts.has("restart") && <Button variant="secondary" className="pk-big" onClick={done}>Restart step</Button>}
              {acts.has("redispatch") && <Button variant="secondary" className="pk-big" onClick={done}>Redispatch</Button>}
            </div>
          )}
          {acts.has("kill") && <HoldToKill onKilled={done} />}
        </footer>
      )}
    </section>
  );
}

function ReviewScreen({ job, back, done }: { job: PocketJob; back: () => void; done: () => void }) {
  const [asking, setAsking] = useState(false);
  const draftId = `changes-${job.id}`;
  return (
    <section className="pk-screen" aria-label={`Review ${job.title}`}>
      <Band title={job.title} onBack={back} back="Back to Needs you" />
      <div className="pk-body"><Facts job={job} /></div>
      {asking ? (
        <footer className="pk-bar pk-bar--sheet" role="dialog" aria-label="Request changes">
          <Draft id={draftId} className="pk-dictate" rows={4} placeholder="Reason" aria-label="Reason" enterKeyHint="send" autoCapitalize="sentences" autoCorrect="on" spellCheck />
          <div className="pk-pair-acts">
            <Button variant="secondary" className="pk-big" onClick={() =>setAsking(false)}>Close</Button>
            <Button variant="primary" className="pk-big" onClick={() =>{ DRAFTS.delete(draftId); done(); }}>Send</Button>
          </div>
        </footer>
      ) : (
        <footer className="pk-bar">
          <div className="pk-pair-acts">
            <Button variant="secondary" className="pk-big" onClick={() =>setAsking(true)}>Request changes{drafted(draftId) && <span className="pk-dot" role="img" aria-label="Draft kept" />}</Button>
            <Button variant="primary" className="pk-big" onClick={done}>Approve</Button>
          </div>
        </footer>
      )}
    </section>
  );
}

function AskScreen({ back, done }: { back: () => void; done: () => void }) {
  const [picked, setPicked] = useState<string | null>(null);
  const draftId = `ask-${HOSTED_ASK.id}`;
  return (
    <section className="pk-screen" aria-label={HOSTED_ASK.title}>
      <Band title={HOSTED_ASK.title} onBack={back} back="Back to Needs you" />
      <div className="pk-body">
        <p className="pk-question">{HOSTED_ASK.question}</p>
        <div className="pk-choices" role="radiogroup" aria-label="Answer">
          {HOSTED_ASK.options.map((option) => (
            <button key={option} role="radio" aria-checked={picked === option} className="pk-choice" onClick={() => setPicked(option)}>{option}</button>
          ))}
        </div>
        <Draft id={draftId} className="pk-dictate" rows={3} placeholder="Other" aria-label="Other answer" enterKeyHint="send" autoCapitalize="sentences" autoCorrect="on" spellCheck />
      </div>
      <footer className="pk-bar">
        <Button variant="primary" className="pk-big" onClick={() =>{ DRAFTS.delete(draftId); done(); }}>Answer</Button>
      </footer>
    </section>
  );
}

function DispatchScreen({ back, done }: { back: () => void; done: (title: string) => void }) {
  const [repo, setRepo] = useState(REPOSITORIES[0]!);
  const [line, setLine] = useState(() => DRAFTS.get("dispatch") ?? "");
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
          {REPOSITORIES.map((name) => (
            <button key={name} role="radio" aria-checked={repo === name} className="pk-choice" onClick={() => setRepo(name)}>{name}</button>
          ))}
        </div>
      </div>
      <footer className="pk-bar">
        <Button variant="primary" className="pk-big" disabled={line.trim() === ""} onClick={() =>{ DRAFTS.delete("dispatch"); DRAFT_REPO.current = repo; done(line.trim()); }}>Dispatch</Button>
      </footer>
    </section>
  );
}

export function App({ path }: { path: string }) {
  const [gone, setGone] = useState<Set<string>>(new Set());
  const [dispatched, setDispatched] = useState<string[]>([]);
  const home = () => go("/");
  const finish = (id: string) => () => { setGone((was) => new Set(was).add(id)); home(); };
  const job = path.startsWith("/jobs/") ? jobById(path.slice("/jobs/".length)) : undefined;
  return (
    <main className="pk-app" aria-label="Armada">
      {path === "/pair" && <Pair />}
      {path === "/" && <Tabs gone={gone} dispatched={dispatched} />}
      {job !== undefined && (job.status === "awaiting_review"
        ? <ReviewScreen key={job.id} job={job} back={home} done={finish(job.id)} />
        : <JobScreen key={job.id} job={job} back={home} done={finish(job.id)} />)}
      {path === `/sessions/${HOSTED_ASK.id}` && <AskScreen back={home} done={finish(HOSTED_ASK.id)} />}
      {path === "/dispatch" && <DispatchScreen back={home} done={(title) => { setDispatched((was) => [title, ...was]); home(); }} />}
    </main>
  );
}
