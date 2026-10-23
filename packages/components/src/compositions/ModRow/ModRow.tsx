import { useEffect, useRef, type KeyboardEvent, type ReactNode } from "react";
import { Bell, Box, ChevronDown, ChevronRight, CircleDashed, ClockAlert, Eclipse, FlaskConical, GitPullRequestArrow, Link, Package, Palette, PanelsTopLeft, ShieldCheck, ShieldX, Ticket } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PullRequestChip } from "../PullRequestChip/PullRequestChip";
import { RowLink } from "../RowLink/RowLink";

export type ModTried = {
  /** `stale` is a try of a `tracker.toml` that has been edited since. */
  state: "passed" | "failed" | "stale" | "never";
  /** What the mark says on hover: when and where it passed, why it failed, or `Stale`. */
  words: string;
};

export type ModTracker = {
  tried: ModTried;
  valid: boolean;
};

/**
 * One mod on this machine: the kind it is, its switch, and Promote.
 *
 * **The leading mark is the kind's glyph**, the palette for a theme, the eclipse for a tweak laid over it, the panels for a
 * layout, the chain for words, the bell for Fleet and the ticket, which the registry draws an issue as, for a tracker, with the kind's name on hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
 * was put on. Once promoted, the pull request it is on and the Job that holds it, each a press. A mod Armada ships
 * carries the package where Promote would be. Nothing is said where there is nothing to say.
 *
 * **A tracker mod adds two marks, each a glyph in a filled chip with its words on hover**: whether the
 * `tracker.toml` as it stands has been tried against the real tracker, and whether it is valid. A
 * Manifest uses the mod only while it is tried, valid and switched on, and the switch is the third fact.
 * `standing` frames the row in that hue (used, waiting for a try, refused, or grey where switched off); a row without it draws no frame.
 */
export type ModRowProps = {
  /** The mod's name. Names the row and the switch. */
  title: string;
  /** What the mod changes. A theme where absent. */
  kind?: ModKind;
  /** A tracker mod's tried and valid marks. Absent for any other kind. */
  tracker?: ModTracker;
  /**
   * Keeps the tracker columns, the marks and Try, in this row whether or not it is a tracker, so a list that holds a tracker lines
   * up in every row. A list without one leaves them out.
   */
  withTracker?: boolean;
  /** The frame's hue: in use, waiting on a try, refused, or off in grey. Absent draws no frame. */
  standing?: "used" | "waiting" | "refused" | "off";
  /** This machine's switch for the mod. Absent for a mod only the shared home has, which has nothing here to switch. */
  enabled?: boolean;
  onEnabled?: (on: boolean) => void;
  /** Why the mod cannot be drawn, or the branch it is on. Absent draws no line. */
  detail?: ReactNode;
  /** Ships with Armada: marked so, and never promoted. */
  builtIn?: boolean;
  /** How it stands against the shared home, as Kit draws it: `SyncMarks`. */
  syncMark?: ReactNode;
  /** The act that settles it, as Kit draws it: `SyncAct`. */
  syncAct?: ReactNode;
  /** The pull request Promote opened. */
  pull?: { number: number; title: string; onOpen: () => void };
  /** The Job that holds that pull request. */
  job?: { title: string; onOpen: () => void };
  /** Promote is off while the mod is invalid or already on a branch. */
  promoteDisabled?: boolean;
  /** Absent draws no Promote. */
  onPromote?: () => void;
  /** Try a tracker mod against the repository. Drawn only where given, off while the mod is invalid. */
  onTry?: () => void;
  /**
   * A mod that can be opened to read what it holds, which `children` draws under the row. The press is a chevron that
   * names what it does, and `autoFocus` puts focus on it as the row mounts, for a row that was asked for by name.
   */
  opened?: boolean;
  onOpened?: (open: boolean) => void;
  autoFocus?: boolean;
  children?: ReactNode;
};

/** What a mod changes: how Bridge looks, how it is arranged, how it shows words, what Fleet does with a notice. */
export type ModKind = "theme" | "tweak" | "layout" | "words" | "fleet" | "tracker";

const KIND = { theme: "Theme", tweak: "Theme tweak", layout: "Layout", words: "Words", fleet: "Fleet", tracker: "Tracker" } as const;
const GLYPH = { theme: Palette, tweak: Eclipse, layout: PanelsTopLeft, words: Link, fleet: Bell, tracker: Ticket } as const;

/** The kind's glyph, with its name on hover. Kit's Mods list draws the same one. */
export function ModKindMark({ kind = "theme" }: { kind?: ModKind }) {
  const Glyph = GLYPH[kind];
  return (
    <Tooltip label={KIND[kind]}>
      <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
        <Glyph size={16} />
      </span>
    </Tooltip>
  );
}

const TRIED_GLYPH = { passed: FlaskConical, failed: FlaskConical, stale: ClockAlert, never: CircleDashed } as const;

function Mark({ tone, words, children }: { tone: string; words: string; children: ReactNode }) {
  return (
    <Tooltip label={words}>
      <span className="armada-mod-row__mark" data-tone={tone} role="img" aria-label={words}>
        {children}
      </span>
    </Tooltip>
  );
}

export const BUILT_IN_SAID = "Ships with Armada";

