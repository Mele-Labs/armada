import { Children, isValidElement, useState, type ReactNode } from "react";
import { AppWindow, BellRing, Box, CircleCheck, CircleX, GitMerge, GitPullRequestArrow, LoaderCircle, CornerUpRight, SquarePen, MessageSquare, Check, CircleDot, Files, Globe, Hand, Image, ListChecks, Megaphone, MoveRight, NotebookText, Reply, Search, Terminal, GitBranch, GitPullRequest, KeyRound, Presentation, PencilRuler, RotateCw, ShieldCheck, ShieldEllipsis, ShieldX, Split, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { forgetKept, readKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { RunnableBlock, useCodeRun, type CodeRunner } from "../../primitives/Prose/RunnableBlock";
import { QuestionDeck } from "../SessionThread/QuestionDeck";

/**
 * What a Session has accumulated, a section per kind and a row per thing.
 * **Every row is a press that opens what it names**: the slot on Cleanup, the
 * pull request in a sheet, the Job in its detail, the Studio on its whiteboard,
 * a sketch beside it, a page or a document at its address, a file on the
 * machine. The ledger draws no surface of its own; the host says where each
 * press goes.
 *
 * **A branch row ends in the chip of the slot it stands in**, so slots have no section of their own:
 * a slot no branch is on keeps a row of its own, drawn last under Branches with the key glyph (the
 * owner's choice of 16 Oct 2026, "slot chip on branch").
 *
 * **A section is drawn only when it holds a row.** A ledger with nothing in it
 * draws one small picture and no words (the owner's choice of 7 Oct 2026, who
 * first asked for every section dim and then took it back: once some filled,
 * the empty ones only took room).
 */
export type LedgerKind = "waiting" | "slot" | "branch" | "pull_request" | "job" | "studio" | "sketch" | "subagent" | "artifact" | "fork" | "start";

/** What an artifact is: a page published, a file written outside the code, a picture looked at, a Doc, or a page shown in a window. */
export type ArtifactForm = "page" | "file" | "image" | "doc" | "window";

/** A hue a ledger row or a card's fact carries: the Checks or the turn it stands for, never a chosen colour. */
export type SessionLedgerTone = "pending" | "passed" | "failed" | "running" | "done" | "escalated" | "piloted";

/**
 * What hovering or focusing a row shows beside it: what the row is, one fact a line. **Built by the host
 * from what it already holds**, so the card draws nothing the window would have to go and read.
 */
export type SessionLedgerCard = {
  title: string;
  /** `mono` is a machine value, a branch or a path. `tone` is drawn in the hue the row's mark takes. */
  facts: readonly { label: string; value: string; tone?: SessionLedgerTone; mono?: boolean }[];
};

export type LedgerPullRequest = { merged: boolean; checks: "passed" | "running" | "failed"; checksSaid: string };

const PR_CHECKS = { passed: CircleCheck, running: LoaderCircle, failed: CircleX } as const;

/** A pull request's state as one bare mark: merged or open, in its hue, its tooltip naming it. */
export function PullRequestStateMark({ merged }: { merged: boolean }) {
  const said = merged ? "Merged" : "Open";
  return (
    <Tooltip label={said}>
      <span className="armada-pr-state" data-state={merged ? "merged" : "open"} role="img" aria-label={said}>
        {merged ? <GitMerge size={12} strokeWidth={2} aria-hidden /> : <GitPullRequest size={12} strokeWidth={2} aria-hidden />}
      </span>
    </Tooltip>
  );
}

/** A pull request's Checks as one bare mark, the PR card's: spinning while they run. */
export function PullRequestChecksMark({ checks, said }: { checks: LedgerPullRequest["checks"]; said: string }) {
  const Glyph = PR_CHECKS[checks];
  return (
    <Tooltip label={said}>
      <span className="armada-pr-card__check" data-checks={checks} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

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
  /** A Job the person tagged, which the Session is looking at and did not dispatch. */
  looking?: boolean;
  /** Came with a Job a person piloted: handed over, not leased. */
  handed?: string;
  /** A Job this Session is piloting: its exits are drawn in the row. */
  exits?: ReactNode;
  /** The slot a Job or a branch stands in, as a chip at the row's end. */
  slot?: number;
  /**
   * Done with: a merged pull request, an ended Job, a finished subagent. Such a row is drawn only
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
  run?: { command: string; runner: CodeRunner };
  /**
   * A `waiting` row whose act is a restart onto a build (`buildChangeOf`): the row is one key that
   * switches it, with nothing to copy and no command to read. `command` is its tooltip. Without a
   * `runner` the key does nothing.
   */
  restart?: { command: string; runner?: CodeRunner };
  /** What the reply box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  replyKeep?: string;
  /**
   * A `waiting` row the agent stated: the person takes it off, with a reason for the agent that may be empty.
   * The row then draws Remove at its end, and its press opens the reason box.
   */
  onRemove?: (reason: string) => void;
  /** What the reason box's draft is about (the Session and the item), so it survives closing the box and leaving the panel. */
  removeKeep?: string;
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

const SECTIONS: { kind: LedgerKind; label: string; Glyph: LucideIcon }[] = [
  { kind: "waiting", label: "Waiting on you", Glyph: BellRing },
  { kind: "pull_request", label: "Pull requests", Glyph: GitPullRequest },
  { kind: "branch", label: "Branches", Glyph: GitBranch },
  { kind: "job", label: "Jobs", Glyph: Box },
  { kind: "studio", label: "Studios", Glyph: Presentation },
  { kind: "sketch", label: "Sketches", Glyph: PencilRuler },
  { kind: "subagent", label: "Subagents", Glyph: Split },
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
  window: { Glyph: AppWindow, said: "Shown in a window" },
};

const MARK: Record<NonNullable<LedgerEntry["mark"]>["glyph"], LucideIcon> = {
  pending: ShieldEllipsis,
  passed: ShieldCheck,
  failed: ShieldX,
  running: CircleDot,
  done: Check,
  escalated: Megaphone,
  piloted: Terminal,
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
];

/** How many Artifacts rows show before More. */
const NEWEST = 5;

/** Which artifact kinds the viewer lets through, everything but Pictures to begin with. A viewer's convenience, so a blocked store only forgets it. */
function useKinds(): [readonly ArtifactForm[], (next: readonly ArtifactForm[]) => void] {
  const key = "armada.session-ledger.kinds.artifact";
  const fallback: ArtifactForm[] = ["window", "page", "file", "doc"];
  const [on, set] = useState<readonly ArtifactForm[]>(() => {
    try {
      const kept: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
      return Array.isArray(kept) ? KINDS.map((one) => one.form).filter((form) => kept.includes(form)) : fallback;
    } catch {
      return fallback;
    }
  });
  return [
    on,
    (next) => {
      set(next);
      try {
        localStorage.setItem(key, JSON.stringify(next));
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
  { kind: "waiting", said: "Calls", Glyph: BellRing },
  { kind: "branch", said: "Branches and slots", Glyph: GitBranch },
  { kind: "artifact", said: "Artifacts", Glyph: Files },
];

/** What the Search field's draft is kept under: a viewer's one search, which survives leaving the panel. */
const SEARCH_KEEP = "session-ledger-search";

/** Which kind the bar narrows to, kept like a section's view. A viewer's convenience, so a blocked store only forgets it. */
function useOnly(): [LedgerKind | undefined, (next: LedgerKind | undefined) => void] {
  const key = "armada.session-ledger.only";
  const [only, set] = useState<LedgerKind | undefined>(() => {
    try {
      return BAR.find((one) => one.kind === localStorage.getItem(key))?.kind;
    } catch {
      return undefined;
    }
  });
  return [
    only,
    (next) => {
      set(next);
      try {
        if (next === undefined) localStorage.removeItem(key);
        else localStorage.setItem(key, next);
      } catch {
        // Not kept: the bar is as it was left until the window closes.
      }
    },
  ];
}

/** The words a node draws, for matching a search against a row's text. */
function plain(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(plain).join("");
  return isValidElement<{ children?: ReactNode }>(node) ? Children.toArray(node.props.children).map(plain).join("") : "";
}

export function SessionLedger({ entries, folded = false }: { entries: readonly LedgerEntry[]; folded?: boolean }) {
  const [only, setOnly] = useOnly();
  const [query, setQuery] = useState(() => readKept<string>(SEARCH_KEEP) ?? "");
  const [finding, setFinding] = useState(query !== "");
  // A kind the Session holds no row of cannot narrow anything, so a choice kept from another Session is not applied.
  const chosen = only !== undefined && rowsOf(entries, only).length > 0 ? only : undefined;
  const asked = query.trim().toLowerCase();
  const sections = SECTIONS.filter(({ kind }) => chosen === undefined || kind === chosen).map((section) => ({
    ...section,
    all: rowsOf(entries, section.kind).filter((row) => asked === "" || `${row.name} ${plain(row.text)}`.toLowerCase().includes(asked)),
  }));
  const stopFinding = () => {
    forgetKept(SEARCH_KEEP);
    setQuery("");
    setFinding(false);
  };
  return (
    <aside className="armada-session-ledger" role="region" aria-label="Attachments" data-folded={folded || undefined}>
      <div className="armada-session-ledger__bar" role="group" aria-label="Narrow the ledger">
        <Tooltip label="Search">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Search"
            aria-pressed={finding}
            disabled={entries.length === 0}
            onClick={() => (finding ? stopFinding() : setFinding(true))}
          >
            <Search size={12} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        {BAR.map(({ kind, said, Glyph }) => {
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
        {finding ? (
          <div className="armada-session-ledger__find">
            <Textarea
              autoFocus
              rows={1}
              keep={SEARCH_KEEP}
              aria-label="Search the ledger"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
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

/** What a waiting row's glyph is and what its tooltip names: Reply where its press opens a box, else its act's. */
export function waitingMark(row: LedgerEntry): { Glyph: LucideIcon; said: string } {
  return row.act === undefined && row.onReply !== undefined ? REPLY : WAITING_ACT[row.act ?? "none"];
}

/** A section's rows, also drawn by the thread's Needs you panel so a waiting item reads and acts the same in both. */
export function LedgerRows({ kind, Glyph, rows }: { kind: LedgerKind; Glyph: LucideIcon; rows: readonly LedgerEntry[] }) {
  const [replying, setReplying] = useState<string>();
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
              <RunnableBlock command={row.run.command} runner={row.run.runner}>
                <pre className="armada-prose__block">
                  <code>{row.run.command}</code>
                </pre>
              </RunnableBlock>
            </div>
          );
          return (
            <li key={row.key} className="armada-session-ledger__run" aria-label={row.name} data-removable={row.onRemove !== undefined || undefined}>
              {row.card === undefined ? block : <Tooltip label={<LedgerPreview card={row.card} />} card beside asChild>{block}</Tooltip>}
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
        const Mark = row.mark === undefined ? undefined : MARK[row.mark.glyph];
        const replies = kind === "waiting" && row.act === undefined && row.onReply !== undefined;
        const waiting = kind === "waiting" ? waitingMark(row) : undefined;
        const form = row.artifact === undefined ? undefined : ARTIFACT[row.artifact];
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
            {row.handed === undefined ? null : (
              <Tooltip label={row.handed}>
                <span className="armada-session-mark" role="img" aria-label={row.handed}>
                  <Hand size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            {row.looking !== true ? null : (
              <Tooltip label="Looking at it, not dispatched from here">
                <span className="armada-session-mark" role="img" aria-label="Looking at it, not dispatched from here">
                  <Search size={12} strokeWidth={2} aria-hidden />
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
          <li key={row.key} className="armada-session-ledger__row" aria-label={row.name} data-mark={row.mark?.glyph}>
            {row.card === undefined ? (
              opener
            ) : (
              <Tooltip label={<LedgerPreview card={row.card} />} card beside asChild>
                {opener}
              </Tooltip>
            )}
            {removal(row)}
            {row.exits}
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
  const run = useCodeRun(restart.runner);
  const { phase } = run;
  const held = restart.runner === undefined || phase.state === "running" || (phase.state === "ended" && phase.code === 0);
  const mark =
    phase.state === "running"
      ? { Glyph: LoaderCircle, said: "Running", state: "running" }
      : phase.state === "ended"
        ? phase.code === 0
          ? { Glyph: CircleCheck, said: "Passed", state: "passed" }
          : { Glyph: CircleX, said: phase.code === null ? "Stopped" : `Exit ${phase.code}`, state: "failed" }
        : { Glyph: RotateCw, said: restart.command, state: undefined };
  return (
    <li className="armada-session-ledger__row" aria-label={row.name}>
      <Tooltip label={mark.said} asChild>
        <button
          type="button"
          className="armada-session-ledger__open"
          aria-label={row.name}
          aria-disabled={held || undefined}
          onClick={() => (held ? undefined : run.start(restart.command))}
        >
          <span className="armada-session-mark" data-restart={mark.state} role="img" aria-label={mark.said}>
            <mark.Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
          <span className="armada-session-ledger__text">{row.text}</span>
        </button>
      </Tooltip>
    </li>
  );
}

/** A row's preview: its title, then one fact a line, each labelled. Spans, because it is drawn inside the row's press. */
export function LedgerPreview({ card }: { card: SessionLedgerCard }) {
  return (
    <span className="armada-ledger-card">
      <span className="armada-ledger-card__title">{card.title}</span>
      {card.facts.map((fact) => (
        <span key={fact.label} className="armada-ledger-card__fact" data-tone={fact.tone}>
          <span className="armada-ledger-card__label">{fact.label}</span>
          <span className="armada-ledger-card__value" data-mono={fact.mono || undefined}>
            {fact.value}
          </span>
        </span>
      ))}
    </span>
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
