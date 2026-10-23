import type { ReactNode } from "react";
import { Box, GitPullRequestArrow, Package, Palette, PanelsTopLeft } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PullRequestChip } from "../PullRequestChip/PullRequestChip";
import { RowLink } from "../RowLink/RowLink";

/**
 * One mod on this machine: the kind it is, its switch, and Promote.
 *
 * **The leading mark is the kind's glyph**, the palette for a theme and the panels for a layout, with
 * the kind's name on hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
 * was put on. Once promoted, the pull request it is on and the Job that holds it, each a press. A mod Armada ships
 * carries the package where Promote would be. Nothing is said where there is nothing to say.
 */
export type ModRowProps = {
  /** The mod's name. Names the row and the switch. */
  title: string;
  /** What the mod changes. A theme where absent. */
  kind?: "theme" | "layout";
  /** This machine's switch for the mod. Absent for a mod only the shared home has, which has nothing here to switch. */
  enabled?: boolean;
  onEnabled?: (on: boolean) => void;
  /** Why the mod cannot be drawn, or the branch it is on. Absent draws no line. */
  detail?: ReactNode;
  /** Ships with Armada: marked so, and never promoted. */
  builtIn?: boolean;
  /** How it stands against the shared home and the act that settles it, as Kit draws them: `SyncMarks` and `SyncAct`. */
  sync?: ReactNode;
  /** The pull request Promote opened. */
  pull?: { number: number; title: string; onOpen: () => void };
  /** The Job that holds that pull request. */
  job?: { title: string; onOpen: () => void };
  /** Promote is off while the mod is invalid or already on a branch. */
  promoteDisabled?: boolean;
  /** Absent draws no Promote. */
  onPromote?: () => void;
};

const KIND = { theme: "Theme", layout: "Layout" } as const;

/** The kind's glyph, with its name on hover. Kit's Mods list draws the same one. */
export function ModKindMark({ kind = "theme" }: { kind?: "theme" | "layout" }) {
  return (
    <Tooltip label={KIND[kind]}>
      <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
        {kind === "layout" ? <PanelsTopLeft size={16} /> : <Palette size={16} />}
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

export function ModRow({ title, kind = "theme", enabled, onEnabled, detail, builtIn = false, sync, pull, job, promoteDisabled = false, onPromote }: ModRowProps) {
  return (
    <div className="armada-mod-row" role="group" aria-label={title}>
      <Tooltip label={KIND[kind]}>
        <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
          {kind === "layout" ? <PanelsTopLeft size={16} /> : <Palette size={16} />}
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
        {pull === undefined && job === undefined ? null : (
          <div className="armada-mod-row__held">
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
      </div>
      {sync === undefined ? null : <span className="armada-mod-row__sync">{sync}</span>}
      {builtIn ? (
        <BuiltInMark />
      ) : onPromote === undefined ? null : (
        <Tooltip label={`Put ${title} on a branch of the repository`}>
          <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
            <GitPullRequestArrow size={16} />
          </Button>
        </Tooltip>
      )}
    </div>
  );
}
