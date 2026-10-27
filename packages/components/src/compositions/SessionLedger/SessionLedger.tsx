import { Children, isValidElement, useState, type ReactNode } from "react";
import { AppWindow, BellRing, Link, CircleCheck, CircleX, GitPullRequestArrow, LoaderCircle, CornerUpRight, SquarePen, MessageSquare, Check, CircleDot, Files, FileText, Globe, Image, ListChecks, Megaphone, MessageCircleQuestion, MoveRight, NotebookText, Power, Radar, Reply, ScanText, Search, Terminal, GitBranch, GitPullRequest, KeyRound, Presentation, PencilRuler, RotateCw, ShieldCheck, ShieldEllipsis, ShieldX, Split, SquareTerminal, Trash2, Unplug, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { forgetKept, readKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Inspect, InspectCard, type InspectKey } from "../../primitives/Inspect/Inspect";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { RunnableBlock, useCodeRun, type CodeRunner } from "../../primitives/Prose/RunnableBlock";
import { endedSaid } from "../../primitives/TerminalPane/TerminalPane";
import { QuestionDeck } from "../SessionThread/QuestionDeck";
import { LinkedSession, type LinkedSessionProps } from "../LinkedSession/LinkedSession";

/**
 * What a Session has accumulated, a section per kind and a row per thing.
 * **Every row is a press that opens what it names**: the slot on Cleanup, the
 * pull request in a sheet, the Studio on its whiteboard,
 * a sketch beside it, a page or a document at its address, a file on the
 * machine. The ledger draws no surface of its own; the host says where each
 * press goes.
 *
 * **A branch row ends in the chip of the slot it stands in**, so slots have no section of their own:
 * a slot no branch is on keeps a row of its own, drawn last under Branches with the key glyph (the
 * owner's choice of 16 Oct 2026, "slot chip on branch").
 *
 * **What waits on the person is not here.** The thread's Needs you band holds it, in every state of the
 * Session, done included (the owner's choice of 16 Oct 2026: "its already in the thread"); `LedgerRows`
 * and `waitingMark` are what that band draws it with.
 *
 * **A section is drawn only when it holds a row.** A ledger with nothing in it
 * draws one small picture and no words (the owner's choice of 7 Oct 2026, who
 * first asked for every section dim and then took it back: once some filled,
 * the empty ones only took room).
 */
export type LedgerKind = "waiting" | "slot" | "branch" | "pull_request" | "studio" | "sketch" | "subagent" | "background" | "artifact" | "fork" | "start" | "linked";

/** What an artifact is: a page published, a file written outside the code, a picture looked at, a Doc, or a page shown in a window. */
export type ArtifactForm = "page" | "file" | "image" | "doc" | "document" | "window";

/** What a background row is: a shell command the agent ran in the background, or a monitor it set to watch. */
export type BackgroundForm = "command" | "monitor";

/** A hue a ledger row or a card's fact carries: the Checks or the turn it stands for, never a chosen colour. */
export type SessionLedgerTone = "pending" | "passed" | "failed" | "running" | "done" | "escalated" | "piloted" | "stopped";

/**
 * What hovering or focusing a row shows beside it: what the row is, one fact a line. **Built by the host
 * from what it already holds**, so the card draws nothing the window would have to go and read.
 */
export type SessionLedgerCard = {
  title: string;
  /** The item's id, as the card's head says it: `#2301`. */
  where?: string;
  /** Its state, where the host names the glyph; else a fact labelled State is drawn as the state band. */
  state?: { said: string; Glyph?: LucideIcon; tone?: SessionLedgerTone };
  /**
   * `mono` is a machine value, a branch or a path. `tone` is drawn in the hue the row's mark takes.
   * A value of `{ at }` is an RFC 3339 instant, drawn as a stamp in the owner's time format. `open` draws the value as a link
   * to what it names, such as the Job that holds a pull request.
   */
  facts: readonly { label: string; value: string | { at: string }; tone?: SessionLedgerTone; mono?: boolean; open?: () => void }[];
};

import { PullRequestChecksMark, PullRequestStateMark, type LedgerPullRequest } from "./PullRequestMarks";

export { PullRequestChecksMark, PullRequestStateMark, type LedgerPullRequest };

export type LedgerEntry = {
  key: string;
  kind: LedgerKind;
  /** The row's accessible name: what it is, and for a pull request how its Checks stand. */
  name: string;
  text: ReactNode;
  /** A `pull_request` row's state and Checks, drawn as the two bare marks the done callout draws too. */
  pr?: LedgerPullRequest;
  /** A pull request's Checks, or a subagent's turn, as the one mark at the row's end. */
  mark?: { glyph: SessionLedgerTone; said: string };
  /** The preview a hover or a focus shows beside the row. Absent draws none. */
  card?: SessionLedgerCard;
  /** Which of the four an `artifact` row is, and so its glyph and what its tooltip names. */
  artifact?: ArtifactForm;
  /** An `artifact` file the agent showed and did not write: its glyph names it shown, not written. */
  shown?: true;
  /** Which of the two a `background` row is, and so its glyph and what its tooltip names. */
  background?: BackgroundForm;
  /** A `linked` row: a Session this one follows, drawn as a press that opens it. */
  linked?: LinkedSessionProps;
  /** The slot a branch stands in, as a chip at the row's end. */
  slot?: number;
  /**
   * Done with: a merged pull request, a finished subagent, an ended background command. Such a row is drawn only
   * under **All**, and a section that holds one gets the `Open | All` toggle.
   */
  finished?: boolean;
  /** A `waiting` row: what its press does, which is its glyph and what the glyph's tooltip names. */
  act?: WaitingActKind;
  /** A `waiting` row for a question with choices: drawn as the question deck, the row's name its question, `onOther` taking the words of Other. */
  options?: readonly { label: string; description?: string; onPick: () => void | Promise<unknown> }[];
  onOther?: (text: string) => void | Promise<unknown>;
  /**
   * A `waiting` row with no `act`: what the person types under it, sent as his answer. With this and no
   * `act` the row's press opens a reply box in place of doing something, and its glyph is Reply.
   */
  onReply?: (text: string) => void;
  /** A `waiting` row whose act is `run`, where the Session can run it: the row is the command's block with Run, as in the thread. */
  run?: { command: string; runner: CodeRunner; keep?: string };
  /**
   * A `waiting` row whose act is a restart onto a build (`buildChangeOf`): the row is one key that
   * switches it, with nothing to copy and no command to read. `command` is its tooltip. Without a
   * `runner` the key does nothing.
   */
  restart?: { command: string; runner?: CodeRunner; keep?: string };
  /** What the reply box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  replyKeep?: string;
  /** A `waiting` row Fleet read out of the agent's reply rather than the agent stating it: where it asks, its glyph says so in place of Reply or Question. */
  fromReply?: boolean;
  /** A row read from a reply that asks nothing: the person says so at its end, and Fleet keeps the sentence as one that is no question. */
  onMisread?: () => void;
  /** Marked as a question, beside `onMisread`: Fleet keeps the sentence as an ask. */
  onConfirmed?: () => void;
  /** Fleet holds the row as confirmed: it draws no thumbs, and its menu offers only Not a question. */
  confirmed?: boolean;
  /**
   * A `waiting` row the agent stated: the person takes it off, with a reason for the agent that may be empty.
   * The row then draws Remove at its end, and its press opens the reason box.
   */
  onRemove?: (reason: string) => void;
  /** What the reason box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  removeKeep?: string;
  /** Release at the row's end, shown on hover or focus, `name` what its tooltip names. Its press is the host's, which confirms first. */
  release?: { name: string; onPress: () => void };
  /** Released from here: given back, or kept by Fleet with `kept` saying why. */
  released?: { kept?: string };
  onOpen: () => void;
};

/** `message` is an item that waits on the person sending the message in the box. `steps` is a procedure, whose press opens its steps beside the thread. */
export type WaitingActKind = "walk" | "answer" | "approve_pr" | "run" | "message" | "steps";

const WAITING_ACT: Record<WaitingActKind | "none", { Glyph: LucideIcon; said: string }> = {
  walk: { Glyph: AppWindow, said: "Opens the walk" },
  answer: { Glyph: MessageSquare, said: "Goes to the question" },
  approve_pr: { Glyph: GitPullRequestArrow, said: "Opens the pull request" },
  run: { Glyph: Terminal, said: "Copies the command" },
  message: { Glyph: SquarePen, said: "Goes to the message" },
  steps: { Glyph: ListChecks, said: "Steps" },
  none: { Glyph: BellRing, said: "Waiting on you" },
};

const REPLY = { Glyph: Reply, said: "Reply" };

/** A question Fleet read out of the agent's words. */
const READ_FROM_REPLY = { Glyph: ScanText, said: "Read from the reply" };

/** A row with choices is a question, whatever its press does: one asked in the walk window still opens the walk. */
const QUESTION = { Glyph: MessageCircleQuestion, said: "Question" };

const SECTIONS: { kind: LedgerKind; label: string; Glyph: LucideIcon }[] = [
  { kind: "linked", label: "Linked", Glyph: Link },
  { kind: "pull_request", label: "Pull requests", Glyph: GitPullRequest },
  { kind: "branch", label: "Branches", Glyph: GitBranch },
  { kind: "studio", label: "Studios", Glyph: Presentation },
  { kind: "sketch", label: "Sketches", Glyph: PencilRuler },
  { kind: "subagent", label: "Subagents", Glyph: Split },
  { kind: "background", label: "Background", Glyph: SquareTerminal },
  { kind: "artifact", label: "Artifacts", Glyph: Files },
  { kind: "fork", label: "Forks", Glyph: MoveRight },
  { kind: "start", label: "Started", Glyph: CornerUpRight },
];

/** Also the glyphs of the rows a Session's done callout lists, so a page reads the same there as here. */
export const ARTIFACT: Record<ArtifactForm, { Glyph: LucideIcon; said: string }> = {
  page: { Glyph: Globe, said: "Published page" },
  file: { Glyph: Files, said: "File written" },
  image: { Glyph: Image, said: "Looked at" },
  doc: { Glyph: NotebookText, said: "Doc" },
  document: { Glyph: FileText, said: "Document" },
  window: { Glyph: AppWindow, said: "Shown in a window" },
};

export const FILE_SHOWN = "File shown";

/** The form an `artifact` row draws. */
export function artifactOf(row: LedgerEntry): { Glyph: LucideIcon; said: string } | undefined {
  if (row.artifact === undefined) return undefined;
  const form = ARTIFACT[row.artifact];
  return row.shown === true ? { ...form, said: FILE_SHOWN } : form;
}

/** The glyphs of a background row, and what each one's tooltip names. */
export const BACKGROUND: Record<BackgroundForm, { Glyph: LucideIcon; said: string }> = {
  command: { Glyph: SquareTerminal, said: "Background command" },
  monitor: { Glyph: Radar, said: "Monitor" },
};

const MARK: Record<NonNullable<LedgerEntry["mark"]>["glyph"], LucideIcon> = {
  pending: ShieldEllipsis,
  passed: ShieldCheck,
  failed: ShieldX,
  running: CircleDot,
  done: Check,
  escalated: Megaphone,
  piloted: Terminal,
  stopped: Power,
};

type Show = "open" | "all";

/** Which of a section's two views the viewer chose, kept per section. A viewer's convenience, so a blocked store only forgets it. */
function useShow(kind: LedgerKind): [Show, (next: Show) => void] {
  const key = `armada.session-ledger.show.${kind}`;
  const [show, set] = useState<Show>(() => {
    try {
      return localStorage.getItem(key) === "all" ? "all" : "open";
    } catch {
      return "open";
    }
  });
  return [
    show,
    (next) => {
      set(next);
      try {
        localStorage.setItem(key, next);
      } catch {
        // Not kept: the section is as it was left until the window closes.
      }
    },
  ];
}

function ShowToggle({ label, show, onShow }: { label: string; show: Show; onShow: (next: Show) => void }) {
  return (
    <div className="armada-session-ledger__show" role="radiogroup" aria-label={`${label} shown`}>
      {(["open", "all"] as const).map((one) => (
        <button key={one} type="button" role="radio" aria-checked={show === one} className="armada-session-ledger__show-option" onClick={() => onShow(one)}>
          {one === "open" ? "Open" : "All"}
        </button>
      ))}
    </div>
  );
}

/** The kinds the Artifacts head offers a toggle for, and what each toggle is called. */
const KINDS: { form: ArtifactForm; label: string }[] = [
  { form: "window", label: "Windows" },
  { form: "page", label: "Pages" },
  { form: "file", label: "Files" },
  { form: "image", label: "Pictures" },
  { form: "doc", label: "Docs" },
  { form: "document", label: "Documents" },
];

/** How many Artifacts rows show before More. */
const NEWEST = 5;

/**
 * Which artifact kinds the viewer lets through, everything but Pictures to begin with. A viewer's convenience, so a blocked store only forgets it.
 * The kinds hidden are kept, not the kinds shown, so a kind added later shows. A list of kinds shown from before is read once as what it hid.
 */
function useKinds(): [readonly ArtifactForm[], (next: readonly ArtifactForm[]) => void] {
  const key = "armada.session-ledger.kinds.artifact.hidden";
  const shownBefore = "armada.session-ledger.kinds.artifact";
  const forms = KINDS.map((one) => one.form);
  const [on, set] = useState<readonly ArtifactForm[]>(() => {
    try {
      const hidden: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
      if (Array.isArray(hidden)) return forms.filter((form) => !hidden.includes(form));
      const shown: unknown = JSON.parse(localStorage.getItem(shownBefore) ?? "null");
      if (Array.isArray(shown)) return forms.filter((form) => form === "document" || shown.includes(form));
      return forms.filter((form) => form !== "image");
    } catch {
      return forms.filter((form) => form !== "image");
    }
  });
  return [
    on,
    (next) => {
      set(next);
      try {
        localStorage.setItem(key, JSON.stringify(forms.filter((form) => !next.includes(form))));
      } catch {
        // Not kept: the filter is as it was left until the window closes.
      }
    },
  ];
}

/** A section's rows; Branches also holds the slots no branch is on, after the branches. */
function rowsOf(entries: readonly LedgerEntry[], kind: LedgerKind): LedgerEntry[] {
  const own = entries.filter((one) => one.kind === kind);
  return kind === "branch" ? [...own, ...entries.filter((one) => one.kind === "slot")] : own;
}

/** The kinds the bar narrows the ledger to, in the bar's order; each stands for one section. */
const BAR: { kind: LedgerKind; said: string; Glyph: LucideIcon }[] = [
  { kind: "pull_request", said: "Pull requests", Glyph: GitPullRequest },
  { kind: "branch", said: "Branches and slots", Glyph: GitBranch },
  { kind: "artifact", said: "Artifacts", Glyph: Files },
];

/** What a Session's Search draft is kept under, so it survives leaving the panel. */
const searchKeep = (keepKey: string | undefined) => (keepKey === undefined ? false : `session-ledger-search:${keepKey}`);

/** The words a node draws, for matching a search against a row's text. */
function plain(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(plain).join("");
  return isValidElement<{ children?: ReactNode }>(node) ? Children.toArray(node.props.children).map(plain).join("") : "";
}

/**
 * `offers` is the kinds the bar has a button for, all of them by default. **A host that shows a kind elsewhere
 * leaves it out**, so the bar has no button that can never have a row behind it (the Cockpit's Ledger panel
 * keeps the branch and the bay in its band).
 */
export function SessionLedger({ entries, folded = false, keepKey, offers }: { entries: readonly LedgerEntry[]; folded?: boolean; keepKey?: string; offers?: readonly LedgerKind[] }) {
  const [only, setOnly] = useState<LedgerKind>();
  const keep = searchKeep(keepKey);
  const [query, setQuery] = useState(() => (keep === false ? "" : (readKept<string>(keep) ?? "")));
  const [finding, setFinding] = useState(query !== "");
  // The choice lasts as long as this ledger does. A kind that has lost its last row cannot narrow anything, so it is not applied.
  const chosen = only !== undefined && rowsOf(entries, only).length > 0 ? only : undefined;
  const asked = query.trim().toLowerCase();
  const sections = SECTIONS.filter(({ kind }) => chosen === undefined || kind === chosen).map((section) => ({
    ...section,
    all: rowsOf(entries, section.kind).filter((row) => asked === "" || `${row.name} ${plain(row.text)}`.toLowerCase().includes(asked)),
  }));
  const stopFinding = () => {
    if (keep !== false) forgetKept(keep);
    setQuery("");
    setFinding(false);
  };
  return (
    <aside className="armada-session-ledger" role="region" aria-label="Attachments" data-folded={folded || undefined}>
      {SECTIONS.every(({ kind }) => rowsOf(entries, kind).length === 0) ? null : (
      <div className="armada-session-ledger__bar" role="group" aria-label="Narrow the ledger">
        <div className="armada-session-ledger__search" data-open={finding || undefined}>
          <Tooltip label="Search">
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Search"
              aria-pressed={finding}
              // Pressed again while open, it must not take the focus from the field first, or the blur would close it and this press open it again.
              onMouseDown={finding ? (event) => event.preventDefault() : undefined}
              onClick={() => (finding ? stopFinding() : setFinding(true))}
            >
              <Search size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          {finding ? (
            <div className="armada-session-ledger__find">
              <Textarea
                autoFocus
                rows={1}
                keep={keep}
                aria-label="Search the ledger"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                onBlur={() => {
                  if (query.trim() === "") stopFinding();
                }}
                onKeyDown={(event) => {
                  if (event.key === "Escape") {
                    event.preventDefault();
                    event.stopPropagation();
                    stopFinding();
                  } else if (event.key === "Enter") {
                    event.preventDefault();
                  }
                }}
              />
            </div>
          ) : null}
        </div>
        {BAR.filter(({ kind }) => offers === undefined || offers.includes(kind)).map(({ kind, said, Glyph }) => {
          const pressed = chosen === kind;
          return (
            <Tooltip key={kind} label={said}>
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={said}
                aria-pressed={pressed}
                disabled={rowsOf(entries, kind).length === 0}
                onClick={() => setOnly(pressed ? undefined : kind)}
              >
                <Glyph size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          );
        })}
      </div>
      )}
      {sections.every((one) => one.all.length === 0) ? <EmptyLedger /> : null}
      {sections.map(({ kind, label, Glyph, all }) => (
        <LedgerSection key={kind} kind={kind} label={label} Glyph={Glyph} all={all} />
      ))}
    </aside>
  );
}

function LedgerSection({ kind, label, Glyph, all }: { kind: LedgerKind; label: string; Glyph: LucideIcon; all: readonly LedgerEntry[] }) {
  const [show, setShow] = useShow(kind);
  const [kinds, setKinds] = useKinds();
  const [more, setMore] = useState(false);
  if (all.length === 0) return null;
  const toggles = all.some((one) => one.finished === true);
  const filtered = kind === "artifact";
  const present = KINDS.filter((one) => all.some((row) => row.artifact === one.form));
  const letThrough = toggles && show === "open" ? all.filter((one) => one.finished !== true) : all;
  const passed = filtered ? letThrough.filter((one) => one.artifact !== undefined && kinds.includes(one.artifact)) : letThrough;
  // The host gives rows newest first and an entry carries no time of its own, so its order stands.
  const rows = filtered && !more ? passed.slice(0, NEWEST) : passed;
  return (
    <section className="armada-session-ledger__group" aria-label={label}>
      <div className="armada-session-ledger__head">
        <h3 className="armada-session-ledger__eyebrow">{label}</h3>
        {toggles ? <ShowToggle label={label} show={show} onShow={setShow} /> : null}
        {filtered ? (
          <div className="armada-session-ledger__kinds" role="group" aria-label={`${label} shown`}>
            {present.map(({ form, label: said }) => {
              const Kind = ARTIFACT[form].Glyph;
              const pressed = kinds.includes(form);
              return (
                <Tooltip key={form} label={said} asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    iconOnly
                    aria-label={said}
                    aria-pressed={pressed}
                    onClick={() => setKinds(pressed ? kinds.filter((one) => one !== form) : [...kinds, form])}
                  >
                    <Kind size={12} strokeWidth={2} aria-hidden />
                  </Button>
                </Tooltip>
              );
            })}
          </div>
        ) : null}
      </div>
      {rows.length === 0 ? null : <LedgerRows kind={kind} Glyph={Glyph} rows={rows} />}
      {filtered && passed.length > NEWEST ? (
        <button type="button" className="armada-session-ledger__more" onClick={() => setMore(!more)}>
          {more ? "Less" : "More"}
        </button>
      ) : null}
    </section>
  );
}

/** What a waiting row's glyph is and what its tooltip names: Read from the reply where Fleet read a question out of one, Question where it has choices, Reply where its press opens a box, else its act's. */
export function waitingMark(row: LedgerEntry): { Glyph: LucideIcon; said: string } {
  const replies = row.act === undefined && row.onReply !== undefined;
  if (row.fromReply === true && (row.options !== undefined || replies)) return READ_FROM_REPLY;
  if (row.options !== undefined) return QUESTION;
  return replies ? REPLY : WAITING_ACT[row.act ?? "none"];
}

/** A section's rows, also drawn by the thread's Needs you panel so a waiting item reads and acts the same in both. */
export function LedgerRows({ kind, Glyph, rows, expanded }: { kind: LedgerKind; Glyph: LucideIcon; rows: readonly LedgerEntry[]; /** A reply row starts with its box open, for a row that was pressed to be opened. */ expanded?: true }) {
  const [replying, setReplying] = useState<string | undefined>(() => (expanded === undefined ? undefined : rows.find((one) => one.act === undefined && one.onReply !== undefined)?.key));
  const [removing, setRemoving] = useState<string>();
  // Remove at a row's end, and the reason box under it while it is open.
  const removal = (row: LedgerEntry) =>
    row.onRemove === undefined ? null : (
      <RemoveAct name={row.name} open={removing === row.key} onPress={() => setRemoving(removing === row.key ? undefined : row.key)} />
    );
  const reasoning = (row: LedgerEntry) =>
    removing === row.key && row.onRemove !== undefined ? <RemoveBox name={row.name} keep={row.removeKeep} onRemove={row.onRemove} onClose={() => setRemoving(undefined)} /> : null;
  return (
    <ul className="armada-session-ledger__rows">
      {rows.map((row) => {
        if (kind === "waiting" && row.restart !== undefined) return <RestartRow key={row.key} row={row} restart={row.restart} />;
        if (kind === "waiting" && row.act === "run" && row.run !== undefined) {
          const block = (
            <div>
              <RunnableBlock command={row.run.command} runner={row.run.runner} {...(row.run.keep === undefined ? {} : { keep: row.run.keep })}>
                <pre className="armada-prose__block">
                  <code>{row.run.command}</code>
                </pre>
              </RunnableBlock>
            </div>
          );
          return (
            <li key={row.key} className="armada-session-ledger__run" aria-label={row.name} data-removable={row.onRemove !== undefined || undefined}>
              {row.card === undefined ? block : <Inspect card={<LedgerPreview card={row.card} kind={row.kind} keys={[]} />} name={row.card.title} side="beside" pin={false}>{block}</Inspect>}
              {removal(row)}
              {reasoning(row)}
            </li>
          );
        }
        if (kind === "waiting" && row.options !== undefined) {
          const { options } = row;
          return (
            <li key={row.key} aria-label={row.name} className="armada-session-ledger__held" data-removable={row.onRemove !== undefined || undefined}>
              <QuestionDeck
                questions={[{ question: row.name, header: "", multi_select: false, options: options.map((one) => ({ label: one.label, description: one.description ?? "" })) }]}
                canSkip={false}
                flat
                onAnswer={(_, answers) => {
                  const chosen = answers?.[0]?.chosen[0];
                  if (chosen === undefined) return;
                  const at = options.findIndex((one) => one.label === chosen);
                  return at >= 0 ? options[at]!.onPick() : row.onOther?.(chosen);
                }}
              />
              {removal(row)}
              {reasoning(row)}
            </li>
          );
        }
        if (kind === "linked" && row.linked !== undefined) {
          return (
            <li key={row.key} className="armada-session-ledger__row" aria-label={row.name}>
              <LinkedSession {...row.linked} />
            </li>
          );
        }
        // A command that exited badly is no gate failing, so it takes the cross and not the shield.
        const Mark = row.mark === undefined ? undefined : row.kind === "background" && row.mark.glyph === "failed" ? X : MARK[row.mark.glyph];
        const replies = kind === "waiting" && row.act === undefined && row.onReply !== undefined;
        const waiting = kind === "waiting" ? waitingMark(row) : undefined;
        const form = row.artifact !== undefined ? artifactOf(row) : row.background !== undefined ? BACKGROUND[row.background] : undefined;
        const opener = (
          <button
            type="button"
            className="armada-session-ledger__open"
            aria-label={`Open ${row.name}`}
            aria-expanded={replies ? replying === row.key : undefined}
            onClick={replies ? () => setReplying(replying === row.key ? undefined : row.key) : row.onOpen}
          >
            {row.pr !== undefined ? (
              <PullRequestStateMark merged={row.pr.merged} />
            ) : waiting !== undefined ? (
              <Tooltip label={waiting.said}>
                <span className="armada-session-mark" role="img" aria-label={waiting.said}>
                  <waiting.Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : form === undefined ? (
              row.kind === "slot" ? <KeyRound size={12} strokeWidth={2} aria-hidden /> : <Glyph size={12} strokeWidth={2} aria-hidden />
            ) : (
              <Tooltip label={form.said}>
                <span className="armada-session-mark" role="img" aria-label={form.said}>
                  <form.Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            <span className="armada-session-ledger__text">{row.text}</span>
            {row.released === undefined || row.released.kept !== undefined ? null : (
              <Tooltip label="Given back">
                <span className="armada-session-mark" role="img" aria-label="Given back">
                  <Unplug size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            {row.slot === undefined ? null : (
              <span className="armada-ref-chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
                <KeyRound size={12} strokeWidth={2} aria-hidden />
                {row.slot}
              </span>
            )}
            {row.pr !== undefined ? (
              <PullRequestChecksMark checks={row.pr.checks} said={row.pr.checksSaid} />
            ) : Mark === undefined || row.mark === undefined ? null : (
              <Tooltip label={row.mark.said}>
                <span
                  className="armada-session-mark"
                  role="img"
                  aria-label={row.mark.said}
                  data-state-mark
                  data-pulsing={row.mark.glyph === "running" || undefined}
                >
                  <Mark size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
          </button>
        );
        return (
          <li
            key={row.key}
            className="armada-session-ledger__row"
            aria-label={row.name}
            data-mark={row.mark?.glyph}
            data-given={(row.released !== undefined && row.released.kept === undefined) || undefined}
          >
            {row.card === undefined ? (
              opener
            ) : (
              <Inspect card={<LedgerPreview card={row.card} kind={row.kind} />} name={row.card.title} side="beside" pin={false}>
                {opener}
              </Inspect>
            )}
            {removal(row)}
            {row.release === undefined ? null : <ReleaseAct name={row.release.name} onPress={row.release.onPress} />}
            {row.released?.kept === undefined ? null : <span className="armada-session-ledger__kept">{row.released.kept}</span>}
            {replies && replying === row.key && row.onReply !== undefined ? (
              <ReplyBox name={row.name} keep={row.replyKeep} onSend={row.onReply} onClose={() => setReplying(undefined)} />
            ) : null}
            {reasoning(row)}
          </li>
        );
      })}
    </ul>
  );
}

/**
 * A restart's row: its label and one mark, the press the key. Nothing starts it again while it runs or
 * once it has passed and the row is about to go; a failure leaves the key to press again.
 */
function RestartRow({ row, restart }: { row: LedgerEntry; restart: NonNullable<LedgerEntry["restart"]> }) {
  return (
    <li className="armada-session-ledger__row" aria-label={row.name}>
      <RestartKey className="armada-session-ledger__open" name={row.name} restart={restart} text={row.text} />
    </li>
  );
}

/** A restart's key alone, its mark and `text`, as a list row draws it and as the Needs you band does. */
export function RestartKey({ className, name, restart, text = name }: { className: string; name: string; restart: NonNullable<LedgerEntry["restart"]>; text?: ReactNode }) {
  const run = useCodeRun(restart.runner, restart.keep);
  const { phase } = run;
  const held = restart.runner === undefined || phase.state === "running" || (phase.state === "ended" && phase.code === 0);
  const mark =
    phase.state === "running"
      ? { Glyph: LoaderCircle, said: "Running", state: "running" }
      : phase.state === "ended"
        ? phase.code === 0
          ? { Glyph: CircleCheck, said: "Passed", state: "passed" }
          : { Glyph: CircleX, said: endedSaid(phase), state: "failed" }
        : { Glyph: RotateCw, said: restart.command, state: undefined };
  return (
    <Tooltip label={mark.said} asChild>
      <button type="button" className={className} aria-label={name} aria-disabled={held || undefined} onClick={() => (held ? undefined : run.start(restart.command))}>
        <span className="armada-session-mark" data-restart={mark.state} role="img" aria-label={mark.said}>
          <mark.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
        <span className="armada-session-ledger__text">{text}</span>
      </button>
    </Tooltip>
  );
}

const KIND: Record<LedgerKind, { Glyph: LucideIcon; label: string }> = {
  waiting: { Glyph: BellRing, label: "Waiting" },
  slot: { Glyph: KeyRound, label: "Slot" },
  branch: { Glyph: GitBranch, label: "Branch" },
  pull_request: { Glyph: GitPullRequest, label: "Pull request" },
  studio: { Glyph: Presentation, label: "Studio" },
  sketch: { Glyph: PencilRuler, label: "Sketch" },
  subagent: { Glyph: Split, label: "Subagent" },
  background: { Glyph: SquareTerminal, label: "Background" },
  artifact: { Glyph: Files, label: "Artifact" },
  fork: { Glyph: MoveRight, label: "Fork" },
  start: { Glyph: CornerUpRight, label: "Started" },
  linked: { Glyph: Link, label: "Linked" },
};

/** The status token a card's tone draws in: the same ones a row's mark takes. */
const TONE_COLOR: Record<SessionLedgerTone, string> = {
  pending: "var(--status-awaiting-review)",
  passed: "var(--status-completed-success)",
  failed: "var(--status-completed-failed)",
  running: "var(--status-running)",
  piloted: "var(--status-running)",
  escalated: "var(--status-escalated)",
  done: "var(--fg-muted)",
  stopped: "var(--fg-muted)",
};

/** What a row's card keys say: a row's press opens it. */
const OPENS: readonly InspectKey[] = [{ key: "Enter", verb: "Open" }];

/** A row's preview as the inspect card: its kind, its name, its state, then one fact a line. */
export function LedgerPreview({ card, kind, keys = OPENS }: { card: SessionLedgerCard; kind: LedgerKind; keys?: readonly InspectKey[] }) {
  const held = card.state ?? (() => {
    const one = card.facts.find((fact) => fact.label === "State");
    return one === undefined || typeof one.value !== "string" ? undefined : { said: one.value, ...(one.tone === undefined ? {} : { tone: one.tone }) };
  })();
  const Glyph = card.state?.Glyph ?? (held?.tone === undefined ? undefined : MARK[held.tone]);
  return (
    <InspectCard
      kind={KIND[kind]}
      {...(card.where === undefined ? {} : { where: card.where })}
      title={card.title}
      {...(held === undefined ? {} : { state: { ...(Glyph === undefined ? {} : { Glyph }), said: held.said, ...(held.tone === undefined ? {} : { tone: TONE_COLOR[held.tone] }) } })}
      facts={card.facts.filter((fact) => card.state !== undefined || fact.label !== "State").map((fact) => ({ label: fact.label, value: typeof fact.value === "string" ? fact.value : <Stamp at={fact.value.at} />, ...(fact.mono === true ? { mono: true } : {}), ...(fact.tone === undefined ? {} : { tone: TONE_COLOR[fact.tone] }), ...(fact.open === undefined ? {} : { open: fact.open }) }))}
      keys={keys}
      width="wide"
    />
  );
}

/**
 * The box under a waiting row that has no act: a textarea and Send. Enter sends, Shift+Enter breaks the
 * line, Escape closes and keeps the draft; a send closes it and clears the draft.
 */
function ReplyBox({ name, keep, onSend, onClose }: { name: string; keep: string | undefined; onSend: (text: string) => void; onClose: () => void }) {
  const [text, setText] = useState("");
  const send = () => {
    const said = text.trim();
    if (said === "") return;
    onSend(said);
    if (keep !== undefined) forgetKept(keep);
    onClose();
  };
  return (
    <div className="armada-session-ledger__reply">
      <Textarea
        autoFocus
        rows={2}
        keep={keep ?? false}
        aria-label={`Reply to ${name}`}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          } else if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={send}>
        Send
      </Button>
    </div>
  );
}

/** Takes an item the agent stated off the list. Its tooltip names it; the reason box opens under the row. */
export function RemoveAct({ name, open, onPress }: { name: string; open: boolean; onPress: () => void }) {
  return (
    <Tooltip label="Remove">
      <Button variant="ghost" size="sm" iconOnly aria-label={`Remove ${name}`} aria-expanded={open} onClick={onPress}>
        <Trash2 size={12} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
}

/** Gives one row back. The row's hover or focus shows it; its tooltip names the row. */
function ReleaseAct({ name, onPress }: { name: string; onPress: () => void }) {
  return (
    <span className="armada-session-ledger__release">
      <Tooltip label={`Release ${name}`}>
        <Button variant="ghost" size="sm" iconOnly aria-label={`Release ${name}`} onClick={onPress}>
          <Unplug size={12} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </span>
  );
}

/**
 * The reason box under a removed item: one line, whose words go to the agent. **Enter removes it, and an
 * empty reason is fine.** Esc puts the box away and removes nothing; what was typed is kept for next time.
 */
export function RemoveBox({ name, keep, onRemove, onClose }: { name: string; keep: string | undefined; onRemove: (reason: string) => void; onClose: () => void }) {
  const [text, setText] = useState("");
  const remove = () => {
    onRemove(text.trim());
    if (keep !== undefined) forgetKept(keep);
    onClose();
  };
  return (
    <div className="armada-session-ledger__remove">
      <Textarea
        autoFocus
        rows={1}
        keep={keep ?? false}
        aria-label={`Reason for removing ${name}`}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onClose();
          } else if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            remove();
          }
        }}
      />
    </div>
  );
}

/**
 * A ledger with nothing on it: a small picture of one, in the border tokens, and no words under it.
 * Three ruled rows, the last one dashed for what has not come yet. The merge line's empty picture
 * is the precedent for an illustration, and the owner's the licence (`iconography.md`).
 */
function EmptyLedger() {
  return (
    <svg className="armada-session-ledger__picture" viewBox="0 0 96 40" role="img" aria-label="Nothing attached">
      <circle className="armada-session-ledger__picture-place" cx="10" cy="8" r="3.5" />
      <path d="M22 8 H84" />
      <circle className="armada-session-ledger__picture-place" cx="10" cy="20" r="3.5" />
      <path d="M22 20 H66" />
      <circle className="armada-session-ledger__picture-place armada-session-ledger__picture-later" cx="10" cy="32" r="3.5" />
      <path className="armada-session-ledger__picture-later" d="M22 32 H76" />
    </svg>
  );
}
