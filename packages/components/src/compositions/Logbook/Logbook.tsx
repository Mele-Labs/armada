import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import {
  BookDashed,
  BookMarked,
  BookPlus,
  CircleAlert,
  Combine,
  Container,
  Lock,
  Repeat,
  Ruler,
  SquareChevronRight,
  TrafficCone,
  type LucideIcon,
} from "lucide-react";
import type { LogbookEarlier, LogbookEntry, LogbookKind, LogbookState } from "@armada/protocol";

import { keyFor } from "../../actions";
import { forgetKept, readKept } from "../../keep";
import { isPressed, pressedSlot } from "../../keymap";
import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Readout } from "../../primitives/Readout/Readout";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";

/**
 * The Logbook: what agents know, and on its own tab what Armada got wrong. `docs/concepts/logbook.md`.
 *
 * **Nothing in the entries asks him for anything.** He reads an entry and its evidence, and can edit, lock or forget
 * it. Every repository's Logbook is drawn at once, each in its own panel, then Habits and This Mac. Three tabs say which
 * entries the panels hold: those in use, the new ones, the archived. An entry draws one line in the list and whole on the right.
 *
 * The Retros tab holds every retro item, whether its fix lands in Armada, Kit or Manifest, drawn by the host (`fixes`) only
 * while the tab is open.
 *
 * The page is keyed by the registry's acts: the arrows move, `Enter` opens the entry, `e`, `l` and `f` edit, lock and
 * forget it, and `Esc` backs out a level. The tabs take the arrows when focused.
 */

/** One panel: a repository's Logbook, Habits, or This Mac. `hint` is what its header says on hover. */
export type LogbookPanel = { key: string; title: string; hint: string; entries: readonly LogbookEntry[] };

/** A tab: the entries in one state, or the retro items. */
export type LogbookShown = LogbookState | "retros";

export type LogbookViewProps = {
  panels: readonly LogbookPanel[];
  /** The tab chosen: the panels hold entries in this state, or Retros holds `fixes`. */
  shown: LogbookShown;
  onShown: (shown: LogbookShown) => void;
  /** The retro items, drawn by the host under the Retros tab. Absent, the tab is not drawn. */
  fixes?: ReactNode;
  /** Changes when something asks for the retro items: their tab takes focus. */
  toFixes?: number;
  /** The Logbook could not be read, so only the Retros tab is drawn. */
  unread?: boolean;
  /** The entry open on the right. */
  selected?: string;
  onSelect: (id: string) => void;
  /** Its text is open in place. */
  editing: boolean;
  onEdit: (editing: boolean) => void;
  onSave: (id: string, text: string) => void;
  onLock: (id: string, locked: boolean) => void;
  onForget: (id: string) => void;
  /** Tidy now: folds near-duplicates together. Absent, the button is not drawn. */
  onTidy?: () => void;
  /** Fleet is tidying, so the button is busy. */
  tidying?: boolean;
  /** What the last tidy came to, as a plain fact. */
  tidied?: string;
  /** Opens the Job an evidence row came from. */
  onOpenJob?: (jobId: string) => void;
  /** What else an entry changed, drawn under its text. Absent, or absent for an entry, nothing is drawn. */
  also?: (entry: LogbookEntry) => ReactNode;
  /** Fleet refused the last act on the open entry. Drawn above its text until another entry is opened. */
  refusal?: { title: string; said: string };
  now: number;
};

/**
 * Whether Fleet keeps the entry, so the owner may act on it. Run memory and the machine note are folded in with ids that
 * start `runs-` and `machine-`, and Fleet refuses every act on them (`fleet.logbook_entry_not_kept`).
 */
export const isKept = (entry: Pick<LogbookEntry, "id">): boolean => !entry.id.startsWith("runs-") && !entry.id.startsWith("machine-");

const KIND: Record<LogbookKind, { Glyph: LucideIcon; said: string }> = {
  command: { Glyph: SquareChevronRight, said: "Command" },
  gotcha: { Glyph: TrafficCone, said: "Gotcha" },
  convention: { Glyph: Ruler, said: "Convention" },
  environment: { Glyph: Container, said: "Environment" },
  habit: { Glyph: Repeat, said: "Habit" },
};

