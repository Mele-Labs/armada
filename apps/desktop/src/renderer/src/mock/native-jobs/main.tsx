// The native Jobs screens as an HTML mock, on the real tokens, for the owner's walk before the gpui
// build. Everything here is a mock: no Fleet, fixtures from ./data. Two takes of the list row,
// `?take=two` (the default) and `?take=one`.
import "@armada/tokens/tokens.css";
import "@armada/tokens/base.css";
import "@armada/tokens/takes.css";
import "./jobs.css";

import { StrictMode, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import {
  ChevronDown,
  ChevronRight,
  CircleCheck,
  CircleDashed,
  CircleDot,
  CirclePause,
  Footprints,
  GitFork,
  GitMerge,
  GitPullRequest,
  Link2,
  MessageCircleQuestion,
  Mic,
  NotebookPen,
  OctagonAlert,
  Paperclip,
  PencilLine,
  PencilRuler,
  Play,
  Power,
  PowerOff,
  Radar,
  ScanSearch,
  Search,
  Send,
  ShieldCheck,
  ShieldX,
  Split,
  Stamp,
  Ticket,
  X,
  type LucideIcon,
} from "lucide-react";
import waitingSound from "../../../../../sounds/armada-waiting.wav?url";
import blockedSound from "../../../../../sounds/armada-blocked.wav?url";
import { ENDED, JOBS, SLASH, TAGS, threadOf, type Item, type Job, type Stretch, type Thread } from "./data";

document.documentElement.dataset.look = "new";
document.documentElement.dataset.theme = "dark";

const TAKE = new URLSearchParams(location.search).get("take") === "one" ? "one" : "two";
const KEPT = "native-jobs-mock";

type Place = { tab: string; scroll: number; open: string[] };
type Kept = { job: string; places: Record<string, Place>; drafts: Record<string, string> };

const readKept = (): Kept => {
  try {
    const kept = JSON.parse(localStorage.getItem(KEPT) ?? "");
    if (kept && typeof kept.job === "string") return kept;
  } catch {
    // nothing kept yet
  }
  return { job: "20b4e00a", places: {}, drafts: {} };
};

const needs = (job: Job) => job.waiting_for.length > 0;

const ago = (minutes: number) =>
  minutes < 1 ? "now" : minutes < 60 ? `${minutes}m` : minutes < 1440 ? `${Math.floor(minutes / 60)}h` : `${Math.floor(minutes / 1440)}d`;

// ── A glyph with its tooltip: every state is one of these, never a phrase.
const Glyph = ({ icon: Icon, tip, tone }: { icon: LucideIcon; tip: string; tone?: string }) => (
  <span className="nj-glyph" data-tip={tip} data-tone={tone} role="img" aria-label={tip}>
    <Icon size={14} strokeWidth={1.75} aria-hidden />
  </span>
);

const stateGlyph = (job: Job): ReactNode => {
  if (needs(job)) return <Glyph icon={MessageCircleQuestion} tip="Needs you" tone="ask" />;
  if (job.turn === "paused") return <Glyph icon={CirclePause} tip="Paused" />;
  if (job.turn === "stopped") return <Glyph icon={OctagonAlert} tip="Stopped" tone="bad" />;
  if (job.turn === "working" && job.phase === "read") return <Glyph icon={ScanSearch} tip="Reading" tone="live" />;
  if (job.turn === "working" && job.phase === "write") return <Glyph icon={PencilLine} tip="Writing" tone="live" />;
  if (job.turn === "working") return <Glyph icon={Radar} tip="Running a command" tone="live" />;
  return null;
};

const lampOf = (job: Job) =>
  needs(job) ? "needs" : job.turn === "stopped" ? "failed" : job.turn === "working" ? "running" : "idle";

const prOf = (job: Job) => job.in_flight.find((flight) => flight.lifecycle === "pull_request");

// ── The Jobs list: grouped by project, Needs you first in each group. Ended Jobs are not here.
function JobsList({
  jobs,
  selected,
  onOpen,
  listRef,
}: {
  jobs: Job[];
  selected: string;
  onOpen: (id: string) => void;
  listRef: React.RefObject<HTMLDivElement | null>;
}) {
  const groups = useMemo(() => {
    const byProject = new Map<string, Job[]>();
    for (const job of jobs) byProject.set(job.manifest_id, [...(byProject.get(job.manifest_id) ?? []), job]);
    return [...byProject].map(([project, rows]) => ({
      project,
      rows: [...rows].sort((a, b) => Number(needs(b)) - Number(needs(a)) || a.last_turn_at - b.last_turn_at),
    }));
  }, [jobs]);
  const order = groups.flatMap((group) => group.rows.map((row) => row.id));

  const onKey = (event: React.KeyboardEvent) => {
    const at = order.indexOf(selected);
    if (event.key === "ArrowDown" || event.key === "j") {
      event.preventDefault();
      onOpen(order[Math.min(order.length - 1, at + 1)]);
    } else if (event.key === "ArrowUp" || event.key === "k") {
      event.preventDefault();
      onOpen(order[Math.max(0, at - 1)]);
    }
  };

  return (
    <section className="nj-panel nj-list" aria-label="Jobs">
      <header className="nj-legend">
        <span className="nj-legend__label">Jobs</span>
      </header>
      <div className="nj-list__rows" role="listbox" aria-label="Live Jobs" tabIndex={0} ref={listRef} onKeyDown={onKey}
        aria-activedescendant={`row-${selected}`}>
        {groups.map((group) => (
          <div key={group.project} role="group" aria-label={group.project} className="nj-group">
            <div className="nj-group__head">{group.project}</div>
            {group.rows.map((job) => {
              const pr = prOf(job);
              const said = needs(job) ? job.waiting_for[0].text : job.open_call?.text;
              return (
                <div
                  key={job.id}
                  id={`row-${job.id}`}
                  role="option"
                  aria-selected={job.id === selected}
                  aria-label={job.title}
                  className="nj-row"
                  data-lamp={lampOf(job)}
                  data-needs={needs(job) || undefined}
                  onClick={() => onOpen(job.id)}
                >
                  <span className="nj-lamp" aria-hidden />
                  <span className="nj-row__title">{job.title}</span>
                  <span className="nj-col">{stateGlyph(job)}</span>
                  <span className="nj-col">
                    {pr ? <Glyph icon={GitPullRequest} tip={`#${pr.key} ${pr.state}`} /> : null}
                  </span>
                  <span className="nj-row__at" aria-label={`Last turn ${ago(job.last_turn_at)}`}>
                    {ago(job.last_turn_at)}
                  </span>
                  {(TAKE === "two" || needs(job)) && said ? <span className="nj-row__said">{said}</span> : null}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </section>
  );
}

// ── The plan's progress line: each group of the plan a stretch, closed by its gate.
const GATE: Record<Stretch["gate"], { icon: LucideIcon; tip: string }> = {
  plan_approved: { icon: Stamp, tip: "Plan approved" },
  walk_approved: { icon: Footprints, tip: "Walk approved" },
  checks_green: { icon: ShieldCheck, tip: "Checks green" },
  merged: { icon: GitMerge, tip: "Merged" },
};
const STRETCH: Record<Stretch["state"], { icon: LucideIcon; tip: string }> = {
  done: { icon: CircleCheck, tip: "Done" },
  now: { icon: CircleDot, tip: "Now" },
  next: { icon: CircleDashed, tip: "Next" },
};

function Progress({ stretches }: { stretches: Stretch[] }) {
  return (
    <ol className="nj-progress" aria-label="Plan">
      {stretches.map((stretch) => (
        <li key={stretch.title} className="nj-stretch" data-state={stretch.state} aria-label={`${stretch.title}, ${STRETCH[stretch.state].tip}`}>
          <span className="nj-stretch__bar" aria-hidden />
          <span className="nj-stretch__label">
            <Glyph icon={STRETCH[stretch.state].icon} tip={STRETCH[stretch.state].tip} tone={stretch.state === "now" ? "live" : undefined} />
            <span className="nj-stretch__title">{stretch.title}</span>
            <Glyph icon={GATE[stretch.gate].icon} tip={GATE[stretch.gate].tip} />
          </span>
        </li>
      ))}
    </ol>
  );
}

// ── The Job's head: what it is, its marks, its acts.
function JobHead({ job, onPause, onEnd, onReview }: { job: Job; onPause: () => void; onEnd: () => void; onReview: () => void }) {
  return (
    <header className="nj-head">
      <div className="nj-head__line">
        <span className="nj-lamp" data-lamp={lampOf(job)} aria-hidden />
        <h1 className="nj-head__title">{job.title}</h1>
        <span className="nj-head__where">
          {job.manifest_id} · {job.branch}
          {job.slot ? ` · slot ${job.slot}` : ""}
        </span>
        <span className="nj-head__acts" role="toolbar" aria-label="Job">
          {needs(job) ? (
            <button className="nj-icon" data-tip="Needs you ⇧⌘Y" aria-label="Needs you" onClick={onReview} data-tone="ask">
              <MessageCircleQuestion size={14} aria-hidden />
            </button>
          ) : null}
          <button className="nj-icon" data-tip="Fork ⌥⌘F" aria-label="Fork">
            <GitFork size={14} aria-hidden />
          </button>
          <button
            className="nj-icon"
            data-tip={job.turn === "paused" ? "Resume ⌘P" : "Pause ⌘P"}
            aria-label={job.turn === "paused" ? "Resume" : "Pause"}
            onClick={onPause}
          >
            {job.turn === "paused" ? <Play size={14} aria-hidden /> : <CirclePause size={14} aria-hidden />}
          </button>
          <button className="nj-icon" data-tip="End ⌘⌫" aria-label="End" onClick={onEnd}>
            <PowerOff size={14} aria-hidden />
          </button>
        </span>
      </div>
      {job.handed_from || job.forked_from || job.in_flight.length ? (
        <div className="nj-head__marks">
          {job.handed_from ? (
            <span className="nj-mark" aria-label={`Handed on from ${job.handed_from}`}>
              <Glyph icon={Link2} tip="Handed on from" /> {job.handed_from}
            </span>
          ) : null}
          {job.forked_from ? (
            <span className="nj-mark" aria-label={`Forked from ${job.forked_from}`}>
              <Glyph icon={GitFork} tip="Forked from" /> {job.forked_from}
            </span>
          ) : null}
          {job.in_flight.map((flight) => (
            <span key={flight.key} className="nj-mark" aria-label={`${flight.label}, ${flight.state}`}>
              <Glyph
                icon={flight.lifecycle === "pull_request" ? GitPullRequest : flight.lifecycle === "ticket" ? Ticket : Link2}
                tip={flight.state}
              />
              {flight.lifecycle === "pull_request" ? `#${flight.key}` : flight.lifecycle === "ticket" ? flight.key : flight.label}
            </span>
          ))}
        </div>
      ) : null}
      {job.stretches ? <Progress stretches={job.stretches} /> : null}
    </header>
  );
}

// ── The thread.
function Calls({ item, open, onToggle }: { item: Extract<Item, { kind: "calls" }>; open: boolean; onToggle: () => void }) {
  const last = item.rows[item.rows.length - 1];
  return (
    <div className="nj-calls" data-open={open || undefined}>
      <button className="nj-calls__head" aria-expanded={open} aria-label={`Tool calls: ${item.names.join(", ")}`} onClick={onToggle}>
        {open ? <ChevronDown size={14} aria-hidden /> : <ChevronRight size={14} aria-hidden />}
        <span className="nj-calls__names">{item.names.join(" · ")}</span>
        {!open ? <span className="nj-calls__peek">{last.text.slice(last.text.indexOf(" ") + 1)}</span> : null}
      </button>
      {open ? (
        <ol className="nj-calls__rows">
          {item.rows.map((row) => (
            <li key={row.id}>
              <span className="nj-calls__tool">{row.text.split(" ")[0]}</span> {row.text.slice(row.text.indexOf(" ") + 1)}
            </li>
          ))}
        </ol>
      ) : null}
    </div>
  );
}

function Card({ icon, tip, title, children, acts }: { icon: LucideIcon; tip: string; title: string; children?: ReactNode; acts?: ReactNode }) {
  return (
    <article className="nj-card" aria-label={`${tip}: ${title}`}>
      <header className="nj-card__head">
        <Glyph icon={icon} tip={tip} />
        <span className="nj-card__title">{title}</span>
        {acts ? <span className="nj-card__acts">{acts}</span> : null}
      </header>
      {children ? <div className="nj-card__body">{children}</div> : null}
    </article>
  );
}

function ThreadItem({ item, open, onToggle, onReview }: { item: Item; open: boolean; onToggle: () => void; onReview: () => void }) {
  switch (item.kind) {
    case "calls":
      return <Calls item={item} open={open} onToggle={onToggle} />;
    case "handoff":
      return (
        <div className="nj-divider" role="note">
          <Glyph icon={Link2} tip="Handed on from" /> {item.from}
        </div>
      );
    case "change":
      return (
        <div className="nj-divider" role="note">
          {item.what} {item.from} → {item.to} <span className="nj-at">{item.at}</span>
        </div>
      );
    case "subagent":
      return (
        <div className="nj-line" role="note" aria-label={`Subagent ${item.subagent.description}`}>
          <Glyph icon={Split} tip="Subagent" />
          <span>{item.subagent.description}</span>
          <span className="nj-mono-subtle">
            {item.subagent.agent} · {item.subagent.model}
          </span>
          <span className="nj-line__end">
            {item.subagent.state === "running" ? <Glyph icon={CircleDot} tip="Running" tone="live" /> : <Glyph icon={CircleCheck} tip="Finished" />}
          </span>
        </div>
      );
    case "checks":
      return (
        <div className="nj-line" role="note" aria-label={`Check ${item.name}`}>
          {item.rows[item.rows.length - 1].state === "passed" ? (
            <Glyph icon={ShieldCheck} tip="Passed" tone="good" />
          ) : (
            <Glyph icon={ShieldX} tip="Failed" tone="bad" />
          )}
          <span className="nj-mono">{item.name}</span>
          <span className="nj-line__runs">
            {item.rows.map((run, n) => (
              <span key={n} className="nj-run" data-state={run.state} data-tip={`${run.state} ${run.at}`} role="img" aria-label={`${run.state} ${run.at}`} />
            ))}
          </span>
        </div>
      );
    case "fleet_turn":
      return (
        <div className="nj-fleet" role="note" aria-label="Fleet">
          <span className="nj-fleet__who">Fleet</span>
          <span className="nj-fleet__notice">{item.notice}</span>
        </div>
      );
    case "sketch":
      return <Card icon={PencilRuler} tip="Sketch" title={item.title} acts={<button className="nj-button">Open</button>} />;
    case "document":
      return <Card icon={NotebookPen} tip="Document" title={item.title} acts={<button className="nj-button">Open</button>} />;
    case "row": {
      const row = item.row;
      if (row.kind === "lease")
        return (
          <div className="nj-divider" role="note">
            slot {row.slot} · {row.branch} <span className="nj-at">{row.at}</span>
          </div>
        );
      if (row.kind === "plan")
        return (
          <Card
            icon={Stamp}
            tip="Plan"
            title={row.title}
            acts={
              row.asked ? (
                <>
                  <button className="nj-button" onClick={onReview}>Send back</button>
                  <button className="nj-button" data-primary onClick={onReview}>Approve</button>
                </>
              ) : null
            }
          >
            <ol className="nj-plan">
              {row.groups.map((group) => (
                <li key={group.title}>
                  <span className="nj-plan__title">{group.title}</span>
                  <span className="nj-plan__gives">{group.gives}</span>
                </li>
              ))}
            </ol>
          </Card>
        );
      if (row.kind === "draft")
        return (
          <Card
            icon={PencilLine}
            tip="Draft"
            title={row.title}
            acts={
              row.asked ? (
                <>
                  <button className="nj-button" onClick={onReview}>Send back</button>
                  <button className="nj-button" data-primary onClick={onReview}>Use it</button>
                </>
              ) : null
            }
          >
            <p className="nj-draft">{row.text}</p>
          </Card>
        );
      return (
        <div className="nj-message" data-from={row.from} role="article" aria-label={row.from === "person" ? "You" : "Agent"}>
          <p>{row.text}</p>
          {row.tags?.length ? (
            <div className="nj-message__tags">
              {row.tags.map((tag) => (
                <span key={tag} className="nj-tag">{tag}</span>
              ))}
            </div>
          ) : null}
          <span className="nj-at">{row.at}</span>
        </div>
      );
    }
  }
}

function Asked({ thread }: { thread: Thread }) {
  const [picked, setPicked] = useState(0);
  if (!thread.asked) return null;
  const asked = thread.asked;
  return (
    <section className="nj-ask" aria-label={asked.question}>
      <header className="nj-ask__head">
        <Glyph icon={MessageCircleQuestion} tip="Question" tone="ask" />
        <span className="nj-ask__question">{asked.question}</span>
      </header>
      <div className="nj-ask__options" role="radiogroup" aria-label={asked.header}>
        {asked.options.map((option, n) => (
          <button
            key={option.label}
            role="radio"
            aria-checked={picked === n}
            className="nj-ask__option"
            onClick={() => setPicked(n)}
          >
            <kbd className="nj-kbd">{n + 1}</kbd>
            <span className="nj-ask__label">{option.label}</span>
            <span className="nj-ask__said">{option.description}</span>
          </button>
        ))}
      </div>
      <footer className="nj-ask__foot">
        <button className="nj-button" data-primary>Answer <kbd className="nj-kbd">⌥↵</kbd></button>
      </footer>
    </section>
  );
}

// ── The composer: the draft is kept per Job; # and / open their menus; attachments sit above.
function Composer({ job, draft, setDraft, focusRef }: { job: Job; draft: string; setDraft: (text: string) => void; focusRef: React.RefObject<HTMLTextAreaElement | null> }) {
  const [files, setFiles] = useState<string[]>(job.id === "20b4e00a" ? ["jobs-list.png"] : []);
  const [listening, setListening] = useState(false);
  const [menuAt, setMenuAt] = useState(0);
  const word = draft.split(/\s/).pop() ?? "";
  const menu =
    /^\/\S*$/.test(draft) ? SLASH.filter((one) => one.name.startsWith(draft)).map((one) => ({ name: one.name, says: one.says }))
    : word.startsWith("#") ? TAGS.filter((tag) => tag.name.includes(word.slice(1)) || tag.says.toLowerCase().includes(word.slice(1).toLowerCase()))
    : [];

  const take = (name: string) => {
    setDraft(draft.startsWith("/") ? `${name} ` : `${draft.slice(0, draft.length - word.length)}${name} `);
    setMenuAt(0);
  };

  return (
    <section className="nj-composer" aria-label="Message the Job">
      {menu.length ? (
        <ul className="nj-menu" role="listbox" aria-label={draft.startsWith("/") ? "Commands" : "Tags"}>
          {menu.map((one, n) => (
            <li key={one.name} role="option" aria-selected={n === menuAt} onMouseDown={(event) => { event.preventDefault(); take(one.name); }}>
              <span className="nj-mono">{one.name}</span>
              <span className="nj-menu__says">{one.says}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {files.length ? (
        <div className="nj-tray" aria-label="Attachments">
          {files.map((file) => (
            <span key={file} className="nj-tray__tile">
              <span className="nj-tray__picture" aria-hidden />
              <span className="nj-tray__name">{file}</span>
              <button className="nj-icon nj-icon--sm" aria-label={`Remove ${file}`} data-tip="Remove" onClick={() => setFiles(files.filter((one) => one !== file))}>
                <X size={12} aria-hidden />
              </button>
            </span>
          ))}
        </div>
      ) : null}
      <textarea
        ref={focusRef}
        className="nj-composer__editor"
        aria-label="Message"
        rows={2}
        value={draft}
        onChange={(event) => { setDraft(event.target.value); setMenuAt(0); }}
        onKeyDown={(event) => {
          if (menu.length && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
            event.preventDefault();
            setMenuAt((menuAt + (event.key === "ArrowDown" ? 1 : menu.length - 1)) % menu.length);
          } else if (menu.length && (event.key === "Enter" || event.key === "Tab") && !event.metaKey) {
            event.preventDefault();
            take(menu[menuAt].name);
          } else if (event.key === "Enter" && event.metaKey) {
            event.preventDefault();
            setDraft("");
            setFiles([]);
          }
        }}
      />
      <div className="nj-composer__bar">
        <button className="nj-icon" aria-label="Attach a file" data-tip="Attach" onClick={() => setFiles([...files, `screenshot-${files.length + 1}.png`])}>
          <Paperclip size={14} aria-hidden />
        </button>
        <button
          className="nj-icon"
          aria-label="Talk"
          aria-pressed={listening}
          data-tip="Talk"
          data-tone={listening ? "live" : undefined}
          onClick={() => setListening(!listening)}
        >
          <Mic size={14} aria-hidden />
        </button>
        <span className="nj-composer__hint"><kbd className="nj-kbd">#</kbd> tag <kbd className="nj-kbd">/</kbd> command</span>
        <button className="nj-button" data-primary aria-label="Send" onClick={() => { setDraft(""); setFiles([]); }}>
          <Send size={12} aria-hidden /> Send <kbd className="nj-kbd">⌘↵</kbd>
        </button>
      </div>
    </section>
  );
}

// ── The review sheet: what the open Job waits on, one item at a time.
function ReviewSheet({ job, onClose }: { job: Job; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current?.focus(), []);
  return (
    <div className="nj-scrim" onClick={onClose}>
      <div
        className="nj-sheet"
        role="dialog"
        aria-label="Needs you"
        tabIndex={-1}
        ref={ref}
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.key === "Escape" && onClose()}
      >
        <header className="nj-legend">
          <span className="nj-legend__label">Needs you</span>
          <button className="nj-icon" aria-label="Close" data-tip="Close Esc" onClick={onClose}><X size={14} aria-hidden /></button>
        </header>
        {job.waiting_for.map((item) => (
          <div key={item.id} className="nj-sheet__item" aria-label={item.text}>
            <Glyph icon={item.kind === "plan" ? Stamp : item.kind === "walk" ? Footprints : item.kind === "draft" ? PencilLine : MessageCircleQuestion} tip={item.kind} />
            <span className="nj-sheet__text">{item.text}</span>
            <span className="nj-sheet__acts">
              <button className="nj-button">Send back <kbd className="nj-kbd">⌥R</kbd></button>
              <button className="nj-button" data-primary>Approve <kbd className="nj-kbd">⌘↵</kbd></button>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── The palette: live and ended Jobs by title, project or branch.
function Palette({ jobs, onOpen, onClose }: { jobs: Job[]; onOpen: (id: string) => void; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [at, setAt] = useState(0);
  const all = [...jobs, ...ENDED];
  const words = query.toLowerCase().trim();
  const found = all
    .filter((job) => !words || `${job.title} ${job.manifest_id} ${job.branch}`.toLowerCase().includes(words))
    .filter((job) => words || !job.ended)
    .sort((a, b) => Number(needs(b)) - Number(needs(a)) || Number(!!a.ended) - Number(!!b.ended));
  return (
    <div className="nj-scrim" onClick={onClose}>
      <div className="nj-palette" role="dialog" aria-label="Palette" onClick={(event) => event.stopPropagation()}>
        <div className="nj-palette__field">
          <Search size={14} aria-hidden />
          <input
            autoFocus
            aria-label="Search Jobs"
            placeholder="Search Jobs"
            value={query}
            onChange={(event) => { setQuery(event.target.value); setAt(0); }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") { event.preventDefault(); setAt(Math.min(found.length - 1, at + 1)); }
              if (event.key === "ArrowUp") { event.preventDefault(); setAt(Math.max(0, at - 1)); }
              if (event.key === "Enter" && found[at]) onOpen(found[at].id);
              if (event.key === "Escape") onClose();
            }}
          />
        </div>
        <ul className="nj-palette__rows" role="listbox" aria-label="Jobs">
          {found.map((job, n) => (
            <li key={job.id} role="option" aria-selected={n === at} aria-label={job.title} className="nj-palette__row" data-ended={job.ended || undefined}
              onMouseEnter={() => setAt(n)} onClick={() => onOpen(job.id)}>
              <span className="nj-lamp" data-lamp={job.ended ? "ended" : lampOf(job)} aria-hidden />
              <span className="nj-palette__title">{job.title}</span>
              <span className="nj-palette__project">{job.manifest_id}</span>
              <span className="nj-col">{job.ended ? <Glyph icon={Power} tip="Ended" /> : stateGlyph(job)}</span>
            </li>
          ))}
        </ul>
        <footer className="nj-palette__foot">
          <span>Open <kbd className="nj-kbd">↵</kbd></span>
        </footer>
      </div>
    </div>
  );
}

// ── The banner macOS draws for a notification, drawn here so the walk can show it.
function Banner({ banner, onOpen }: { banner: { job: Job; said: string } | null; onOpen: (id: string) => void }) {
  if (!banner) return null;
  return (
    <button className="nj-banner" aria-label={`${banner.job.title}: ${banner.said}`} onClick={() => onOpen(banner.job.id)}>
      <span className="nj-banner__app" aria-hidden>A</span>
      <span className="nj-banner__words">
        <span className="nj-banner__title">{banner.job.title}</span>
        <span className="nj-banner__said">{banner.said}</span>
      </span>
    </button>
  );
}

function Keys({ onClose }: { onClose: () => void }) {
  const rows: [string, string][] = [
    ["⌘K", "Search Jobs, live and ended"],
    ["⇧⌘Y", "Needs you on the open Job"],
    ["⌥⌘↓", "The next Job that needs you"],
    ["⌘L", "The Jobs list"],
    ["↑ ↓  j k", "Move in the list, and open the Job"],
    ["⌥M", "The message box"],
    ["⌘↵", "Send"],
    ["⌥↵", "Answer the question"],
    ["⌘P", "Pause or resume"],
    ["⌥⌘F", "Fork"],
    ["⌘⌫", "End"],
    ["⌘[  ⌘]", "Back and forward through Jobs opened"],
    ["Space", "Open or fold the tool calls in focus"],
    ["Esc", "Close the sheet or palette"],
  ];
  return (
    <div className="nj-scrim" onClick={onClose}>
      <div className="nj-sheet" role="dialog" aria-label="Keys" onClick={(event) => event.stopPropagation()}>
        <header className="nj-legend"><span className="nj-legend__label">Keys</span></header>
        <dl className="nj-keys">
          {rows.map(([key, act]) => (
            <div key={key}><dt><kbd className="nj-kbd">{key}</kbd></dt><dd>{act}</dd></div>
          ))}
        </dl>
      </div>
    </div>
  );
}

function App() {
  const [jobs, setJobs] = useState<Job[]>(JOBS);
  const [kept, setKept] = useState<Kept>(readKept);
  const [palette, setPalette] = useState(false);
  const [sheet, setSheet] = useState(false);
  const [keys, setKeys] = useState(false);
  const [loading, setLoading] = useState(false);
  const [banner, setBanner] = useState<{ job: Job; said: string } | null>(null);
  const [back, setBack] = useState<string[]>([]);
  const threadRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const composerRef = useRef<HTMLTextAreaElement>(null);

  const job = [...jobs, ...ENDED].find((one) => one.id === kept.job) ?? jobs[0];
  const place: Place = kept.places[job.id] ?? { tab: "Thread", scroll: -1, open: [] };
  const thread = threadOf(job.id);

  useEffect(() => localStorage.setItem(KEPT, JSON.stringify(kept)), [kept]);

  useEffect(() => {
    if (!banner) return;
    const gone = setTimeout(() => setBanner(null), 6000);
    return () => clearTimeout(gone);
  }, [banner]);

  const setPlace = (next: Partial<Place>) =>
    setKept((was) => ({ ...was, places: { ...was.places, [job.id]: { ...place, ...next } } }));

  const open = (id: string) => {
    if (id === job.id) return;
    const scroll = threadRef.current?.scrollTop ?? 0;
    setBack((was) => [...was, job.id]);
    setKept((was) => ({ ...was, job: id, places: { ...was.places, [job.id]: { ...place, scroll } } }));
    setPalette(false);
    setBanner(null);
  };

  // Each Job keeps its place: the scroll comes back as it was, or the end on a first visit.
  useLayoutEffect(() => {
    const el = threadRef.current;
    if (el) el.scrollTop = place.scroll < 0 ? el.scrollHeight : place.scroll;
  }, [job.id, place.tab]);

  const saveScroll = () => {
    const el = threadRef.current;
    if (el) setKept((was) => ({ ...was, places: { ...was.places, [job.id]: { ...place, scroll: el.scrollTop } } }));
  };

  const nextNeeding = () => {
    const needing = jobs.filter(needs);
    if (!needing.length) return;
    const at = needing.findIndex((one) => one.id === job.id);
    open(needing[(at + 1) % needing.length].id);
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey && event.key.toLowerCase() === "k") { event.preventDefault(); setPalette((was) => !was); }
      else if (event.metaKey && event.shiftKey && event.key.toLowerCase() === "y") { event.preventDefault(); if (needs(job)) setSheet(true); }
      else if (event.metaKey && event.altKey && event.key === "ArrowDown") { event.preventDefault(); nextNeeding(); }
      else if (event.metaKey && event.key.toLowerCase() === "l") { event.preventDefault(); listRef.current?.focus(); }
      else if (event.altKey && event.code === "KeyM") { event.preventDefault(); composerRef.current?.focus(); }
      else if (event.metaKey && event.key === "[" && back.length) {
        event.preventDefault();
        const to = back[back.length - 1];
        setBack(back.slice(0, -1));
        setKept((was) => ({ ...was, job: to }));
      } else if (event.key === "?" && !(event.target instanceof HTMLTextAreaElement) && !(event.target instanceof HTMLInputElement)) setKeys(true);
      else if (event.key === "Escape") { setPalette(false); setSheet(false); setKeys(false); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // Mock events: what makes a banner and a sound, and what does not.
  const play = (src: string) => void new Audio(src).play().catch(() => undefined);
  const asks = () => {
    const target = jobs.find((one) => one.id === "699aab92")!;
    const waiting = { id: "q", text: "Answer which Operator queue to keep", kind: "question" as const };
    setJobs(jobs.map((one) => (one.id === target.id ? { ...one, turn: "idle", waiting_for: [waiting] } : one)));
    setBanner({ job: target, said: waiting.text });
    play(waitingSound);
  };
  const stops = () => {
    const target = jobs.find((one) => one.id === "f1a2c3d4")!;
    setJobs(jobs.map((one) => (one.id === target.id ? { ...one, turn: "stopped" } : one)));
    setBanner({ job: target, said: "Stopped" });
    play(blockedSound);
  };
  const merges = () => {
    setJobs(jobs.map((one) => (one.id === "cf2b4b49" ? { ...one, in_flight: [] } : one)));
  };
  const pause = () => setJobs(jobs.map((one) => (one.id === job.id ? { ...one, turn: one.turn === "paused" ? "idle" : "paused" } : one)));
  const switchTab = (tab: string) => {
    setPlace({ tab });
    if (tab !== "Thread") {
      setLoading(true);
      setTimeout(() => setLoading(false), 900);
    }
  };

  const needing = jobs.filter(needs).length;

  return (
    <div className="nj-desk">
      <div className="nj-window" role="application" aria-label="Armada">
        <div className="nj-titlebar">
          <span className="nj-titlebar__lights" aria-hidden><i /><i /><i /></span>
          <span className="nj-titlebar__mark">Armada</span>
          <button className="nj-search" aria-label="Search Jobs" onClick={() => setPalette(true)}>
            <Search size={14} aria-hidden />
            <span>Search Jobs</span>
            <kbd className="nj-kbd">⌘K</kbd>
          </button>
          <button className="nj-titlebar__keys" aria-label="Keys" data-tip="Keys ?" onClick={() => setKeys(true)}>?</button>
        </div>
        <div className="nj-body">
          <JobsList jobs={jobs} selected={job.id} onOpen={open} listRef={listRef} />
          <section className="nj-panel nj-job" aria-label={job.title}>
            <JobHead job={job} onPause={pause} onEnd={() => undefined} onReview={() => setSheet(true)} />
            <nav className="nj-tabs" role="tablist" aria-label="Job panes">
              {["Thread", "Plan", "Changes", "Checks", "Files", "Log"].map((tab) => (
                <button key={tab} role="tab" aria-selected={place.tab === tab} className="nj-tab" onClick={() => switchTab(tab)}>
                  {tab}
                </button>
              ))}
            </nav>
            {place.tab === "Thread" ? (
              <>
                <div className="nj-thread" ref={threadRef} onScroll={saveScroll} role="log" aria-label="Thread">
                  {thread.items.map((item) => (
                    <ThreadItem
                      key={item.kind === "row" ? item.row.id : item.id}
                      item={item}
                      open={place.open.includes(item.kind === "row" ? item.row.id : item.id)}
                      onToggle={() => {
                        const id = item.kind === "row" ? item.row.id : item.id;
                        setPlace({ open: place.open.includes(id) ? place.open.filter((one) => one !== id) : [...place.open, id] });
                      }}
                      onReview={() => setSheet(true)}
                    />
                  ))}
                  {job.open_call ? (
                    <div className="nj-open-call" role="status" aria-label={`Running ${job.open_call.text}`}>
                      <Glyph icon={Radar} tip="Running" tone="live" />
                      <span className="nj-mono">{job.open_call.text}</span>
                      <span className="nj-at">{job.open_call.since}s</span>
                    </div>
                  ) : null}
                </div>
                <Asked thread={thread} />
                <Composer
                  key={job.id}
                  job={job}
                  draft={kept.drafts[job.id] ?? ""}
                  setDraft={(text) => setKept((was) => ({ ...was, drafts: { ...was.drafts, [job.id]: text } }))}
                  focusRef={composerRef}
                />
              </>
            ) : (
              <div className="nj-pane" aria-label={place.tab} aria-busy={loading}>
                {loading ? (
                  <div className="nj-skeleton" aria-hidden>
                    <i /><i /><i /><i />
                  </div>
                ) : null}
              </div>
            )}
          </section>
        </div>
        <Banner banner={banner} onOpen={open} />
      </div>
      <footer className="nj-dock" aria-label="Mock only">
        <span className="nj-dock__app" role="img" aria-label={needing ? `Armada, ${needing} need you` : "Armada"}>
          A{needing ? <span className="nj-dock__badge">{needing}</span> : null}
        </span>
        <span className="nj-dock__events" role="group" aria-label="Mock events">
          <button className="nj-button" onClick={asks}>A Job asks</button>
          <button className="nj-button" onClick={stops}>A Job stops</button>
          <button className="nj-button" onClick={merges}>A pull request merges</button>
        </span>
      </footer>
      {palette ? <Palette jobs={jobs} onOpen={open} onClose={() => setPalette(false)} /> : null}
      {sheet ? <ReviewSheet job={job} onClose={() => setSheet(false)} /> : null}
      {keys ? <Keys onClose={() => setKeys(false)} /> : null}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
