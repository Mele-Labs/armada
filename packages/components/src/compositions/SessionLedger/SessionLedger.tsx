import { useState, type ReactNode } from "react";
import { AppWindow, Box, Check, CircleDot, Files, Globe, Hand, Image, Megaphone, MoveRight, NotebookText, Search, Terminal, GitBranch, GitPullRequest, KeyRound, Presentation, PencilRuler, ShieldCheck, ShieldEllipsis, ShieldX, Split } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a Session has accumulated, a section per kind and a row per thing.
 * **Every row is a press that opens what it names**: the slot on Cleanup, the
 * pull request in a sheet, the Job in its detail, the Studio on its whiteboard,
 * a sketch beside it, a page or a document at its address, a file on the
 * machine. The ledger draws no surface of its own; the host says where each
 * press goes.
 *
 * **A section is drawn only when it holds a row.** A ledger with nothing in it
 * draws one small picture and no words (the owner's choice of 7 Oct 2026, who
 * first asked for every section dim and then took it back: once some filled,
 * the empty ones only took room).
 */
export type LedgerKind = "slot" | "branch" | "pull_request" | "job" | "studio" | "sketch" | "subagent" | "artifact" | "fork";

/** What an artifact is: a page published, a file written outside the code, a picture looked at, a Doc, or a page shown in a window. */
export type ArtifactForm = "page" | "file" | "image" | "doc" | "window";

export type LedgerEntry = {
  key: string;
  kind: LedgerKind;
  /** The row's accessible name: what it is, and for a pull request how its Checks stand. */
  name: string;
  text: ReactNode;
  /** A pull request's Checks, or a subagent's turn, as the one mark at the row's end. */
  mark?: { glyph: "pending" | "passed" | "failed" | "running" | "done" | "escalated" | "piloted"; said: string };
  /** Which of the four an `artifact` row is, and so its glyph and what its tooltip names. */
  artifact?: ArtifactForm;
  /** A Job the person tagged, which the Session is looking at and did not dispatch. */
  looking?: boolean;
  /** Came with a Job a person piloted: handed over, not leased. */
  handed?: string;
  /** A Job this Session is piloting: its exits are drawn in the row. */
  exits?: ReactNode;
  /** A Job's own slot, as a chip at the row's end. */
  slot?: number;
  /**
   * Done with: a merged pull request, an ended Job, a finished subagent. Such a row is drawn only
   * under **All**, and a section that holds one gets the `Open | All` toggle.
   */
  finished?: boolean;
  onOpen: () => void;
};

const SECTIONS: { kind: LedgerKind; label: string; Glyph: LucideIcon }[] = [
  { kind: "slot", label: "Worktree Slot", Glyph: KeyRound },
  { kind: "branch", label: "Branches", Glyph: GitBranch },
  { kind: "pull_request", label: "Pull requests", Glyph: GitPullRequest },
  { kind: "job", label: "Jobs", Glyph: Box },
  { kind: "studio", label: "Studios", Glyph: Presentation },
  { kind: "sketch", label: "Sketches", Glyph: PencilRuler },
  { kind: "subagent", label: "Subagents", Glyph: Split },
  { kind: "artifact", label: "Artifacts", Glyph: Files },
  { kind: "fork", label: "Forks", Glyph: MoveRight },
];

const ARTIFACT: Record<ArtifactForm, { Glyph: LucideIcon; said: string }> = {
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

export function SessionLedger({ entries, folded = false }: { entries: readonly LedgerEntry[]; folded?: boolean }) {
  return (
    <aside className="armada-session-ledger" role="region" aria-label="Attachments" data-folded={folded || undefined}>
      {entries.length === 0 ? <EmptyLedger /> : null}
      {SECTIONS.map(({ kind, label, Glyph }) => (
        <LedgerSection key={kind} kind={kind} label={label} Glyph={Glyph} all={entries.filter((one) => one.kind === kind)} />
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
      {rows.length === 0 ? null : (
            <ul className="armada-session-ledger__rows">
              {rows.map((row) => {
                const Mark = row.mark === undefined ? undefined : MARK[row.mark.glyph];
                const form = row.artifact === undefined ? undefined : ARTIFACT[row.artifact];
                return (
                  <li key={row.key} className="armada-session-ledger__row" aria-label={row.name}>
                    <button type="button" className="armada-session-ledger__open" aria-label={`Open ${row.name}`} onClick={row.onOpen}>
                      {form === undefined ? (
                        <Glyph size={12} strokeWidth={2} aria-hidden />
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
                      {Mark === undefined || row.mark === undefined ? null : (
                        <Tooltip label={row.mark.said}>
                          <span
                            className="armada-session-mark"
                            role="img"
                            aria-label={row.mark.said}
                            data-pulsing={row.mark.glyph === "running" || undefined}
                          >
                            <Mark size={12} strokeWidth={2} aria-hidden />
                          </span>
                        </Tooltip>
                      )}
                    </button>
                    {row.exits}
                  </li>
                );
              })}
            </ul>
      )}
      {filtered && passed.length > NEWEST ? (
        <button type="button" className="armada-session-ledger__more" onClick={() => setMore(!more)}>
          {more ? "Less" : "More"}
        </button>
      ) : null}
    </section>
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
