// The Sessions walk's window: a rail, the Overview with its Job Board and its
// Sessions list, and one Session open. Mock-only, and **a stand-in for the
// real Overview and Board, not a copy of them**: it draws only what the walk
// looks at, from the same components and tokens the app draws with.
// `packages/screens/src/draft/sessions.ts` holds what Fleet would owe.

import { useEffect, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import {
  Bot,
  Box,
  Check,
  CircleDashed,
  CircleDot,
  CirclePause,
  GitBranch,
  GitPullRequest,
  KeyRound,
  Layers,
  LayoutDashboard,
  Search,
  Send,
  ShieldCheck,
  ShieldEllipsis,
  ShieldX,
  SquareTerminal,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button, Card, Input, Prose, Textarea, Tooltip } from "@armada/components";
import {
  attachmentsOf,
  isBlank,
  ownerOf,
  sessionsMatching,
} from "@armada/screens/src/draft/sessions";
import type { ChipRef, Session, SessionAttachment, SessionChecks, SessionRow } from "@armada/screens/src/draft/sessions";

import "../../styles/index.css";
import "./sessions.css";
import { onTimePassing } from "../time-passes";
import { PLAIN_JOBS, createStore } from "./stage";
import type { Store } from "./stage";

const titleOf = (session: Session) => session.title ?? session.id;

/** A chip's own words and glyph. Its name says the kind, so the figure alone is never bare. */
function chipFor(ref: ChipRef): { Glyph: LucideIcon; text: string; name: string } {
  switch (ref.kind) {
    case "branch":
      return { Glyph: GitBranch, text: ref.name, name: `Branch ${ref.name}` };
    case "pull_request":
      return { Glyph: GitPullRequest, text: `#${ref.number}`, name: `Pull request #${ref.number}` };
    case "slot":
      return { Glyph: KeyRound, text: String(ref.slot), name: `Slot ${ref.slot}` };
    case "job":
      return { Glyph: Box, text: ref.id, name: `Job ${ref.id}` };
  }
}

/** Where a Session is, as a frame hue and one mark; the mark's tooltip says it, never a phrase beside it. */
function frameOf(session: Session): { hue: string; Glyph: LucideIcon; said: string; pulsing: boolean } {
  const failing = attachmentsOf(session, "pull_request").find((pr) => pr.checks.state === "failed");
  if (session.turn.state === "working") {
    const by = session.turn.wokenBy;
    return { hue: "running", Glyph: CircleDot, said: by === undefined ? "Working" : `Woken by ${titleOf(sessionNamed(by))}`, pulsing: true };
  }
  if (isBlank(session)) return { hue: "not-started", Glyph: CircleDashed, said: "Blank: no slot, no branch", pulsing: false };
  if (failing !== undefined) return { hue: "completed-failed", Glyph: ShieldX, said: `Checks failed on #${failing.number}`, pulsing: false };
  return { hue: "awaiting-review", Glyph: CirclePause, said: "Waiting on you", pulsing: false };
}

function sessionNamed(by: { id: string; title: string }): Session {
  return { id: by.id, title: by.title, attachments: [], rows: [], turn: { state: "idle" } };
}

function Mark({ Glyph, said, pulsing = false }: { Glyph: LucideIcon; said: string; pulsing?: boolean }) {
  return (
    <Tooltip label={said}>
      <span className="sx-mark" role="img" aria-label={said} data-pulsing={pulsing || undefined}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * A branch, pull request, slot or Job chip. **Where a Session owns what it
 * names, hovering it draws that Session's card** and pressing it keeps the
 * card up; `own` is the Session the chip is already drawn inside, which needs
 * no card to say who owns it.
 */
function Chip({ chip, sessions, own, onOpen }: { chip: ChipRef; sessions: readonly Session[]; own?: string; onOpen: (id: string) => void }) {
  const { Glyph, text, name } = chipFor(chip);
  const owner = ownerOf(sessions, chip);
  const [hovered, setHovered] = useState(false);
  const [kept, setKept] = useState(false);
  useEffect(() => {
    if (!kept) return;
    const away = (event: MouseEvent) => {
      if (!(event.target instanceof Element) || event.target.closest("[data-owned-chip]") === null) setKept(false);
    };
    const escape = (event: KeyboardEvent) => event.key === "Escape" && setKept(false);
    document.addEventListener("mousedown", away);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", away);
      document.removeEventListener("keydown", escape);
    };
  }, [kept]);

  if (owner === undefined || owner.id === own) {
    return (
      <span className="sx-chip" role="img" aria-label={name}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
        {text}
      </span>
    );
  }
  const open = hovered || kept;
  return (
    <span
      className="sx-owned"
      data-owned-chip
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setHovered(true)}
      onBlur={() => setHovered(false)}
    >
      <button type="button" className="sx-chip" data-owned aria-label={name} aria-expanded={open} onClick={() => setKept((was) => !was)}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
        {text}
      </button>
      {open ? <OwnerCard owner={owner} onOpen={() => onOpen(owner.id)} /> : null}
    </span>
  );
}

/** The Session that owns what was hovered: its title, slot, pull requests, Jobs and last turn. */
function OwnerCard({ owner, onOpen }: { owner: Session; onOpen: () => void }) {
  const slots = attachmentsOf(owner, "slot");
  const prs = attachmentsOf(owner, "pull_request");
  const jobs = attachmentsOf(owner, "job");
  return (
    <div className="sx-card" role="group" aria-label={`Owned by ${titleOf(owner)}`} data-hue={frameOf(owner).hue}>
      <div className="sx-card__band">
        <SquareTerminal size={12} strokeWidth={2} aria-hidden />
        <span className="sx-eyebrow">Session</span>
        <span className="sx-mono">{owner.id}</span>
      </div>
      <div className="sx-card__body">
        <p className="sx-card__title">{titleOf(owner)}</p>
        <ul className="sx-card__facts">
          {slots.map((one) => (
            <li key={one.slot} aria-label={`Slot ${one.slot}`}>
              <KeyRound size={12} strokeWidth={2} aria-hidden />
              <span className="sx-mono">{one.slot}</span>
            </li>
          ))}
          {prs.map((one) => (
            <li key={one.number} aria-label={`Pull request #${one.number}`}>
              <GitPullRequest size={12} strokeWidth={2} aria-hidden />
              <span className="sx-mono">#{one.number}</span>
              <ChecksMark checks={one.checks} />
            </li>
          ))}
          {jobs.map((one) => (
            <li key={one.id} aria-label={`Job ${one.id}`}>
              <Box size={12} strokeWidth={2} aria-hidden />
              <span className="sx-mono">{one.id}</span>
            </li>
          ))}
        </ul>
        {owner.lastTurn === undefined ? null : (
          <Tooltip label="Last turn">
            <span className="sx-mono sx-card__last" aria-label={`Last turn ${owner.lastTurn}`}>
              {owner.lastTurn}
            </span>
          </Tooltip>
        )}
        <Button size="sm" variant="secondary" onClick={onOpen}>
          Open Session
        </Button>
      </div>
    </div>
  );
}

function ChecksMark({ checks }: { checks: SessionChecks }) {
  if (checks.state === "failed") return <Mark Glyph={ShieldX} said={`Checks failed: ${checks.failing}`} />;
  if (checks.state === "pending") return <Mark Glyph={ShieldEllipsis} said="Checks running" />;
  return <Mark Glyph={ShieldCheck} said="Checks passed" />;
}

/** The Board, as far as this walk needs it: every Job, from every Session and from none. */
function JobBoard({ sessions, onOpen }: { sessions: readonly Session[]; onOpen: (id: string) => void }) {
  const jobs = [
    ...sessions.flatMap((one) => attachmentsOf(one, "job")),
    ...PLAIN_JOBS,
  ];
  return (
    <section className="sx-section" aria-label="Job Board">
      <h2 className="sx-eyebrow">Job Board</h2>
      <ul className="sx-list">
        {jobs.map((job) => (
          <li key={job.id} className="sx-row" aria-label={`Job ${job.id}, ${job.title}`}>
            <span className="sx-row__title">{job.title}</span>
            <span className="sx-row__chips">
              <Chip chip={{ kind: "job", id: job.id }} sessions={sessions} onOpen={onOpen} />
              <Chip chip={{ kind: "branch", name: job.branch }} sessions={sessions} onOpen={onOpen} />
              <Chip chip={{ kind: "slot", slot: job.slot }} sessions={sessions} onOpen={onOpen} />
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function SessionsList({ stage, store }: { stage: ReturnType<Store["get"]>; store: Store }) {
  const hits = sessionsMatching(stage.sessions, stage.query);
  return (
    <section className="sx-section" aria-label="Sessions">
      <div className="sx-section__head">
        <h2 className="sx-eyebrow">Sessions</h2>
        <Button size="sm" variant="secondary" onClick={store.newSession}>
          New Session
        </Button>
      </div>
      <Input
        type="search"
        aria-label="Search Sessions"
        mono
        value={stage.query}
        onChange={(event) => store.search(event.target.value)}
        trailing={<Search size={12} strokeWidth={2} aria-hidden />}
      />
      <ul className="sx-list">
        {hits.map(({ session, matched }) => {
          const frame = frameOf(session);
          return (
            <li key={session.id} className="sx-row" data-hue={frame.hue} aria-label={titleOf(session)}>
              <button type="button" className="sx-row__open" onClick={() => store.open(session.id)}>
                <Mark Glyph={frame.Glyph} said={frame.said} pulsing={frame.pulsing} />
                <span className="sx-row__title">{session.title ?? <span className="sx-mono">{session.id}</span>}</span>
              </button>
              <span className="sx-row__chips">
                {matched !== "title" && matched !== "id" ? <Matched attachment={matched} /> : null}
                {session.lastTurn === undefined ? null : (
                  <Tooltip label="Last turn">
                    <span className="sx-mono sx-row__last" aria-label={`Last turn ${session.lastTurn}`}>
                      {session.lastTurn}
                    </span>
                  </Tooltip>
                )}
              </span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** What a search found in a Session, drawn as the chip it is. */
function Matched({ attachment }: { attachment: SessionAttachment }) {
  const ref: ChipRef | undefined =
    attachment.kind === "pull_request"
      ? { kind: "pull_request", number: attachment.number }
      : attachment.kind === "branch"
        ? { kind: "branch", name: attachment.name }
        : attachment.kind === "slot"
          ? { kind: "slot", slot: attachment.slot }
          : attachment.kind === "job"
            ? { kind: "job", id: attachment.id }
            : undefined;
  if (ref === undefined) return null;
  const { Glyph, text, name } = chipFor(ref);
  return (
    <span className="sx-chip" data-matched role="img" aria-label={`Matched ${name}`}>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      {text}
    </span>
  );
}

function ThreadRow({ row }: { row: SessionRow }) {
  if (row.kind === "lease") {
    return (
      <li className="sx-lease" role="region" aria-label="Leased on first write">
        <span className="sx-eyebrow">First write</span>
        <span className="sx-lease__facts">
          <span className="sx-chip">
            <KeyRound size={12} strokeWidth={2} aria-hidden />
            {row.slot}
          </span>
          <span className="sx-chip">
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            {row.branch}
          </span>
        </span>
        <span className="sx-mono sx-at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "tool") {
    return (
      <li className="armada-helm-thread__row" data-actor="helm">
        <p className="armada-helm-thread__message" data-mono="true">
          {row.text}
        </p>
      </li>
    );
  }
  const session = row.from.kind === "session" ? row.from : undefined;
  return (
    <li
      className="armada-helm-thread__row"
      data-actor={row.from.kind === "you" ? "you" : "helm"}
      data-from={row.from.kind}
      {...(session === undefined ? {} : { role: "region", "aria-label": `Message from ${session.title}` })}
    >
      <div className="armada-helm-thread__head">
        <span className="armada-helm-thread__who">
          {row.from.kind === "you" ? "You" : session === undefined ? "Agent" : `${session.id} · ${session.title}`}
        </span>
        <span className="armada-helm-thread__at">{row.at}</span>
      </div>
      <div className="armada-helm-thread__message">
        <Prose text={row.text} />
      </div>
    </li>
  );
}

function Ledger({ session, onOpen }: { session: Session; onOpen: (one: SessionAttachment) => void }) {
  const group = (label: string, kind: SessionAttachment["kind"], entries: { key: string; name: string; Glyph: LucideIcon; text: ReactNode; trailing?: ReactNode; one: SessionAttachment }[]) =>
    entries.length === 0 ? null : (
      <div className="sx-group" key={kind}>
        <h3 className="sx-eyebrow">{label}</h3>
        <ul className="sx-list">
          {entries.map((entry) => (
            <li key={entry.key} className="sx-row sx-row--ledger" aria-label={entry.name} data-new>
              <button type="button" className="sx-row__open" aria-label={`Open ${entry.name}`} onClick={() => onOpen(entry.one)}>
                <entry.Glyph size={12} strokeWidth={2} aria-hidden />
                <span className="sx-row__title">{entry.text}</span>
              </button>
              {entry.trailing}
            </li>
          ))}
        </ul>
      </div>
    );
  return (
    <aside className="sx-ledger" role="region" aria-label="Attachments">
      {group("Slots", "slot", attachmentsOf(session, "slot").map((one) => ({ key: `slot${one.slot}`, name: `Slot ${one.slot}`, Glyph: KeyRound, text: <span className="sx-mono">{one.slot}</span>, one })))}
      {group("Branches", "branch", attachmentsOf(session, "branch").map((one) => ({ key: one.name, name: `Branch ${one.name}`, Glyph: GitBranch, text: <span className="sx-mono">{one.name}</span>, one })))}
      {group(
        "Pull requests",
        "pull_request",
        attachmentsOf(session, "pull_request").map((one) => ({
          key: String(one.number),
          name: `Pull request #${one.number}, checks ${one.checks.state}`,
          Glyph: GitPullRequest,
          text: (
            <>
              <span className="sx-mono">#{one.number}</span> {one.title}
            </>
          ),
          trailing: <ChecksMark checks={one.checks} />,
          one,
        })),
      )}
      {group(
        "Jobs",
        "job",
        attachmentsOf(session, "job").map((one) => ({
          key: one.id,
          name: `Job ${one.id}`,
          Glyph: Box,
          text: (
            <>
              <span className="sx-mono">{one.id}</span> {one.title}
            </>
          ),
          trailing: (
            <span className="sx-row__chips">
              <span className="sx-chip" role="img" aria-label={`Slot ${one.slot}`}>
                <KeyRound size={12} strokeWidth={2} aria-hidden />
                {one.slot}
              </span>
            </span>
          ),
          one,
        })),
      )}
      {group("Studios", "studio", attachmentsOf(session, "studio").map((one) => ({ key: one.id, name: `Studio ${one.title}`, Glyph: Layers, text: one.title, one })))}
      {group(
        "Subagents",
        "subagent",
        attachmentsOf(session, "subagent").map((one) => ({
          key: one.id,
          name: `Subagent ${one.task}, ${one.state}`,
          Glyph: Bot,
          text: one.task,
          trailing: <Mark Glyph={one.state === "running" ? CircleDot : Check} said={one.state === "running" ? "Running" : "Done"} pulsing={one.state === "running"} />,
          one,
        })),
      )}
    </aside>
  );
}

/** Each ledger row opens its own surface; here that is a named stand-in, since those surfaces already exist in Bridge. */
function Surface({ one, onClose }: { one: SessionAttachment; onClose: () => void }) {
  const named: Record<SessionAttachment["kind"], string> = {
    slot: "Slot on Cleanup",
    branch: "Branch",
    pull_request: "Pull request",
    job: "Job detail",
    studio: "Studio",
    subagent: "Subagent",
  };
  const name =
    one.kind === "slot" ? `Slot ${one.slot}` : one.kind === "branch" ? one.name : one.kind === "pull_request" ? `#${one.number}` : one.kind === "job" ? `Job ${one.id}` : one.kind === "studio" ? one.title : one.task;
  return (
    <div className="sx-surface" role="dialog" aria-label={name}>
      <div className="sx-card__band">
        <span className="sx-eyebrow">{named[one.kind]}</span>
        <Button size="sm" variant="ghost" aria-label="Close" onClick={onClose}>
          <X size={12} strokeWidth={2} aria-hidden />
        </Button>
      </div>
      <p className="sx-card__title">{name}</p>
    </div>
  );
}

function SessionView({ session, stage, store }: { session: Session; stage: ReturnType<Store["get"]>; store: Store }) {
  const frame = frameOf(session);
  const [text, setText] = useState("");
  const [opened, setOpened] = useState<SessionAttachment | undefined>();
  const waiting = session.turn.state === "working";
  const asked = session.id === stage.mine ? stage.asked : undefined;
  return (
    <section className="sx-frame" data-hue={frame.hue} aria-label={`Session ${session.id}`}>
      <header className="sx-band">
        <SquareTerminal size={16} strokeWidth={2} aria-hidden />
        <span className="sx-eyebrow">Session</span>
        <span className="sx-mono sx-band__id">{session.id}</span>
        <span className="sx-band__title">{session.title}</span>
        <Mark Glyph={frame.Glyph} said={frame.said} pulsing={frame.pulsing} />
      </header>
      <div className="sx-body">
        <div className="sx-centre">
          <div className="sx-thread" role="region" aria-label="Thread">
            <ol className="armada-helm-thread__rows">
              {session.rows.map((row) => (
                <ThreadRow key={row.id} row={row} />
              ))}
            </ol>
          </div>
          {asked === undefined ? null : (
            <Card flat className="sx-ask" role="article" aria-label="Waiting on you">
              <span className="sx-eyebrow">Permission</span>
              <p className="sx-mono sx-ask__command">{asked.command}</p>
              <div className="sx-ask__answers" role="group" aria-label="Answers">
                <Button size="sm" variant="secondary" onClick={store.answer}>
                  Allow once
                </Button>
                <Button size="sm" variant="secondary" onClick={store.answer}>
                  Deny
                </Button>
              </div>
            </Card>
          )}
          <form
            className="sx-composer"
            onSubmit={(event) => {
              event.preventDefault();
              store.send(text);
              setText("");
            }}
          >
            <Textarea aria-label="Message" rows={2} value={text} onChange={(event) => setText(event.target.value)} />
            <Button type="submit" variant="primary" size="sm" disabled={waiting || text.trim() === ""}>
              <Send size={12} strokeWidth={2} aria-hidden />
              Send
            </Button>
          </form>
        </div>
        <Ledger session={session} onOpen={setOpened} />
        {opened === undefined ? null : <Surface one={opened} onClose={() => setOpened(undefined)} />}
      </div>
    </section>
  );
}

export function SessionsStage() {
  const [store] = useState(createStore);
  const stage = useSyncExternalStore(store.subscribe, store.get);
  useEffect(() => {
    onTimePassing(store.later);
    return () => {
      onTimePassing(undefined);
      store.dispose();
    };
  }, [store]);
  const current = stage.view.at === "session" ? stage.sessions.find((one) => one.id === (stage.view as { id: string }).id) : undefined;
  return (
    <div className="sx-window">
      <nav className="sx-rail" aria-label="Bridge">
        <button type="button" className="sx-rail__item" aria-current={current === undefined ? "page" : undefined} onClick={store.overview}>
          <LayoutDashboard size={16} strokeWidth={2} aria-hidden />
          Overview
        </button>
      </nav>
      <main className="sx-content">
        {current !== undefined ? (
          <SessionView session={current} stage={stage} store={store} />
        ) : (
          <div className="sx-overview">
            <JobBoard sessions={stage.sessions} onOpen={store.open} />
            <SessionsList stage={stage} store={store} />
          </div>
        )}
      </main>
    </div>
  );
}
