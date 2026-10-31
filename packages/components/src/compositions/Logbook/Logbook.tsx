import { Fragment, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import {
  BookDashed,
  BookMarked,
  BookPlus,
  Bug,
  CalendarX,
  CircleAlert,
  Combine,
  Container,
  Lock,
  Repeat,
  Ruler,
  SquareChevronRight,
  TrafficCone,
  X,
  type LucideIcon,
} from "lucide-react";
import type { LogbookEarlier, LogbookEntry, LogbookKind, LogbookState, LogbookTopic } from "@armada/protocol";

import { keyFor } from "../../actions";
import { forgetKept, readKept } from "../../keep";
import { isPressed, pressedSlot } from "../../keymap";
import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "../../primitives/Table/Table";
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

/** The three ways Organize looks through the entries. */
export type OrganizeAction = "merge" | "stale" | "bugs";
/** One proposal row: the entry it is about, why, and for a merge the entries folded into it. */
export type OrganizeRow = { id: string; text: string; note: string; absorbed?: readonly string[] };
export type OrganizeProposal = { action: OrganizeAction; rows: readonly OrganizeRow[] };

const ORGANIZE: Record<OrganizeAction, { label: string; hint: string; Glyph: LucideIcon; all: string; title: string }> = {
  merge: { label: "Merge duplicates", hint: "Folds near-duplicate entries into one", Glyph: Combine, all: "Apply all", title: "Merge duplicates" },
  stale: { label: "Find stale entries", hint: "Entries no recent Job has confirmed or tripped", Glyph: CalendarX, all: "Archive all", title: "Stale entries" },
  bugs: { label: "Find Armada bugs", hint: "Entries that work around a fault in Armada itself, such as a tool refusing or erroring", Glyph: Bug, all: "Report all", title: "Armada bugs" },
};

/** One panel: a repository's Logbook, Habits, or This Mac. `hint` is what its header says on hover. */
export type LogbookPanel = { key: string; title: string; hint: string; entries: readonly LogbookEntry[] };

/** The fixed topics, in the order the chips and the groups in a panel read. */
export const LOGBOOK_TOPICS: readonly { id: LogbookTopic; label: string }[] = [
  { id: "git", label: "Git and PRs" },
  { id: "slots", label: "Slots and worktrees" },
  { id: "testing", label: "Testing and checks" },
  { id: "tools", label: "Fleet and Bridge tools" },
  { id: "mac", label: "Mac and shell" },
  { id: "other", label: "Other" },
];

/** The topic an entry is drawn under: the one the sorter placed it in, or Other. */
export const topicOf = (entry: Pick<LogbookEntry, "topic">): LogbookTopic => entry.topic ?? "other";

const topicLabel = (id: LogbookTopic): string => LOGBOOK_TOPICS.find((one) => one.id === id)?.label ?? "Other";

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
  /** The topics that have entries under this tab, drawn as chips above the list after All. Absent, no chips. */
  topics?: readonly LogbookTopic[];
  /** The topic chosen; absent is All. */
  topic?: LogbookTopic;
  onTopic?: (topic: LogbookTopic | undefined) => void;
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
  /** Organize: looks through the entries in one of three ways. Absent, the Tidy button is drawn instead. */
  onOrganize?: (action: OrganizeAction) => void;
  /** Fleet is looking, so the menu is off. */
  organizing?: boolean;
  /** What the last look found, drawn in place of the open entry. */
  proposal?: OrganizeProposal;
  /** The proposal's one act on every row left. */
  onApplyAll?: () => void;
  onDismissRow?: (id: string) => void;
  onCloseProposal?: () => void;
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

/** Each kind's mark, its word, and the three letters its column reads, as a log codes a line. */
const KIND: Record<LogbookKind, { Glyph: LucideIcon; said: string; code: string }> = {
  command: { Glyph: SquareChevronRight, said: "Command", code: "CMD" },
  gotcha: { Glyph: TrafficCone, said: "Gotcha", code: "GOT" },
  convention: { Glyph: Ruler, said: "Convention", code: "CNV" },
  environment: { Glyph: Container, said: "Environment", code: "ENV" },
  habit: { Glyph: Repeat, said: "Habit", code: "HAB" },
};

const STATE: Record<LogbookState, { Glyph: LucideIcon; said: string; word: string }> = {
  standing: { Glyph: BookMarked, said: "Standing: handed to every new Job", word: "In use" },
  candidate: { Glyph: BookPlus, said: "Candidate: handed out once two Jobs confirm it", word: "New" },
  faded: { Glyph: BookDashed, said: "Faded: kept, no longer handed out", word: "Archived" },
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

/** What sets an entry apart in its row: tripped on, or locked. */
function flagsOf(entry: LogbookEntry, now: number): ReactNode {
  return (
    <>
      {entry.last_tripped === undefined ? null : <Mark Glyph={CircleAlert} said={`Tripped ${ago(entry.last_tripped, now)}`} tone="tripped" />}
      {entry.locked ? <Mark Glyph={Lock} said={LOCKED} /> : null}
    </>
  );
}

/**
 * The few words a row reads: the entry's title, or for one written before titles its first clause with the quoting
 * dropped, so the line is words and not the half of a command.
 */
export function titleOf(entry: Pick<LogbookEntry, "title" | "text">): string {
  if (entry.title !== undefined && entry.title !== "") return entry.title;
  const plain = entry.text.replaceAll("`", "").replace(/\s*\([^)]*\)/g, "");
  return plain.split(/(?<=[.:;])\s|\.$/)[0]?.replace(/[.:;]$/, "") ?? plain;
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

export function LogbookView({ panels, shown, onShown, fixes, toFixes, unread, topics, topic, onTopic, selected, onSelect, editing, onEdit, onSave, onLock, onForget, onTidy, tidying, tidied, onOrganize, organizing, proposal, onApplyAll, onDismissRow, onCloseProposal, onOpenJob, also, refusal, now }: LogbookViewProps) {
  const list = useRef<HTMLTableElement>(null);
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

  const [menu, setMenu] = useState<"state" | "topic" | "organize" | undefined>(undefined);
  const topicsShown = !(shown === "retros" && fixes !== undefined) && unread !== true && topics !== undefined && topics.length > 0 && onTopic !== undefined;

  const organizeShown = onOrganize !== undefined && unread !== true && !(shown === "retros" && fixes !== undefined);

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
    if (event.key === "s" && !event.shiftKey) {
      event.preventDefault();
      setMenu("state");
      return;
    }
    if (event.key === "t" && !event.shiftKey && topicsShown) {
      event.preventDefault();
      setMenu("topic");
      return;
    }
    if (event.key === "o" && !event.shiftKey && organizeShown) {
      event.preventDefault();
      setMenu("organize");
      return;
    }
    if (isPressed("close", event) && proposal !== undefined && detail.current?.contains(target) === true) {
      event.preventDefault();
      onCloseProposal?.();
      list.current?.focus();
      return;
    }
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
        <DropdownMenu
          align="start"
          triggerLabel={tabs.find((one) => one.id === shown)?.label ?? "In use"}
          triggerAriaLabel="State"
          triggerTooltip="State (S)"
          entries={tabs.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, hint: one.hint, selected: one.id === shown }))}
          onSelect={(id) => onShown(id as LogbookShown)}
          open={menu === "state"}
          onOpenChange={(next) => setMenu(next ? "state" : undefined)}
        />
        {topicsShown ? (
          <DropdownMenu
            align="start"
            triggerLabel={topic === undefined ? "All topics" : topicLabel(topic)}
            triggerAriaLabel="Topic"
            triggerTooltip="Topic (T)"
            entries={topicEntries(topics, topic)}
            onSelect={(id) => onTopic?.(id === "all" ? undefined : (id as LogbookTopic))}
            open={menu === "topic"}
            onOpenChange={(next) => setMenu(next ? "topic" : undefined)}
          />
        ) : null}
        {organizeShown ? (
          <DropdownMenu
            align="start"
            triggerLabel="Organize"
            triggerTooltip="Organize (O)"
            disabled={organizing === true}
            entries={(Object.keys(ORGANIZE) as OrganizeAction[]).map((id): DropdownMenuEntry => ({ kind: "item", id, label: ORGANIZE[id].label, icon: ORGANIZE[id].Glyph, hint: ORGANIZE[id].hint }))}
            onSelect={(id) => onOrganize?.(id as OrganizeAction)}
            open={menu === "organize"}
            onOpenChange={(next) => setMenu(next ? "organize" : undefined)}
          />
        ) : null}
        {retros || unread === true || onTidy === undefined || onOrganize !== undefined ? null : (
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
          <div className="armada-logbook__panel armada-logbook__log">
            {/* The Table primitive's chrome, as a bare table so the keys can hold it. */}
            <table
              ref={list}
              className="armada-table armada-logbook__table"
              role="grid"
              aria-label="Entries"
              tabIndex={0}
              aria-activedescendant={open === undefined ? undefined : optionId(open.id)}
            >
              <colgroup>
                <col className="armada-logbook__col-kind" />
                <col />
                <col className="armada-logbook__col-flags" />
                <col className="armada-logbook__col-jobs" />
                <col className="armada-logbook__col-when" />
              </colgroup>
              <TableHead>
                <tr>
                  <TableHeaderCell>Kind</TableHeaderCell>
                  <TableHeaderCell>Entry</TableHeaderCell>
                  <TableHeaderCell aria-label="Flags" />
                  <TableHeaderCell className="armada-logbook__num">Jobs</TableHeaderCell>
                  <TableHeaderCell className="armada-logbook__num">Last</TableHeaderCell>
                </tr>
              </TableHead>
              {panels.map((panel) => (
                <TableBody key={panel.key} aria-labelledby={`logbook-panel-${panel.key}`}>
                  <tr className="armada-logbook__section">
                    <Tooltip label={panel.hint} asChild>
                      <th scope="rowgroup" colSpan={5} id={`logbook-panel-${panel.key}`}>
                        {panel.title}
                      </th>
                    </Tooltip>
                  </tr>
                  {panel.entries.map((entry, at) => (
                    <Fragment key={entry.id}>
                    {topic !== undefined || topics === undefined || (at > 0 && topicOf(panel.entries[at - 1]!) === topicOf(entry)) ? null : (
                      <tr className="armada-logbook__topic">
                        <th scope="rowgroup" colSpan={5}>
                          {topicLabel(topicOf(entry))}
                        </th>
                      </tr>
                    )}
                    <TableRow
                      id={optionId(entry.id)}
                      aria-selected={entry.id === selected}
                      selected={entry.id === selected}
                      className="armada-logbook__row"
                      onClick={() => {
                        onSelect(entry.id);
                        list.current?.focus();
                      }}
                    >
                      <TableCell variant="mono" className="armada-logbook__kind">
                        <Tooltip label={KIND[entry.kind].said}>
                          <span>{KIND[entry.kind].code}</span>
                        </Tooltip>
                      </TableCell>
                      <TableCell truncates>{titleOf(entry)}</TableCell>
                      <TableCell className="armada-logbook__flags">{flagsOf(entry, now)}</TableCell>
                      <TableCell variant="mono" className="armada-logbook__num">
                        {entry.confirmed_by}
                      </TableCell>
                      <TableCell variant="mono" className="armada-logbook__num">
                        <Tooltip label={`Last confirmed ${dated(entry.last_confirmed)}`}>
                          <time dateTime={entry.last_confirmed}>{ago(entry.last_confirmed, now)}</time>
                        </Tooltip>
                      </TableCell>
                    </TableRow>
                    </Fragment>
                  ))}
                </TableBody>
              ))}
            </table>
          </div>
          <section ref={detail} className="armada-logbook__detail" aria-label={proposal === undefined ? "Entry" : "Proposal"} tabIndex={-1}>
            {proposal !== undefined ? (
              <Proposal proposal={proposal} onApplyAll={() => onApplyAll?.()} onDismiss={(id) => onDismissRow?.(id)} onClose={() => onCloseProposal?.()} />
            ) : open === undefined ? null : (
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
      {/* The entry's own head, never the list's section: what it is, its id, its flags and its acts. It stays, with the
          title under it; the rule and all that backs it scroll beneath, however long the rule runs. */}
      <div className="armada-logbook__band armada-logbook__head-band">
        <span className="armada-logbook__head-kind">
          <Mark Glyph={KIND[entry.kind].Glyph} said={KIND[entry.kind].said} />
          {KIND[entry.kind].code}
        </span>
        <span className="armada-logbook__head-id">{entry.id}</span>
        <span className="armada-logbook__head-where">
          {flagsOf(entry, now)}
          {kept && !editing ? (
            <span className="armada-logbook__acts">
              <Act label="Edit" act="logbook_edit" onPress={() => onEdit(true)} />
              <Act label={entry.locked ? "Unlock" : "Lock"} act="logbook_lock" tip={entry.locked ? "Armada may rewrite it and let it fade again" : "Armada never rewrites it or lets it fade"} onPress={onLock} />
              <Act label="Forget" act="logbook_forget" onPress={onForget} />
            </span>
          ) : null}
        </span>
      </div>
      <div className="armada-logbook__top">
        {refusal === undefined ? null : (
          <Alert tone="escalated" title={refusal.title}>
            {refusal.said}
          </Alert>
        )}
        <h2 className="armada-logbook__title">{titleOf(entry)}</h2>
      </div>
      <div className="armada-logbook__body">
        <div className="armada-logbook__rule">
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
            </>
          )}
        </div>
        <Table className="armada-logbook__record" aria-label="Record">
          <colgroup>
            <col className="armada-logbook__col-by" />
            <col />
          </colgroup>
          <TableHead>
            <tr>
              <TableHeaderCell colSpan={2}>Record</TableHeaderCell>
            </tr>
          </TableHead>
          <TableBody>
            <Fact name="Kept in">{where}</Fact>
            <Fact name="Kind">{KIND[entry.kind].said}</Fact>
            {entry.topic === undefined ? null : <Fact name="Topic">{topicLabel(entry.topic)}</Fact>}
            <Fact name="State">{STATE[entry.state].word}</Fact>
            <Fact name="Confirmed by" mono>
              {entry.confirmed_by === 1 ? "1 Job" : `${entry.confirmed_by} Jobs`}
            </Fact>
            <Fact name="Last confirmed" mono>
              <Stamp at={entry.last_confirmed} now={now} />
            </Fact>
            <Fact name="First seen" mono>
              <Stamp at={entry.first_seen} now={now} />
            </Fact>
            {entry.last_served === undefined ? null : (
              <Fact name="Last served" mono>
                <Stamp at={entry.last_served} now={now} />
              </Fact>
            )}
            {entry.last_tripped === undefined ? null : (
              <Fact name="Last tripped" mono>
                <Stamp at={entry.last_tripped} now={now} />
              </Fact>
            )}
            <Fact name="Locked">{entry.locked ? "Yes" : "No"}</Fact>
            {entry.paths === undefined || entry.paths.length === 0 ? null : (
              <Fact name="Paths" mono>
                {entry.paths.join("  ")}
              </Fact>
            )}
          </TableBody>
        </Table>
        {entry.evidence.length === 0 ? null : (
          <Table className="armada-logbook__evidence" aria-label="Evidence">
            <colgroup>
              <col />
              <col className="armada-logbook__col-cites" />
              <col className="armada-logbook__col-when" />
            </colgroup>
            <TableHead>
              <tr>
                <TableHeaderCell>Evidence · Job</TableHeaderCell>
                <TableHeaderCell>Cites</TableHeaderCell>
                <TableHeaderCell className="armada-logbook__num">Last</TableHeaderCell>
              </tr>
            </TableHead>
            <TableBody>
              {byJob(entry.evidence).map((job) => (
                <TableRow key={job.job_id}>
                  <TableCell truncates>
                    <Tooltip label={job.job_title} asChild>
                      {onOpenJob === undefined ? (
                        <span>{job.job_title}</span>
                      ) : (
                        <button type="button" className="armada-logbook__job" onClick={() => onOpenJob(job.job_id)}>
                          {job.job_title}
                        </button>
                      )}
                    </Tooltip>
                  </TableCell>
                  <TableCell variant="mono">
                    <span className="armada-logbook__cites">
                      {job.rows.map((row) => (
                        <Tooltip key={row.cite} label={row.says === undefined ? dated(row.at) : `${row.says.replaceAll("`", "")} · ${dated(row.at)}`}>
                          <span className="armada-logbook__cite">{row.cite}</span>
                        </Tooltip>
                      ))}
                    </span>
                  </TableCell>
                  <TableCell variant="mono" className="armada-logbook__num">
                    <Stamp at={job.rows.reduce((a, b) => (Date.parse(a.at) > Date.parse(b.at) ? a : b)).at} now={now} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
        {entry.history.length === 0 ? null : (
          <Table className="armada-logbook__earlier" aria-label="Earlier">
            <colgroup>
              <col className="armada-logbook__col-by" />
              <col />
              <col className="armada-logbook__col-when" />
            </colgroup>
            <TableHead>
              <tr>
                <TableHeaderCell>Earlier · By</TableHeaderCell>
                <TableHeaderCell>Text</TableHeaderCell>
                <TableHeaderCell className="armada-logbook__num">Until</TableHeaderCell>
              </tr>
            </TableHead>
            <TableBody>
              {entry.history.map((one) => (
                <TableRow key={`${one.until}:${one.text}`}>
                  <TableCell variant="secondary">{EARLIER[one.by]}</TableCell>
                  <TableCell variant="secondary">
                    <Words text={one.text} />
                  </TableCell>
                  <TableCell variant="mono" className="armada-logbook__num">
                    <Stamp at={one.until} now={now} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </div>
    </div>
  );
}

/** All topics, then each topic that has entries, the one chosen marked. */
function topicEntries(topics: readonly LogbookTopic[], topic: LogbookTopic | undefined): DropdownMenuEntry[] {
  return [
    { kind: "item", id: "all", label: "All topics", selected: topic === undefined },
    ...LOGBOOK_TOPICS.filter((one) => topics.includes(one.id)).map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, selected: one.id === topic })),
  ];
}

/** One row of the record: its name, and its value. */
function Fact({ name, mono = false, children }: { name: string; mono?: boolean; children: ReactNode }) {
  return (
    <TableRow>
      <th scope="row" className="armada-logbook__fact">
        {name}
      </th>
      <TableCell variant={mono ? "mono" : "primary"}>{children}</TableCell>
    </TableRow>
  );
}

/** A time as its age, with the date on hover. */
function Stamp({ at, now }: { at: string; now: number }) {
  return (
    <Tooltip label={dated(at)}>
      <time dateTime={at}>{ago(at, now)}</time>
    </Tooltip>
  );
}

/** An act on the entry, with the key that does it beside its name. */
function Proposal({ proposal, onApplyAll, onDismiss, onClose }: { proposal: OrganizeProposal; onApplyAll: () => void; onDismiss: (id: string) => void; onClose: () => void }) {
  const one = ORGANIZE[proposal.action];
  return (
    <div className="armada-logbook__card">
      <div className="armada-logbook__band armada-logbook__head-band">
        <span className="armada-logbook__head-kind">
          <one.Glyph size={12} strokeWidth={2} aria-hidden />
          {one.title}
        </span>
        <span className="armada-logbook__head-where">
          <span className="armada-logbook__acts">
            <Button variant="primary" size="sm" onClick={onApplyAll} disabled={proposal.rows.length === 0}>
              {one.all}
            </Button>
            <Tooltip label="Close">
              <Button variant="ghost" size="sm" iconOnly aria-label="Close" onClick={onClose}>
                <X size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        </span>
      </div>
      <ul className="armada-logbook__body armada-logbook__proposal" aria-label="Proposals">
        {proposal.rows.map((row) => (
          <li key={row.id} className="armada-logbook__proposed">
            <div className="armada-logbook__proposed-text">
              <span>{row.text}</span>
              {(row.absorbed ?? []).map((text) => (
                <span key={text} className="armada-logbook__proposed-absorbed">
                  {text}
                </span>
              ))}
              {row.note === "" ? null : <span className="armada-logbook__proposed-note">{row.note}</span>}
            </div>
            <Tooltip label="Dismiss">
              <Button variant="ghost" size="sm" iconOnly aria-label="Dismiss" onClick={() => onDismiss(row.id)}>
                <X size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </li>
        ))}
      </ul>
    </div>
  );
}

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
