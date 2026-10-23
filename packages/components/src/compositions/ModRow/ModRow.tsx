import { useEffect, useRef, type ReactNode } from "react";
import { Bell, ChevronDown, ChevronRight, Eclipse, GitPullRequestArrow, Link, Palette, PanelsTopLeft } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * One mod on this machine: the kind it is, its switch, and Promote.
 *
 * **The leading mark is the kind's glyph**, the palette for a theme, the eclipse for a tweak laid over it, the panels for a
 * layout, the chain for words and the bell for Fleet, with the kind's name on hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
 * was put on. Nothing is said where there is nothing to say.
 */
export type ModRowProps = {
  /** The mod's name. Names the row and the switch. */
  title: string;
  /** What the mod changes. A theme where absent. */
  kind?: ModKind;
  /** This machine's switch for the mod. */
  enabled: boolean;
  onEnabled: (on: boolean) => void;
  /** Why the mod cannot be drawn, or the branch it is on. Absent draws no line. */
  detail?: ReactNode;
  /** Promote is off while the mod is invalid or already on a branch. */
  promoteDisabled?: boolean;
  onPromote: () => void;
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
export type ModKind = "theme" | "tweak" | "layout" | "words" | "fleet";

const KIND = { theme: "Theme", tweak: "Theme tweak", layout: "Layout", words: "Words", fleet: "Fleet" } as const;
const GLYPH = { theme: Palette, tweak: Eclipse, layout: PanelsTopLeft, words: Link, fleet: Bell } as const;

export function ModRow({ title, kind = "theme", enabled, onEnabled, detail, promoteDisabled = false, onPromote, opened, onOpened, autoFocus = false, children }: ModRowProps) {
  const Glyph = GLYPH[kind];
  const toggle = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (autoFocus) toggle.current?.focus();
  }, [autoFocus]);
  return (
    <div className="armada-mod-row" role="group" aria-label={title}>
      <Tooltip label={KIND[kind]}>
        <span className="armada-mod-row__kind" role="img" aria-label={KIND[kind]}>
          <Glyph size={16} />
        </span>
      </Tooltip>
      <div className="armada-mod-row__switch">
        <Switch checked={enabled} onChange={(event) => onEnabled(event.target.checked)} {...(detail === undefined ? {} : { description: detail })}>
          {title}
        </Switch>
      </div>
      {onOpened === undefined ? null : (
        <Tooltip label={opened ? "Hide the details" : "Show the details"}>
          <Button
            ref={toggle}
            iconOnly
            variant="ghost"
            aria-label={`Details of ${title}`}
            aria-expanded={opened === true}
            onClick={() => onOpened(opened !== true)}
            // The arrows open and close it, as they do a row of a tree.
            onKeyDown={(event) => {
              if (event.key === "ArrowRight" && opened !== true) onOpened(true);
              if (event.key === "ArrowLeft" && opened === true) onOpened(false);
            }}
          >
            {opened ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          </Button>
        </Tooltip>
      )}
      <Tooltip label={`Put ${title} on a branch of the repository`}>
        <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
          <GitPullRequestArrow size={16} />
        </Button>
      </Tooltip>
      {opened && children !== undefined ? <div className="armada-mod-row__held">{children}</div> : null}
    </div>
  );
}
