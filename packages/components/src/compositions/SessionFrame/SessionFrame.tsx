import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { BatteryLow, LoaderCircle, Check, CircleDashed, CircleDot, CirclePause, Clock, Ellipsis, Eye, GitFork, Link, MessageSquareDashed, MoveRight, Power, Radar, Reply, RotateCw, ShieldX, SquarePen, Unplug, WandSparkles } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Popover } from "../../primitives/Popover/Popover";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { ContextFigure, type ContextCompact, type ContextFill } from "./ContextFigure";
import { RunningChecks, type RunningCheck } from "../RunningChecks/RunningChecks";
import { LoadoutMark, type LoadoutMarkProps } from "../LoadoutMark/LoadoutMark";
import { useMarking, type Mark } from "../../marking";
import { stampOf, timeFormat } from "../../time-format";

/**
 * A Session open, in the glass panel a Job's detail and Overview's panels are
 * (`Card` on the canvas). **The state is one registry mark in the header with
 * its tooltip**, not a colour on the frame: blank is a step not yet started,
 * working is a Drone running, waiting is a Job at review, idle is a turn finished with nothing asked, failing is a
 * pull request's Checks red, and ended is nothing listening, whether it was ended
 * or its terminal stopped asking, limited is held until the plan's limit resets, and after is dispatched to wait on another Session and not started
 * (`link`, the Queued reason's upstream that has not finished), draft is a Job holding a first message nobody has sent (`message-square-dashed`), and
 * paused is a Job the person parked: nothing asks anything of him until he resumes it (`circle-pause`), and held is a Job whose turn ended with nothing asked of the person and a live watch all that keeps it open (`radar`, the watch's own glyph). `[conventions.session_state_borrowing]` and
 * `[conventions.session_fork_marks]` in `packages/icons/` lend them.
 */
export type SessionState = "blank" | "working" | "stuck" | "waiting" | "replied" | "done" | "failing" | "idle" | "quiet" | "ended" | "limited" | "after" | "draft" | "held" | "paused";

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
  held: Radar,
  paused: CirclePause,
};

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

