import { useState } from "react";
import type { ReactNode } from "react";
import { BatteryLow, Check, CircleDashed, CircleDot, Clock, Ellipsis, Eye, GitFork, Link, MessageSquareDashed, MoveRight, Power, Reply, RotateCw, ShieldX, SquarePen, Unplug } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { ContextFigure, type ContextFill } from "./ContextFigure";
import { RunningChecks, type RunningCheck } from "../RunningChecks/RunningChecks";
import { LoadoutMark, type LoadoutMarkProps } from "../LoadoutMark/LoadoutMark";
import { useMarking, type Mark } from "../../marking";

/**
 * A Session open, in the glass panel a Job's detail and Overview's panels are
 * (`Card` on the canvas). **The state is one registry mark in the header with
 * its tooltip**, not a colour on the frame: blank is a step not yet started,
 * working is a Drone running, waiting is a Job at review, idle is a turn finished with nothing asked, failing is a
 * pull request's Checks red, and ended is nothing listening, whether it was ended
 * or its terminal stopped asking, limited is held until the plan's limit resets, and after is dispatched to wait on another Session and not started
 * (`link`, the Queued reason's upstream that has not finished), and draft is a Job holding a first message nobody has sent (`message-square-dashed`). `[conventions.session_state_borrowing]` and
 * `[conventions.session_fork_marks]` in `packages/icons/` lend them.
 */
export type SessionState = "blank" | "working" | "stuck" | "waiting" | "replied" | "done" | "failing" | "idle" | "quiet" | "ended" | "limited" | "after" | "draft";

/**
 * `stuck` is a turn running that has been quiet, `replied` a turn that ended on the agent's message
 * with no answer yet, and `done` an agent that said nothing is left. They are what Fleet reads a
 * Session to be doing (`SessionStanding`), so the list and the Cockpit draw one answer.
 * `[conventions.session_standing_marks]` in `packages/icons/` lends the glyphs.
 */
const MARK: Record<SessionState, LucideIcon> = {
  stuck: Clock,
  replied: Reply,
  done: Power,
  blank: CircleDashed,
  working: CircleDot,
  waiting: Eye,
  failing: ShieldX,
  idle: Check,
  quiet: Unplug,
  ended: Unplug,
  limited: BatteryLow,
  after: Link,
  draft: MessageSquareDashed,
};

/**
 * Mock-only: the owner's second take on the Draft mark, swapped in by the mock's `?take=b`. Production
 * draws take A, `message-square-dashed`, until he chooses.
 */
export function takeDraftGlyph(glyph: LucideIcon): void {
  MARK.draft = glyph;
}

/** The glyph a state wears, for a surface that draws the mark itself, as a tile does. */
export const sessionGlyphOf = (state: SessionState): LucideIcon => MARK[state];

