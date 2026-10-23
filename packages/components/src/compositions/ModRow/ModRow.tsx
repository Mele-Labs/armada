import type { ReactNode } from "react";
import { CircleDashed, ClockAlert, FlaskConical, GitPullRequestArrow, Palette, PanelsTopLeft, ShieldCheck, ShieldX, Ticket } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

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
 * **The leading mark is the kind's glyph**, the palette for a theme and the panels for a layout, with
 * the kind's name on hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
 * was put on. Nothing is said where there is nothing to say.
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
  kind?: "theme" | "layout" | "tracker";
  /** A tracker mod's tried and valid marks. Absent for any other kind. */
  tracker?: ModTracker;
  /** The frame's hue: in use, waiting on a try, refused, or off in grey. Absent draws no frame. */
  standing?: "used" | "waiting" | "refused" | "off";
  /** This machine's switch for the mod. */
  enabled: boolean;
  onEnabled: (on: boolean) => void;
  /** Why the mod cannot be drawn, or the branch it is on. Absent draws no line. */
  detail?: ReactNode;
  /** Promote is off while the mod is invalid or already on a branch. */
  promoteDisabled?: boolean;
  onPromote: () => void;
  /** Try a tracker mod against the repository. Drawn only where given, off while the mod is invalid. */
  onTry?: () => void;
};

const KIND = { theme: "Theme", layout: "Layout", tracker: "Tracker" } as const;
const KIND_GLYPH = { theme: Palette, layout: PanelsTopLeft, tracker: Ticket } as const;
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

export function ModRow({ title, kind = "theme", tracker, standing, enabled, onEnabled, detail, promoteDisabled = false, onPromote, onTry }: ModRowProps) {
  const Kind = KIND_GLYPH[kind];
  const Tried = tracker === undefined ? undefined : TRIED_GLYPH[tracker.tried.state];
  return (
    <div className="armada-mod-row" role="group" aria-label={title} data-standing={standing}>
      <Tooltip label={KIND[kind]}>
        <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
          <Kind size={16} />
        </span>
      </Tooltip>
      <div className="armada-mod-row__switch">
        <Switch checked={enabled} onChange={(event) => onEnabled(event.target.checked)} {...(detail === undefined ? {} : { description: detail })}>
          {title}
        </Switch>
      </div>
      {tracker === undefined || Tried === undefined ? null : (
        <div className="armada-mod-row__marks">
          <Mark tone={tracker.tried.state} words={tracker.tried.words}>
            <Tried size={16} />
          </Mark>
          <Mark tone={tracker.valid ? "valid" : "invalid"} words={tracker.valid ? "Valid" : "Invalid"}>
            {tracker.valid ? <ShieldCheck size={16} /> : <ShieldX size={16} />}
          </Mark>
        </div>
      )}
      {onTry === undefined ? null : (
        <Tooltip label={`Try ${title} against the repository`}>
          <Button iconOnly variant="ghost" aria-label={`Try ${title}`} disabled={tracker?.valid === false} onClick={onTry}>
            <FlaskConical size={16} />
          </Button>
        </Tooltip>
      )}
      <Tooltip label={`Put ${title} on a branch of the repository`}>
        <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
          <GitPullRequestArrow size={16} />
        </Button>
      </Tooltip>
    </div>
  );
}
