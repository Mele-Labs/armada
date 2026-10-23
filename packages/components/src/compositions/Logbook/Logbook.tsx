import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import {
  BookDashed,
  BookMarked,
  BookPlus,
  CircleAlert,
  Container,
  Pin,
  Repeat,
  Ruler,
  SquareChevronRight,
  TrafficCone,
  type LucideIcon,
} from "lucide-react";
import type { LogbookEntry, LogbookKind, LogbookState } from "@armada/protocol";

import { keyFor } from "../../actions";
import { forgetKept, readKept } from "../../keep";
import { isPressed, pressedSlot } from "../../keymap";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";

/**
 * The Logbook: what Armada learned from every Job, read when the owner is curious. `docs/concepts/logbook.md`.
 *
 * **Nothing here asks him for anything.** He reads an entry and its evidence, and can edit, pin or forget it. Every
 * repository's Logbook is drawn at once, each in its own panel, then Habits and This Mac. Candidates and faded entries
 * are behind the filter, never mixed in with what agents are handed.
 *
 * The page is keyed by the registry's acts: the arrows move, `Enter` opens the entry, `e`, `p` and `f` edit, pin and
 * forget it, and `Esc` backs out a level.
 */

/** One panel: a repository's Logbook, Habits, or This Mac. */
export type LogbookPanel = { key: string; title: string; entries: readonly LogbookEntry[] };

export type LogbookViewProps = {
  panels: readonly LogbookPanel[];
  /** Which entries the list shows. */
  filter: LogbookState;
  onFilter: (filter: LogbookState) => void;
  /** The entry open on the right. */
  selected?: string;
  onSelect: (id: string) => void;
  /** Its text is open in place. */
  editing: boolean;
  onEdit: (editing: boolean) => void;
  onSave: (id: string, text: string) => void;
  onPin: (id: string, pinned: boolean) => void;
  onForget: (id: string) => void;
  /** Opens the Job an evidence row came from. */
  onOpenJob?: (jobId: string) => void;
  now: number;
};

const KIND: Record<LogbookKind, { Glyph: LucideIcon; said: string }> = {
  command: { Glyph: SquareChevronRight, said: "Command" },
  gotcha: { Glyph: TrafficCone, said: "Gotcha" },
  convention: { Glyph: Ruler, said: "Convention" },
  environment: { Glyph: Container, said: "Environment" },
  habit: { Glyph: Repeat, said: "Habit" },
};

const STATE: Record<LogbookState, { Glyph: LucideIcon; said: string; tab: string }> = {
  standing: { Glyph: BookMarked, said: "Standing", tab: "Standing" },
  candidate: { Glyph: BookPlus, said: "Candidate", tab: "Candidates" },
  faded: { Glyph: BookDashed, said: "Faded", tab: "Faded" },
};

const FILTERS: readonly LogbookState[] = ["standing", "candidate", "faded"];

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
      {entry.pinned ? <Mark Glyph={Pin} said="Pinned" /> : null}
      {withState ? <Mark Glyph={state.Glyph} said={state.said} /> : null}
      <Mark Glyph={kind.Glyph} said={kind.said} />
    </>
  );
}