const STATE: Record<LogbookState, { Glyph: LucideIcon; said: string }> = {
  standing: { Glyph: BookMarked, said: "Standing: handed to every new Job" },
  candidate: { Glyph: BookPlus, said: "Candidate: handed out once two Jobs confirm it" },
  faded: { Glyph: BookDashed, said: "Faded: kept, no longer handed out" },
};

const TABS: { id: LogbookShown; label: string; hint: string }[] = [
  { id: "standing", label: "In use", hint: "Handed to every new Job" },
  { id: "candidate", label: "New", hint: "Seen in one Job. A second Job puts it in use" },
  { id: "faded", label: "Archived", hint: "Unused for 30 days. Recall still finds it" },
];
const RETROS = { id: "retros" as const, label: "Retros", hint: "What got in a Job's way, and where its fix lands" };

/** One Job's evidence: its rows, newest first, under the Job once. */
type JobEvidence = { job_id: string; job_title: string; rows: LogbookEntry["evidence"][number][] };

/** The evidence one row a Job, the Job with the newest row first. */
function byJob(evidence: LogbookEntry["evidence"]): JobEvidence[] {
  const jobs = new Map<string, JobEvidence>();
  for (const row of evidence) {
    const job = jobs.get(row.job_id) ?? { job_id: row.job_id, job_title: row.job_title, rows: [] };
    job.rows.push(row);
    jobs.set(row.job_id, job);
  }
  const newest = (job: JobEvidence) => Math.max(...job.rows.map((row) => Date.parse(row.at)));
  return [...jobs.values()].sort((a, b) => newest(b) - newest(a));
}

/** Who replaced an earlier text, in the words its line in Earlier opens with. */
const EARLIER: Record<LogbookEarlier["by"], string> = { person: "Replaced by you", distil: "Rewritten by Armada", merged: "Merged" };

const LOCKED = "Locked: Armada never rewrites it or lets it fade.";

const ago = (at: string, now: number): string => ageOf(Date.parse(at), now);
const dated = (at: string): string => new Date(at).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

