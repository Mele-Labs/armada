import type { KeyboardEvent, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

import { Stamp } from "../../primitives/Stamp/Stamp";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * An ask as a card, the one frame it is drawn in wherever it is answered: the Cockpit's call, and
 * every ask beside a Job's canvas. **One component so an ask looks the same in both** (owner,
 * 11 Oct 2026). A band in the ask's hue naming its kind, over a body that leads with its title and
 * holds the question, its answers as tiles and its acts. The Cockpit draws it over the glass with
 * an entrance; `flow` draws it in a panel, where it stays put.
 */
export type AskCardProps = {
  /** The kind's mark, in the band. */
  icon: LucideIcon;
  /** What kind of ask it is, in the band: `Plan question`, `Review`. */
  kind: string;
  /** Where it is, at the band's trailing edge: a Job's handle. Absent draws none. */
  where?: string;
  /** When it was asked, an RFC 3339 instant: drawn as a stamp that reads as an age first. */
  at?: string;
  /** The clock the stamp's age counts from. Absent, now. */
  now?: number;
  /** What it is about, drawn large over the body. Absent draws none. */
  title?: ReactNode;
  /** The card's name for a screen reader. Absent reads `kind: title` where the title is words. */
  label?: string;
  /** The Cockpit's own hue word, kept on the frame. */
  hue?: string;
  /** Leaving the Cockpit's stack: put off, or sent. */
  leaving?: "later" | "sent" | undefined;
  /** It stood behind another card until now, and slides up from there. */
  from?: "stack" | undefined;
  /** Drawn in a panel's flow rather than over the glass. */
  flow?: boolean;
  children: ReactNode;
};

export function AskCard({ icon: Icon, kind, where, at, now, title, label, hue, leaving, from, flow = false, children }: AskCardProps) {
  return (
    <section
      className="armada-callcard"
      data-hue={hue}
      data-leaving={leaving}
      data-from={from}
      data-in={flow ? "flow" : undefined}
      aria-label={label ?? (typeof title === "string" ? `${kind}: ${title}` : kind)}
    >
      <header className="armada-callcard__band">
        <Icon size={16} aria-hidden="true" />
        <span>{kind}</span>
        {where === undefined ? null : <span className="armada-callcard__where">{where}</span>}
        {at === undefined ? null : <Stamp first="age" at={at} {...(now === undefined ? {} : { now })} className="armada-callcard__age" />}
      </header>
      <div className="armada-callcard__body">
        {title === undefined ? null : <h2 className="armada-callcard__title">{title}</h2>}
        {children}
      </div>
    </section>
  );
}

/** What an answer tile draws: its words, the agent's line under it where it gave one, and what it tells the agent. */
export type AskAnswerOffer = { label: string; description?: string; recommended?: boolean; says?: string; standing?: boolean };

/**
 * One answer: its number or its own key, the line under it where one was given, and what it commits
 * to on hover. `pending` is the one waiting on Fleet, which keeps its look and sweeps the bar; `held`
 * is every other while one is, which cannot be pressed.
 */
export function AskAnswer({
  one,
  mark,
  picked,
  pending = false,
  held = false,
  pendingSays,
  onPick,
  onHover,
  onFocusing,
}: {
  one: AskAnswerOffer;
  mark: string;
  picked: boolean;
  pending?: boolean;
  held?: boolean;
  pendingSays?: string;
  onPick: () => void;
  /** Told the pointer is on it, and that it left: where an answer previews what it would do. */
  onHover?: (on: boolean) => void;
  /** Told focus is on it, and that it left. */
  onFocusing?: (on: boolean) => void;
}) {
  const tile = (
    <button
      type="button"
      role="radio"
      aria-checked={picked}
      className="armada-callcard__answer"
      data-standing={one.standing === true ? "" : undefined}
      data-pending={pending || undefined}
      aria-busy={pending || undefined}
      disabled={held && !pending}
      onClick={onPick}
      {...(onHover === undefined ? {} : { onMouseEnter: () => onHover(true), onMouseLeave: () => onHover(false) })}
      {...(onFocusing === undefined ? {} : { onFocus: () => onFocusing(true), onBlur: () => onFocusing(false) })}
    >
      <Kbd>{mark}</Kbd>
      <span className="armada-callcard__option">
        <span className="armada-callcard__label">
          {one.label}
          {one.recommended === true ? <span className="armada-callcard__recommended">Recommended</span> : null}
        </span>
        {one.description === undefined ? null : <span className="armada-callcard__description">{one.description}</span>}
      </span>
    </button>
  );
  const said = pending ? pendingSays : one.says;
  return said === undefined ? tile : <Tooltip label={said}>{tile}</Tooltip>;
}

/** The question the card puts, over a rule. `quiet` is a fact in mono where the ask is a state and not a question. */
export function AskQuestion({ quiet = false, children }: { quiet?: boolean; children: ReactNode }) {
  // A `div`, not a `p`: it holds the decisions' pips, a list, which a paragraph cannot contain.
  return (
    <div className="armada-callcard__ask" data-quiet={quiet ? "" : undefined}>
      {children}
    </div>
  );
}

/** The answers' own group, named by the question it answers. */
export function AskChoices({ label, onKeyDown, children }: { label: string; onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void; children: ReactNode }) {
  return (
    <div role="radiogroup" aria-label={label} className="armada-callcard__choices" {...(onKeyDown === undefined ? {} : { onKeyDown })}>
      {children}
    </div>
  );
}

/** One row of answers. `standing` is the dashed pair that sits under them. */
export function AskAnswers({ standing = false, children }: { standing?: boolean; children: ReactNode }) {
  return (
    <div className="armada-callcard__answers" data-standing={standing ? "" : undefined}>
      {children}
    </div>
  );
}

/** Where a question is among its decisions: a pip each, the one asked now filled. Named by the decision it stands for. */
export function AskPips({ decisions, at }: { decisions: readonly { id: string; question: string }[]; at: number }) {
  return (
    <ol className="armada-callcard__pips" aria-label="Decisions">
      {decisions.map((one, index) => (
        <li key={one.id}>
          <Tooltip label={one.question}>
            <span className="armada-callcard__pip" role="img" aria-label={one.question} aria-current={index === at ? "step" : undefined} data-here={index === at || undefined} />
          </Tooltip>
        </li>
      ))}
    </ol>
  );
}

/** The card's acts, the last at the trailing edge. */
export function AskActs({ children }: { children: ReactNode }) {
  return <div className="armada-callcard__acts">{children}</div>;
}