/** A time drawn as its age, with the date on hover. */
function When({ at, now }: { at: string; now: number }) {
  return (
    <Tooltip label={dated(at)}>
      <time dateTime={at}>{ago(at, now)}</time>
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

export function LogbookView({ panels, filter, onFilter, selected, onSelect, editing, onEdit, onSave, onPin, onForget, onOpenJob, now }: LogbookViewProps) {
  const list = useRef<HTMLDivElement>(null);
  const detail = useRef<HTMLElement>(null);
  const [forgetting, setForgetting] = useState<string>();
  const order = panels.flatMap((panel) => panel.entries);
  const open = order.find((one) => one.id === selected);
  const panelOf = (id: string) => panels.find((panel) => panel.entries.some((one) => one.id === id));

  // The selected row is kept in view as the arrows move it.
  useEffect(() => {
    if (selected !== undefined) document.getElementById(optionId(selected))?.scrollIntoView({ block: "nearest" });
  }, [selected]);

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
    if (open === undefined) return;
    if (isPressed("logbook_edit", event)) {
      event.preventDefault();
      onEdit(true);
    } else if (isPressed("logbook_pin", event)) {
      event.preventDefault();
      onPin(open.id, !open.pinned);
    } else if (isPressed("logbook_forget", event)) {
      event.preventDefault();
      setForgetting(open.id);
    }
  }

  return (
    <div className="armada-logbook" onKeyDown={pressed}>
      <div className="armada-logbook__filters">
        <Tabs items={FILTERS.map((one) => ({ id: one, label: STATE[one].tab }))} value={filter} onChange={(id) => onFilter(id as LogbookState)} />
      </div>
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
              <div className="armada-logbook__band" id={`logbook-panel-${panel.key}`}>
                {panel.title}
              </div>
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
              onEdit={onEdit}
              onSave={(text) => {
                onSave(open.id, text);
                window.requestAnimationFrame(() => detail.current?.focus());
              }}
              onPin={() => onPin(open.id, !open.pinned)}
              onForget={() => setForgetting(open.id)}
              {...(onOpenJob === undefined ? {} : { onOpenJob })}
            />
          )}
        </section>
      </div>
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
  onEdit,
  onSave,
  onPin,
  onForget,
  onOpenJob,
}: {
  entry: LogbookEntry;
  where: string;
  editing: boolean;
  now: number;
  onEdit: (editing: boolean) => void;
  onSave: (text: string) => void;
  onPin: () => void;
  onForget: () => void;
  onOpenJob?: (jobId: string) => void;
}) {
  const key = draftKey(entry.id);
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
      <div className="armada-logbook__body">
        <div className="armada-logbook__marks">{marksOf(entry, now, true)}</div>
        {editing ? (
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
            <div className="armada-logbook__acts">
              <Act label="Edit" act="logbook_edit" onPress={() => onEdit(true)} />
              <Act label={entry.pinned ? "Unpin" : "Pin"} act="logbook_pin" onPress={onPin} />
              <Act label="Forget" act="logbook_forget" onPress={onForget} />
            </div>
          </>
        )}
        <dl className="armada-logbook__facts">
          <dt>Confirmed by</dt>
          <dd>{entry.confirmed_by === 1 ? "1 Job" : `${entry.confirmed_by} Jobs`}</dd>
          <dt>Last confirmed</dt>
          <dd>
            <When at={entry.last_confirmed} now={now} />
          </dd>
          <dt>First seen</dt>
          <dd>
            <When at={entry.first_seen} now={now} />
          </dd>
          {entry.last_served === undefined ? null : (
            <>
              <dt>Last served</dt>
              <dd>
                <When at={entry.last_served} now={now} />
              </dd>
            </>
          )}
        </dl>
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
              {entry.evidence.map((row) => (
                <li key={`${row.job_id}:${row.cite}`}>
                  <span className="armada-logbook__from">
                    {onOpenJob === undefined ? (
                      <span>{row.job_title}</span>
                    ) : (
                      <button type="button" className="armada-logbook__job" onClick={() => onOpenJob(row.job_id)}>
                        {row.job_title}
                      </button>
                    )}
                    <span className="armada-logbook__cite">{row.cite}</span>
                    <When at={row.at} now={now} />
                  </span>
                  <span className="armada-logbook__says">
                    <Words text={row.says} />
                  </span>
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
                <li key={one.until}>
                  <span className="armada-logbook__says">
                    <Words text={one.text} />
                  </span>
                  <span className="armada-logbook__from">
                    {one.by === "person" ? "Replaced by you" : "Rewritten by Armada"} <When at={one.until} now={now} />
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
function Act({ label, act, onPress }: { label: string; act: string; onPress: () => void }) {
  const key = keyFor(act);
  return (
    <Button variant={act === "logbook_forget" ? "ghost" : "secondary"} size="sm" onClick={onPress} aria-keyshortcuts={key === "" ? undefined : key}>
      {label}
      {key === "" ? null : <Kbd aria-hidden>{key}</Kbd>}
    </Button>
  );
}