/** Where a mod Armada ships came from. Kit's Mods list draws the same one. */
export function BuiltInMark() {
  return (
    <Tooltip label={BUILT_IN_SAID}>
      <span className="armada-mod-row__origin" role="img" aria-label={BUILT_IN_SAID}>
        <Package size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

export function ModRow({ title, kind = "theme", tracker, withTracker = false, standing, enabled, onEnabled, detail, builtIn = false, syncMark, syncAct, pull, job, promoteDisabled = false, onPromote, onTry, opened, onOpened, autoFocus = false, children }: ModRowProps) {
  const Glyph = GLYPH[kind];
  const Tried = tracker === undefined ? undefined : TRIED_GLYPH[tracker.tried.state];
  const lined = withTracker || tracker !== undefined;
  const toggle = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (autoFocus) toggle.current?.focus();
  }, [autoFocus]);
  // The arrows open and close a row and move between rows, as they do in a tree. They act on the row's own controls: what an opened
  // row holds keeps its keys.
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const row = event.currentTarget;
    const target = event.target as HTMLElement;
    if (target.closest(".armada-mod-row__held") !== null || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (event.key === "ArrowRight" && onOpened !== undefined && opened !== true) onOpened(true);
    else if (event.key === "ArrowLeft" && onOpened !== undefined && opened === true) onOpened(false);
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      const rows = Array.from(row.parentElement?.children ?? []).filter((one) => one.classList.contains("armada-mod-row"));
      const next = rows[rows.indexOf(row) + (event.key === "ArrowDown" ? 1 : -1)];
      if (next === undefined) return;
      const controls = (of: Element) => Array.from(of.querySelectorAll<HTMLElement>("input, button")).filter((one) => one.closest(".armada-mod-row__held") === null && !one.hasAttribute("disabled"));
      const at = controls(row).indexOf(target);
      (controls(next)[Math.max(at, 0)] ?? controls(next)[0])?.focus();
    } else return;
    // A page that moves between its columns on the arrows is not told of a key the row took.
    event.preventDefault();
    event.stopPropagation();
  };
  return (
    <div className="armada-mod-row" role="group" aria-label={title} data-standing={standing} data-tracking={lined || undefined} onKeyDown={onKeyDown}>
      <Tooltip label={KIND[kind]}>
        <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
          <Glyph size={16} />
        </span>
      </Tooltip>
      <div className="armada-mod-row__switch">
        {enabled === undefined ? (
          <span className="armada-mod-row__title">{title}</span>
        ) : (
          <Switch checked={enabled} onChange={(event) => onEnabled?.(event.target.checked)} {...(detail === undefined ? {} : { description: detail })}>
            {title}
          </Switch>
        )}
      </div>
      {!lined ? null : tracker === undefined || Tried === undefined ? (
        <span className="armada-mod-row__marks" aria-hidden="true" />
      ) : (
        <div className="armada-mod-row__marks">
          <Mark tone={tracker.tried.state} words={tracker.tried.words}>
            <Tried size={16} />
          </Mark>
          <Mark tone={tracker.valid ? "valid" : "invalid"} words={tracker.valid ? "Valid" : "Invalid"}>
            {tracker.valid ? <ShieldCheck size={16} /> : <ShieldX size={16} />}
          </Mark>
        </div>
      )}
      {!lined ? null : onTry === undefined ? (
        <span className="armada-mod-row__try" aria-hidden="true" />
      ) : (
        <Tooltip label={`Try ${title} against the repository`}>
          <Button iconOnly variant="ghost" aria-label={`Try ${title}`} disabled={tracker?.valid === false} onClick={onTry}>
            <FlaskConical size={16} />
          </Button>
        </Tooltip>
      )}
      {onOpened === undefined ? (
        <span className="armada-mod-row__slot" aria-hidden="true" />
      ) : (
        <Tooltip label={opened ? "Hide the details" : "Show the details"}>
          <Button
            ref={toggle}
            iconOnly
            variant="ghost"
            aria-label={`Details of ${title}`}
            aria-expanded={opened === true}
            onClick={() => onOpened(opened !== true)}
          >
            {opened ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          </Button>
        </Tooltip>
      )}
      {/* Every cell is drawn, empty or not, so each control keeps its column down a list. */}
      <span className="armada-mod-row__cell">{syncMark}</span>
      <span className="armada-mod-row__cell">{syncAct}</span>
      {builtIn ? (
        <BuiltInMark />
      ) : onPromote === undefined ? (
        <span className="armada-mod-row__cell" />
      ) : (
        <Tooltip label={`Add ${title} to Armada in a pull request`}>
          <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
            <GitPullRequestArrow size={16} />
          </Button>
        </Tooltip>
      )}
      {pull === undefined && job === undefined ? null : (
        <div className="armada-mod-row__held armada-mod-row__promoted">
          {pull === undefined ? null : <PullRequestChip number={pull.number} title={pull.title} state="open" onOpen={pull.onOpen} />}
          {job === undefined ? null : (
            <RowLink
              label={`Job ${job.title}`}
              mark={
                <Tooltip label="Job">
                  <span className="armada-mod-row__job" role="img" aria-label="Job">
                    <Box size={12} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
              }
              onOpen={job.onOpen}
            >
              {job.title}
            </RowLink>
          )}
        </div>
      )}
      {opened && children !== undefined ? <div className="armada-mod-row__held">{children}</div> : null}
    </div>
  );
}
