import { useEffect, useRef, type KeyboardEvent, type ReactNode } from "react";
import { Bell, ChevronDown, ChevronRight, Eclipse, GitPullRequestArrow, Link, Palette, PanelsTopLeft, Ticket } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * One mod on this machine: the kind it is, its switch, and Promote.
 *
 * **The leading mark is the kind's glyph**, the palette for a theme, the eclipse for a tweak laid over it, the panels for a
 * layout, the chain for words, the bell for Fleet and the ticket, which the registry draws an issue as, for a tracker, with the kind's name on hover. The line under the title is one plain fact: what is wrong with the mod, or the branch it
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
export type ModKind = "theme" | "tweak" | "layout" | "words" | "fleet" | "tracker";

const KIND = { theme: "Theme", tweak: "Theme tweak", layout: "Layout", words: "Words", fleet: "Fleet", tracker: "Tracker" } as const;
const GLYPH = { theme: Palette, tweak: Eclipse, layout: PanelsTopLeft, words: Link, fleet: Bell, tracker: Ticket } as const;

export function ModRow({ title, kind = "theme", enabled, onEnabled, detail, promoteDisabled = false, onPromote, opened, onOpened, autoFocus = false, children }: ModRowProps) {
  const Glyph = GLYPH[kind];
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
    <div className="armada-mod-row" role="group" aria-label={title} onKeyDown={onKeyDown}>
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
      <Tooltip label={`Put ${title} on a branch of the repository`}>
        <Button iconOnly variant="ghost" aria-label={`Promote ${title}`} disabled={promoteDisabled} onClick={onPromote}>
          <GitPullRequestArrow size={16} />
        </Button>
      </Tooltip>
      {opened && children !== undefined ? <div className="armada-mod-row__held">{children}</div> : null}
    </div>
  );
}