/** The state's one mark, animated while a turn runs, named by its tooltip. */
export function SessionMark({ state, said }: { state: SessionState; said: string }) {
  const Glyph = MARK[state];
  if (state === "limited") return <LimitMark said={said} />;
  return (
    <Tooltip label={said}>
      <span className="armada-session-mark" role="img" aria-label={said} data-pulsing={state === "working" || undefined}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** Held for the plan's limit, until Fleet wakes it at the reset: an icon and its tooltip, nothing animated. */
export function LimitMark({ said, size = 16 }: { said: string; size?: 12 | 16 }) {
  return (
    <Tooltip label={said}>
      <span className="armada-session-mark armada-limit-mark" role="img" aria-label={said}>
        <BatteryLow size={size} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The mark's tooltip: "Limit reached · resumes 6:40 PM", in local time. */
export function limitHeldSaid(until: string): string {
  const at = new Date(until);
  if (Number.isNaN(at.getTime())) return "Limit reached";
  return `Limit reached · resumes ${at.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })}`;
}

/** A terminal Session whose mod is older than the repository's. The act is the terminal's own. */
export const MOD_OUT_OF_DATE = "Mod out of date: run /reload-plugins";

export function ModMark({ size = 16 }: { size?: 12 | 16 }) {
  return (
    <Tooltip label={MOD_OUT_OF_DATE}>
      <span className="armada-session-mark" role="img" aria-label={MOD_OUT_OF_DATE}>
        <RotateCw size={size} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export { ContextFigure } from "./ContextFigure";
export type { ContextFill } from "./ContextFigure";

/** A Session that is big or old, as figures already said, and the two acts offered on it in the header's menu. */
export type FreshStart = {
  /** The live tokens in its context window. */
  tokens: number;
  /** Whole hours since it started. */
  hours: number;
  /** The old Session is writing its handoff: New Job is listed and waits. */
  busy?: boolean;
  /** Starts a new Job that opens on a short handoff from this one. */
  onFresh: () => void;
  /** Starts a new Job from this one's conversation. Absent where Fleet offers no fork of a live Session. */
  onFork?: () => void;
};

const FRESH_ENTRY = "fresh";
const FRESH_FORK_ENTRY = "fresh-fork";

/** The menu the caller gave, with the fresh start's acts added before its first rule, or last when it has none. */
function menuWithFresh(menu: SessionFrameProps["menu"], fresh: FreshStart | undefined): SessionFrameProps["menu"] {
  if (fresh === undefined) return menu;
  const acts: DropdownMenuEntry[] = [
    { kind: "item", id: FRESH_ENTRY, label: "New Job", icon: MoveRight, hint: "Starts with a short handoff from this one", ...(fresh.busy === true ? { disabled: true } : {}) },
    ...(fresh.onFork === undefined ? [] : [{ kind: "item", id: FRESH_FORK_ENTRY, label: "Fork", icon: GitFork } as const]),
  ];
  const entries = menu?.entries ?? [];
  const rule = entries.findIndex((one) => one.kind === "separator");
  const at = rule === -1 ? entries.length : rule;
  return {
    entries: [...entries.slice(0, at), ...acts, ...entries.slice(at)],
    onSelect: (id) => {
      if (id === FRESH_ENTRY) fresh.onFresh();
      else if (id === FRESH_FORK_ENTRY) fresh.onFork?.();
      else menu?.onSelect(id);
    },
  };
}
export { modelSaid } from "./model-said";
export type { ContextBreakdown } from "../../context-breakdown";

export type SessionFrameProps = {
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  id: string;
  /** What other sessions call it, in the terminal glyph's tooltip in place of `id`. */
  address?: string;
  /** Absent until the first turn has named it. */
  title?: string;
  /**
   * Saves a new name. **Present, the title is edited where it stands**: a press opens it as a field,
   * Enter saves it and Esc, or leaving it, keeps the old one.
   */
  onRename?: (title: string) => void;
  /** The Session's mod is older than the repository's: marked beside the state. */
  modOutOfDate?: boolean;
  /** The context window's fill, drawn as a tank after `actions`. Absent draws nothing. */
  context?: ContextFill;
  /**
   * The Checks the Session has out, drawn as the running glyph and their count before the context tank: a press lists
   * them and a press on one calls `onOpenCheck` with its run. Absent or empty draws nothing.
   */
  checks?: readonly RunningCheck[];
  onOpenCheck?: (run: number) => void;
  /**
   * What the Session's agent carries, as a quiet mark after the context tank: hover opens a card of the counts and
   * the latest turn, a press `onOpen`s the whole Loadout. Absent until the Session has reported one.
   */
  loadout?: LoadoutMarkProps;
  /** Big or old: New Job and Fork join the header's menu, which is drawn for them when `menu` is absent. Absent adds nothing. */
  fresh?: FreshStart;
  /** Where the Session works, after the title: the repository's name, or the control that moves it. Absent draws nothing. */
  repository?: ReactNode;
  /** Icon-only acts after the title: the ledger's button where it has folded. */
  actions?: ReactNode;
  /**
   * The Session's other acts, behind one three-dots button just before `back`: each an item with its glyph, and a key
   * where it has one. Absent or empty draws no button.
   */
  menu?: { entries: DropdownMenuEntry[]; onSelect: (id: string) => void };
  /** The way out, the header's far right item. */
  back?: ReactNode;
  /** The ledger, as a panel of its own beside the conversation: a sibling card the height of the first, scrolling on its own. */
  aside?: ReactNode;
  children: ReactNode;
};

/** The title, in place, on one line with the whole of it in a tooltip; a field while it is being changed. */
function SessionTitle({ title, onRename }: { title?: string; onRename?: (title: string) => void }) {
  const [draft, setDraft] = useState<string | undefined>(undefined);
  if (onRename === undefined) {
    if (title === undefined) return <span className="armada-session-frame__title" />;
    return (
      <span className="armada-session-frame__title">
        <Tooltip label={title}>
          <span className="armada-session-frame__title-text">{title}</span>
        </Tooltip>
      </span>
    );
  }
  if (draft === undefined && title === undefined) {
    return (
      <span className="armada-session-frame__title">
        <Tooltip label="Rename">
          <Button variant="ghost" size="sm" iconOnly aria-label="Rename" onClick={() => setDraft("")}>
            <SquarePen size={16} strokeWidth={2} aria-hidden="true" />
          </Button>
        </Tooltip>
      </span>
    );
  }
  if (draft === undefined) {
    return (
      <span className="armada-session-frame__title">
        <Tooltip label={title}>
          <button type="button" className="armada-session-frame__title-text" aria-label={`${title}, rename`} onClick={() => setDraft(title ?? "")}>
            {title}
          </button>
        </Tooltip>
      </span>
    );
  }
  const save = () => {
    const next = draft.trim();
    setDraft(undefined);
    if (next !== "" && next !== title) onRename(next);
  };
  return (
    <span className="armada-session-frame__title" data-editing>
      <Input
        aria-label="Job name"
        value={draft}
        autoFocus
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => setDraft(undefined)}
        onKeyDown={(event) => {
          if (event.key === "Enter") save();
          if (event.key === "Escape") {
            event.stopPropagation();
            setDraft(undefined);
          }
        }}
      />
    </span>
  );
}

const DOCKED: readonly Mark[] = [{ of: ".armada-shell", as: "data-dock" }];

export function SessionFrame({ state, said, id, address, title, onRename, modOutOfDate = false, checks, onOpenCheck, context, loadout, fresh, repository, actions, menu, back, aside, children }: SessionFrameProps) {
  const named = address ?? id;
  const more = menuWithFresh(menu, fresh);
  // The aside wears the dock's class, and the shell's glow follows it (`TheShell.css`).
  const docked = useMarking(DOCKED);
  return (
    <div ref={aside === undefined || aside === null ? undefined : docked} className="armada-session-panels">
    <Card className="armada-session-frame" role="region" aria-label={`Job ${named}`}>
      <header className="armada-session-frame__head">
        <span className="armada-session-frame__state">{state === "quiet" && modOutOfDate ? <ModMark /> : <SessionMark state={state} said={`${said} · ${named}`} />}</span>
        <SessionTitle {...(title === undefined ? {} : { title })} {...(onRename === undefined ? {} : { onRename })} />
        {repository}
        {actions}
        {checks === undefined ? null : <RunningChecks checks={checks} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />}
        {context === undefined ? null : <ContextFigure fill={context} />}
        {loadout === undefined ? null : <LoadoutMark {...loadout} />}
        <span className="armada-session-frame__marks">
          {modOutOfDate && state !== "quiet" ? <ModMark /> : null}
        </span>
        {more === undefined || more.entries.length === 0 ? null : <DropdownMenu align="end" portal icon={Ellipsis} triggerLabel="More" entries={more.entries} onSelect={more.onSelect} />}
        {back}
      </header>
      <div className="armada-session-frame__body">{children}</div>
    </Card>
    {aside === undefined || aside === null ? null : <Card className="armada-session-frame__aside armada-shell__dock">{aside}</Card>}
    </div>
  );
}