/** A registry glyph that names itself on hover, as every bare mark here does. */
function Mark({ Glyph, said, tone }: { Glyph: LucideIcon; said: string; tone?: "tripped" }) {
  return (
    <Tooltip label={said}>
      <span className="armada-logbook__mark" data-tone={tone} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function marksOf(entry: LogbookEntry, now: number, withState: boolean): ReactNode {
  const kind = KIND[entry.kind];
  const state = STATE[entry.state];
  return (
    <>
      {entry.last_tripped === undefined ? null : <Mark Glyph={CircleAlert} said={`Tripped ${ago(entry.last_tripped, now)}`} tone="tripped" />}
      {entry.locked ? <Mark Glyph={Lock} said={LOCKED} /> : null}
      {withState ? <Mark Glyph={state.Glyph} said={state.said} /> : null}
      <Mark Glyph={kind.Glyph} said={kind.said} />
    </>
  );
}

/** A time drawn as its age in a readout, with the date on hover, and the column kept where a row sits in a list. */
function When({ at, now, said, fixed = false }: { at: string; now: number; said?: string; fixed?: boolean }) {
  return (
    <Tooltip label={said === undefined ? dated(at) : `${said} ${dated(at)}`}>
      <Readout fixed={fixed}>
        <time dateTime={at}>{ago(at, now)}</time>
      </Readout>
    </Tooltip>
  );
}

/** Words with their `quoted` spans set in the mono face, as an entry writes a command. */
function Words({ text }: { text: string }) {
  return (
    <>
      {text.split("`").map((part, at) =>
        at % 2 === 1 ? (
          <code key={at} className="armada-logbook__code">
            {part}
          </code>
        ) : (
          part
        ),
      )}
    </>
  );
}

const optionId = (id: string) => `logbook-entry-${id}`;
const draftKey = (id: string) => `logbook:${id}`;

export function LogbookView({ panels, shown, onShown, fixes, toFixes, unread, selected, onSelect, editing, onEdit, onSave, onLock, onForget, onTidy, tidying, tidied, onOpenJob, also, refusal, now }: LogbookViewProps) {
  const list = useRef<HTMLDivElement>(null);
  const part = useRef<HTMLElement>(null);
  const detail = useRef<HTMLElement>(null);
  const [forgetting, setForgetting] = useState<string>();
  const order = panels.flatMap((panel) => panel.entries);
  const open = order.find((one) => one.id === selected);
  const panelOf = (id: string) => panels.find((panel) => panel.entries.some((one) => one.id === id));

  // The selected row is kept in view as the arrows move it.
  useEffect(() => {
    if (selected !== undefined) document.getElementById(optionId(selected))?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  // Something asked for the retro items: their tab is the host's to open, and it takes focus once drawn.
  useEffect(() => {
    if (toFixes === undefined || toFixes === 0) return;
    part.current?.focus({ preventScroll: true });
  }, [toFixes]);

  const move = (by: 1 | -1) => {
    const at = order.findIndex((one) => one.id === selected);
    const next = order[at < 0 ? 0 : Math.min(order.length - 1, Math.max(0, at + by))];
    if (next !== undefined) onSelect(next.id);
  };

  function pressed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement;
    // The text box keeps its keys, Esc aside, which leaves it with the draft kept.
    if (target.tagName === "TEXTAREA" || target.tagName === "INPUT") {
      if (isPressed("close", event)) {
        event.preventDefault();
        onEdit(false);
        window.requestAnimationFrame(() => detail.current?.focus());
      }
      return;
    }
    // The retro items keep their own keys.
    if (part.current?.contains(target) === true) return;
    const inList = list.current?.contains(target) === true;
    const slot = pressedSlot("move_focus", event);
    if (slot !== -1 && inList) {
      event.preventDefault();
      move(slot === 0 || slot === 2 ? 1 : -1);
      return;
    }
    if (isPressed("open_focused", event) && inList && open !== undefined) {
      event.preventDefault();
      detail.current?.focus();
      return;
    }
    if (isPressed("close", event) && detail.current?.contains(target) === true) {
      event.preventDefault();
      list.current?.focus();
      return;
    }
    if (open === undefined || !isKept(open)) return;
    if (isPressed("logbook_edit", event)) {
      event.preventDefault();
      onEdit(true);
    } else if (isPressed("logbook_lock", event)) {
      event.preventDefault();
      onLock(open.id, !open.locked);
    } else if (isPressed("logbook_forget", event)) {
      event.preventDefault();
      setForgetting(open.id);
    }
  }

  const retros = shown === "retros" && fixes !== undefined;
  const tabs = fixes === undefined ? TABS : [...TABS, RETROS];
  return (
    <div className="armada-logbook" onKeyDown={pressed}>
      <div className="armada-logbook__filters">
        <Tabs items={tabs} value={shown} onChange={(id) => onShown(id as LogbookShown)} />
        {retros || unread === true || onTidy === undefined ? null : (
          <Tooltip label="Tidy now">
            <Button variant="ghost" size="sm" iconOnly aria-label="Tidy now" pending={tidying === true} onClick={onTidy}>
              <Combine size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
        {retros ? null : (
          <span className="armada-logbook__tidied" role="status">
            {tidied}
          </span>
        )}
      </div>
      {retros ? (
        <section ref={part} className="armada-logbook__fixes" aria-label="Retros" tabIndex={-1}>
          {fixes}
        </section>
      ) : unread === true ? (
        <p className="armada-logbook__unread">Fleet serves no Logbook yet.</p>
      ) : (
        <div className="armada-logbook__page">
          <div
            ref={list}
            className="armada-logbook__list"
            role="listbox"
            aria-label="Entries"
            tabIndex={0}
            aria-activedescendant={open === undefined ? undefined : optionId(open.id)}
          >
            {panels.map((panel) => (
              <div key={panel.key} className="armada-logbook__panel" role="group" aria-labelledby={`logbook-panel-${panel.key}`}>
                <Tooltip label={panel.hint} asChild>
                  <div className="armada-logbook__band" id={`logbook-panel-${panel.key}`}>
                    {panel.title}
                  </div>
                </Tooltip>
                {panel.entries.map((entry) => (
                  <div
                    key={entry.id}
                    id={optionId(entry.id)}
                    className="armada-logbook__row"
                    role="option"
                    aria-selected={entry.id === selected}
                    onClick={() => {
                      onSelect(entry.id);
                      list.current?.focus();
                    }}
                  >
                    <span className="armada-logbook__text">
                      <Words text={entry.text} />
                    </span>
                    <span className="armada-logbook__marks">{marksOf(entry, now, false)}</span>
                    <When at={entry.last_confirmed} now={now} said="Last confirmed" fixed />
                  </div>
                ))}
              </div>
            ))}
          </div>
          <section ref={detail} className="armada-logbook__detail" aria-label="Entry" tabIndex={-1}>
            {open === undefined ? null : (
              <Detail
                key={open.id}
                entry={open}
                where={panelOf(open.id)?.title ?? ""}
                editing={editing}
                now={now}
                {...(refusal === undefined ? {} : { refusal })}
                onEdit={onEdit}
                onSave={(text) => {
                  onSave(open.id, text);
                  window.requestAnimationFrame(() => detail.current?.focus());
                }}
                onLock={() => onLock(open.id, !open.locked)}
                onForget={() => setForgetting(open.id)}
                {...(onOpenJob === undefined ? {} : { onOpenJob })}
                {...(also === undefined ? {} : { also: also(open) })}
              />
            )}
          </section>
        </div>
      )}
      <Dialog
        open={forgetting !== undefined}
        title="Forget this entry"
        tone="destructive"
        confirmLabel="Forget"
        onConfirm={() => {
          if (forgetting !== undefined) onForget(forgetting);
          setForgetting(undefined);
          window.requestAnimationFrame(() => list.current?.focus());
        }}
        onCancel={() => setForgetting(undefined)}
      >
        <p>No later Job brings it back.</p>
      </Dialog>
    </div>
  );
}

function Detail({
  entry,
  where,
  editing,
  now,
  refusal,
  onEdit,
  onSave,
  onLock,
  onForget,
  onOpenJob,
  also,
}: {
  entry: LogbookEntry;
  where: string;
  editing: boolean;
  now: number;
  refusal?: { title: string; said: string };
  onEdit: (editing: boolean) => void;
  onSave: (text: string) => void;
  onLock: () => void;
  onForget: () => void;
  onOpenJob?: (jobId: string) => void;
  also?: ReactNode;
}) {
  const key = draftKey(entry.id);
  const kept = isKept(entry);
  const [draft, setDraft] = useState(() => readKept<string>(key) ?? entry.text);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (editing) box.current?.querySelector("textarea")?.focus();
  }, [editing]);
  const save = () => {
    const text = draft.trim();
    if (text === "") return;
    forgetKept(key);
    onSave(text);
    onEdit(false);
  };
  return (
    <div className="armada-logbook__card">
      <div className="armada-logbook__band">{where}</div>
      {/* The text and its acts stay put; what backs it scrolls under them. */}
      <div className="armada-logbook__top">
        <div className="armada-logbook__marks">{marksOf(entry, now, true)}</div>
        {refusal === undefined ? null : (
          <Alert tone="escalated" title={refusal.title}>
            {refusal.said}
          </Alert>
        )}
        {editing && kept ? (
          <div
            ref={box}
            className="armada-logbook__edit"
            onKeyDown={(event) => {
              if (event.key === "Enter" && event.metaKey) {
                event.preventDefault();
                save();
              }
            }}
          >
            <Textarea aria-label="Entry text" keep={key} value={draft} rows={3} grow onChange={(event) => setDraft(event.currentTarget.value)} />
            <div className="armada-logbook__acts">
              <Tooltip label="Save" shortcut="⌘↵">
                <Button variant="primary" size="sm" onClick={save} disabled={draft.trim() === ""}>
                  Save
                </Button>
              </Tooltip>
            </div>
          </div>
        ) : (
          <>
            <p className="armada-logbook__said">
              <Words text={entry.text} />
            </p>
            {also}
            {kept ? (
              <div className="armada-logbook__acts">
                <Act label="Edit" act="logbook_edit" onPress={() => onEdit(true)} />
                <Act label={entry.locked ? "Unlock" : "Lock"} act="logbook_lock" tip={entry.locked ? "Armada may rewrite it and let it fade again" : "Armada never rewrites it or lets it fade"} onPress={onLock} />
                <Act label="Forget" act="logbook_forget" onPress={onForget} />
              </div>
            ) : null}
          </>
        )}
        <dl className="armada-logbook__facts">
          <div>
            <dt>Confirmed by</dt>
            <dd>{entry.confirmed_by === 1 ? "1 Job" : `${entry.confirmed_by} Jobs`}</dd>
          </div>
          <div>
            <dt>Confirmed</dt>
            <dd>
              <When at={entry.last_confirmed} now={now} said="Last confirmed" />
            </dd>
          </div>
          <div>
            <dt>First seen</dt>
            <dd>
              <When at={entry.first_seen} now={now} said="First seen" />
            </dd>
          </div>
          {entry.last_served === undefined ? null : (
            <div>
              <dt>Served</dt>
              <dd>
                <When at={entry.last_served} now={now} said="Last served" />
              </dd>
            </div>
          )}
        </dl>
      </div>
      <div className="armada-logbook__body">
        {entry.paths === undefined || entry.paths.length === 0 ? null : (
          <div className="armada-logbook__part">
            <h3 className="armada-logbook__head">Paths</h3>
            <ul className="armada-logbook__paths">
              {entry.paths.map((path) => (
                <li key={path}>{path}</li>
              ))}
            </ul>
          </div>
        )}
        {entry.evidence.length === 0 ? null : (
          <div className="armada-logbook__part">
            <h3 className="armada-logbook__head">Evidence</h3>
            <ul className="armada-logbook__evidence">
              {byJob(entry.evidence).map((job) => (
                <li key={job.job_id}>
                  <Tooltip label={job.job_title}>
                    {onOpenJob === undefined ? (
                      <span className="armada-logbook__job">{job.job_title}</span>
                    ) : (
                      <button type="button" className="armada-logbook__job" onClick={() => onOpenJob(job.job_id)}>
                        {job.job_title}
                      </button>
                    )}
                  </Tooltip>
                  <span className="armada-logbook__cites">
                    {job.rows.map((row) => (
                      <Tooltip key={row.cite} label={row.says === undefined ? dated(row.at) : `${row.says.replaceAll("`", "")} · ${dated(row.at)}`}>
                        <Readout className="armada-logbook__cite">{row.cite}</Readout>
                      </Tooltip>
                    ))}
                  </span>
                  <When at={job.rows.reduce((a, b) => (Date.parse(a.at) > Date.parse(b.at) ? a : b)).at} now={now} fixed />
                </li>
              ))}
            </ul>
          </div>
        )}
        {entry.history.length === 0 ? null : (
          <div className="armada-logbook__part">
            <h3 className="armada-logbook__head">Earlier</h3>
            <ul className="armada-logbook__history">
              {entry.history.map((one) => (
                <li key={`${one.until}:${one.text}`}>
                  <span className="armada-logbook__says">
                    <Words text={one.text} />
                  </span>
                  <span className="armada-logbook__from">
                    {EARLIER[one.by]} <When at={one.until} now={now} />
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

/** An act on the entry, with the key that does it beside its name. */
function Act({ label, act, tip, onPress }: { label: string; act: string; tip?: string; onPress: () => void }) {
  const key = keyFor(act);
  const button = (
    <Button variant={act === "logbook_forget" ? "ghost" : "secondary"} size="sm" onClick={onPress} aria-keyshortcuts={key === "" ? undefined : key}>
      {label}
      {key === "" ? null : <Kbd aria-hidden>{key}</Kbd>}
    </Button>
  );
  return tip === undefined ? button : <Tooltip label={tip}>{button}</Tooltip>;
}