/** The mark's tooltip: "Limit reached · resumes 18:40:00", in the format the owner set. */
export function limitHeldSaid(until: string): string {
  const at = Date.parse(until);
  if (Number.isNaN(at)) return "Limit reached";
  return `Limit reached · resumes ${stampOf(at, timeFormat())}`;
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

/**
 * One thing the agent came up without, as the mark's menu lists it. `act` is the press that puts it right and is absent where nothing in this
 * build can; `running` shows the act's button as working, and `error` is the CLI's own words under a row whose act failed.
 */
export type ToolsFix = { piece: string; said: string; act?: string; running?: boolean; error?: string };

/** Arrow keys move among the menu's acts, as a menu's do. */
function arrowAmong(event: ReactKeyboardEvent<HTMLElement>): void {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const acts = [...event.currentTarget.querySelectorAll<HTMLElement>("button")];
  if (acts.length === 0) return;
  event.preventDefault();
  const at = acts.indexOf(document.activeElement as HTMLElement);
  const next = event.key === "ArrowDown" ? (at + 1) % acts.length : (at <= 0 ? acts.length : at) - 1;
  acts[next]?.focus();
}

/** The menu's body: what the mark's sentence says beyond its pieces (the CLI and its version), then a row for each piece, its act at the row's end. */
function ToolsMenu({ said, fixes, onFix }: { said: string; fixes: readonly ToolsFix[]; onFix: (piece: string) => void }) {
  const body = useRef<HTMLDivElement>(null);
  const head = fixes.reduce((left, one) => left.replace(one.said, ""), said).trim();
  // Opened, it takes focus on the first act, so the arrow keys have somewhere to start.
  useEffect(() => body.current?.querySelector<HTMLElement>("button")?.focus(), []);
  return (
    <div ref={body} className="armada-tools-menu" onKeyDown={arrowAmong}>
      {head === "" ? null : <p className="armada-tools-menu__head">{head}</p>}
      <ul className="armada-tools-menu__rows">
        {fixes.map((one) => (
          <li key={one.piece} className="armada-tools-menu__row">
            <span className="armada-tools-menu__said">{one.said}</span>
            {one.act === undefined ? null : (
              <Button size="sm" ground="sunken" pending={one.running === true} onClick={() => onFix(one.piece)}>
                {one.running === true ? (
                  <span className="armada-tools-menu__working" role="img" aria-label="Working">
                    <LoaderCircle size={12} strokeWidth={2} aria-hidden />
                  </span>
                ) : null}
                {one.act}
              </Button>
            )}
            {one.error === undefined ? null : <p className="armada-tools-menu__error">{one.error}</p>}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * The agent came up without some of its Armada tools: the mark, and its tooltip is the label. Given `fixes` it is a button, and a press opens
 * a menu of the pieces, each with the act that puts it right (the owner, 29 Oct 2026: "I hate leaving users in a state of error with no way out").
 */
export function ToolsMark({ said, size = 16, fixes, onFix }: { said: string; size?: 12 | 16; fixes?: readonly ToolsFix[]; onFix?: (piece: string) => void }) {
  if (fixes === undefined || fixes.length === 0 || onFix === undefined) {
    return (
      <Tooltip label={said}>
        <span className="armada-session-mark" role="img" aria-label={said}>
          <Unplug size={size} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    );
  }
  return (
    <Popover
      align="end"
      label={said}
      trigger={
        <Tooltip label={said}>
          <button type="button" className="armada-session-mark armada-tools-mark" aria-label={said} aria-haspopup="dialog">
            <Unplug size={size} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      }
    >
      <ToolsMenu said={said} fixes={fixes} onFix={onFix} />
    </Popover>
  );
}

export { ContextFigure } from "./ContextFigure";
export type { ContextCompact, ContextFill } from "./ContextFigure";

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
  /**
   * Has Fleet name the Job from its first message: a wand after the title, busy until this settles. The caller says a
   * refusal. Absent draws no wand.
   */
  onNameForMe?: () => Promise<unknown>;
  /** The Session's mod is older than the repository's: marked beside the state. */
  modOutOfDate?: boolean;
  /** What the agent came up without, said as the mark's tooltip: marked beside the state. Absent draws no mark. */
  toolsMissing?: string;
  /** What the mark's menu lists, one row for each piece, and the press on a row's act. Absent draws the mark with no menu. */
  toolsFixes?: { fixes: readonly ToolsFix[]; onFix: (piece: string) => void };
  /** The context window's fill, drawn as a tank after `actions`. Absent draws nothing. */
  context?: ContextFill;
  /** Compact, offered at the foot of the context tank's card or popover. Absent offers none. */
  compact?: ContextCompact;
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
  /** Icon buttons for the Session's views, in the head's row before `actions`, with a rule after them. */
  headViews?: ReactNode;
  children: ReactNode;
};

/** The title, in place, on one line with the whole of it in a tooltip; a field while it is being changed. */
function SessionTitle({ title, onRename, onNameForMe }: { title?: string; onRename?: (title: string) => void; onNameForMe?: () => Promise<unknown> }) {
  const [draft, setDraft] = useState<string | undefined>(undefined);
  const [naming, setNaming] = useState(false);
  const wand =
    onNameForMe === undefined ? null : (
      <Tooltip label="Name it for me">
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label="Name it for me"
          pending={naming}
          onClick={() => {
            setNaming(true);
            // The caller says its own refusal; the button only waits.
            void onNameForMe().then(
              () => setNaming(false),
              () => setNaming(false),
            );
          }}
        >
          <WandSparkles size={16} strokeWidth={2} aria-hidden="true" />
        </Button>
      </Tooltip>
    );
  if (onRename === undefined) {
    if (title === undefined) return <span className="armada-session-frame__title">{wand}</span>;
    return (
      <span className="armada-session-frame__title">
        <Tooltip label={title}>
          <span className="armada-session-frame__title-text">{title}</span>
        </Tooltip>
        {wand}
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
        {wand}
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
        {wand}
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

export function SessionFrame({ state, said, id, address, title, onRename, onNameForMe, modOutOfDate = false, toolsMissing, toolsFixes, checks, onOpenCheck, context, compact, loadout, fresh, repository, actions, menu, back, aside, headViews, children }: SessionFrameProps) {
  const named = address ?? id;
  const more = menuWithFresh(menu, fresh);
  // The aside wears the dock's class, and the shell's glow follows it (`TheShell.css`).
  const docked = useMarking(DOCKED);
  return (
    <div ref={aside === undefined || aside === null ? undefined : docked} className="armada-session-panels">
    <Card className="armada-session-frame" role="region" aria-label={`Job ${named}`}>
      <header className="armada-session-frame__head">
        <span className="armada-session-frame__state">{state === "quiet" && modOutOfDate ? <ModMark /> : <SessionMark state={state} said={`${said} · ${named}`} />}</span>
        <SessionTitle {...(title === undefined ? {} : { title })} {...(onRename === undefined ? {} : { onRename })} {...(onNameForMe === undefined ? {} : { onNameForMe })} />
        {repository}
        {headViews}
        {actions}
        {checks === undefined ? null : <RunningChecks checks={checks} {...(onOpenCheck === undefined ? {} : { onOpenCheck })} />}
        {context === undefined ? null : <ContextFigure fill={context} {...(compact === undefined ? {} : { compact })} />}
        {loadout === undefined ? null : <LoadoutMark {...loadout} />}
        <span className="armada-session-frame__marks">
          {modOutOfDate && state !== "quiet" ? <ModMark /> : null}
          {toolsMissing === undefined ? null : <ToolsMark said={toolsMissing} {...(toolsFixes === undefined ? {} : toolsFixes)} />}
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
