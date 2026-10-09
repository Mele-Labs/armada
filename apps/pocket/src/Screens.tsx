// The phone app's screens (#1997): a plain responsive page, fixture data only.
// Everything it reads comes through `./data`, from the Gateway.

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  Check, ChevronLeft, CircleDot, Eye, GitPullRequest, LoaderCircle, OctagonAlert, RefreshCw,
  Split, Unplug, UserCheck, X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button, HoldButton } from "@armada/components";

import { Refused, lastRefusal } from "./client";
import { startLive, useJob, useJobs } from "./data";
import type { PocketJob } from "./data";
import { paired } from "./device";
import { claim, codeFromUrl, defaultName, waitForConfirm } from "./pair";
import { subscribe } from "./push";
import { go } from "./router";

type Tab = "Needs you" | "Running" | "Done";
const TABS: Tab[] = ["Needs you", "Running", "Done"];

/** Glyphs and verbs are `crates/core-model/domain/enum-verbs.toml`'s. */
const REASON: Record<string, { icon?: LucideIcon; says: string }> = {
  stalled: { icon: OctagonAlert, says: "Stalled" },
  thrashing: { icon: RefreshCw, says: "Churning" },
  rate_cap: { icon: Split, says: "Hit the sub-dispatch cap" },
  interrupted: { icon: Unplug, says: "Interrupted" },
};

const reasonOf = (reason: string): { icon?: LucideIcon; says: string } => REASON[reason] ?? { says: reason.replace(/_/g, " ") };

type Act = "approve" | "redirect" | "restart" | "redispatch" | "kill";
/** What a state allows. `approve_dispatch` means something only on a Job awaiting dispatch approval. */
const ACTS: Record<string, Set<Act>> = {
  approval: new Set(["approve"]),
  stalled: new Set(["redirect", "restart", "redispatch", "kill"]),
  thrashing: new Set(["redirect", "restart", "redispatch", "kill"]),
  rate_cap: new Set(["restart", "kill"]),
  interrupted: new Set(["restart", "redispatch", "kill"]),
};
const actsOf = (job: PocketJob): Set<Act> => ACTS[job.reason ?? "approval"] ?? ACTS.stalled!;

// TODO(#2001): the acts below do nothing yet; their Gateway routes (approve, redirect,
// restart_step, kill, redispatch, approve_review, request_changes) come with #2001.

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
  const [name, setName] = useState(() => defaultName(navigator.userAgent));
  const [trouble, setTrouble] = useState(lastRefusal());
  const code = codeFromUrl(window.location.search);
  const pair = async () => {
    setTrouble("");
    try {
      await claim(name);
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

function needRow(job: PocketJob) {
  const approval = job.status === "awaiting_approval";
  const review = job.status === "awaiting_review";
  const shown = job.reason !== undefined ? reasonOf(job.reason) : approval ? { icon: UserCheck, says: "Approval" } : review ? { icon: Eye, says: "Review" } : { says: "Asking" };
  return <Row key={job.id} icon={shown.icon} tip={shown.says} title={job.title} where={job.repository} age={job.quiet ?? job.age} onOpen={() => go(`/jobs/${job.id}`)} />;
}

function Tabs({ gone }: { gone: Set<string> }) {
  const [tab, setTab] = useState<Tab>("Needs you");
  const { needs, running, done, error } = useJobs();
  const left = (job: PocketJob) => !gone.has(job.id);
  const waiting = needs.filter(left);
  return (
    <section className="pk-screen" aria-label={tab}>
      <Band title={tab} />
      <div className="pk-body" role="tabpanel" aria-label={tab}>
        {error !== undefined && <p className="pk-fact"><Unplug aria-hidden="true" /> {error}</p>}
        {tab === "Needs you" && (
          <ul className="pk-list">
            {waiting.map(needRow)}
          </ul>
        )}
        {tab === "Running" && (
          <ul className="pk-list">
            {running.map((job) => (
              <Row key={job.id} icon={CircleDot} tip="Running" spin title={job.title} where={job.step === undefined ? job.repository : `${job.repository}  ${job.step.at}/${job.step.of} ${job.step.name}`} age={job.age} />
            ))}
          </ul>
        )}
        {tab === "Done" && (
          <ul className="pk-list">
            {done.map((job) => (
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

function Facts({ job }: { job: PocketJob }) {
  const reason = job.reason === undefined ? undefined : reasonOf(job.reason);
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

/** The acts reach Fleet with #2001. Until then none is drawn: a press that changed
 * nothing on the Mac would read as done. */
const ACTS_REACH_FLEET = false;

function JobScreen({ job, back, done }: { job: PocketJob; back: () => void; done: () => void }) {
  const [redirecting, setRedirecting] = useState(false);
  const draftId = `redirect-${job.id}`;
  const acts = actsOf(job);
  return (
    <section className="pk-screen" aria-label={job.title}>
      <Band title={job.title} onBack={back} back="Back to Needs you" />
      <div className="pk-body"><Facts job={job} /></div>
      {!ACTS_REACH_FLEET ? null : redirecting ? (
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
      {!ACTS_REACH_FLEET ? null : asking ? (
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

export function App({ path }: { path: string }) {
  const [gone, setGone] = useState<Set<string>>(new Set());
  const home = () => go("/");
  const finish = (id: string) => () => { setGone((was) => new Set(was).add(id)); home(); };
  const job = useJob(path.startsWith("/jobs/") ? path.slice("/jobs/".length) : undefined);
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
      {path === "/" && <Tabs gone={gone} />}
      {job !== undefined && (job.status === "awaiting_review"
        ? <ReviewScreen key={job.id} job={job} back={home} done={finish(job.id)} />
        : <JobScreen key={job.id} job={job} back={home} done={finish(job.id)} />)}
    </main>
  );
}
