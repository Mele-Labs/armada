import { MessageSquare } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { BoardEmptyState } from "../BoardEmptyState/BoardEmptyState";
import { FleetPanel, type FleetPanelProps } from "../FleetPanel/FleetPanel";
import { TitleBar, type TitleBarProps } from "../TitleBar/TitleBar";
import { Button } from "../../primitives/Button/Button";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { ShortcutRevealProvider } from "../../shortcut-reveal";
import { HelmSheet } from "./HelmSheet";
import { useMarking, type Mark } from "../../marking";
import { DockHandle, clampToRange, defaultDockWidth, dockWidthRange, tokenPx } from "../../dock-handle";

/**
 * The shell: the title row, the panel the work mounts in, and Helm's dock. **There is no left column.** The places are the title
 * row's tabs and the System strip, which the caller passes as `places`, and Fleet's panel is a popover on the title row's dot,
 * because nothing else would hold its restart, its build and its Doctor roll-up.
 *
 * The panel scrolls; the dock does not. `min-height: 0` on the scrolling child is what stops the window growing instead.
 */
export type TheShellProps = {
  /** Teach on the title row. See `TitleBar`. */
  onTeach?: TitleBarProps["onTeach"];
  /** The places the title row draws in its centre, before the search trigger. See `TitleBar`. */
  places?: TitleBarProps["places"];
  /** The word the title row draws at its start in place of the lockup. See `TitleBar`. */
  wordmark?: TitleBarProps["wordmark"];
  /** The picked repository's name in the title row, read-only. See `TitleBar`. */
  repository?: TitleBarProps["repository"];
  /** What the title row's one menu opens besides the account and Sleep. */
  menu?: TitleBarProps["menu"];
  /** The picker's entries, for the title row's narrow menu. */
  repositoryMenu?: TitleBarProps["repositoryMenu"];
  /** Opens the command palette from the title row's search field. Absent draws no field. */
  onSearch?: () => void;
  /** Opens the composer from the title row's Dispatch control. Absent draws no control. */
  onDispatch?: () => void;
  /** Disabled while nothing is connected to dispatch into. */
  dispatchDisabled?: boolean;
  /** Sleep mode's control in the title row, beside Helm's. Absent draws none. */
  sleep?: TitleBarProps["sleep"];
  /** The plan's limits, as figures in the title row. Absent draws none. */
  planUsage?: TitleBarProps["planUsage"];
  /** The panel body. The one region that scrolls. No head above it: #1090
   *  ended the panel head every screen used to spend on its own name, so a
   *  screen's own controls sit at the top of its content instead. */
  children: ReactNode;
  /** Helm's dock, on every surface. Absent draws none. */
  dock?: TheShellDock;
  /** Fleet's panel, opened from the title row's dot. */
  fleet: Omit<FleetPanelProps, "narrow">;
};

/**
 * Helm's dock (#948). **Over the content, never a column in it** (#1583): the
 * panel keeps its frame and its place on the trailing edge, and the screen
 * underneath keeps the window's width open or shut. It starts shut.
 *
 * **Closed draws nothing at all, at every width.** The title row's own Helm
 * button (#1087) is the one way back. #1094 dropped the edge strip that used
 * to sit on the trailing edge beyond the layout breakpoint, and the owner
 * dropped the folded one on 30 Sep 2026 once the title row fit the button in a slim
 * window. Open and folded draws the sheet.
 */
export type TheShellDock = {
  /** The panel over the content, or the sheet when folded. */
  open: boolean;
  /** Below `--layout-breakpoint`. A prop, because a media query cannot read the token. */
  folded?: boolean;
  /** Questions waiting on the person, counted on the title row's Helm button. Zero draws no count. */
  questions?: number;
  /** A call that lives in Helm is waiting: the title row's Helm button is tinted and pulses. */
  calling?: boolean;
  /** The binding, beside the close and in the title row's Helm tooltip. */
  binding?: string;
  onOpen: (open: boolean) => void;
  /**
   * The resting width in px. Absent draws `--w-dock`. A folded dock is a
   * `Sheet` that reads and drags this same width, clamped to its own area.
   */
  width?: number;
  /**
   * Drags and arrow-key nudges the leading-edge handle, clamped between
   * `--w-dock-min` and whatever the window leaves once `--w-work-min` is
   * accounted for — `clampDockWidth`. **Absent draws no
   * handle at all** — an edge that looks grabbable and does nothing is worse
   * than no edge. Persisting the result across a restart is the caller's, the
   * same way `open` is.
   */
  onResize?: (width: number) => void;
  /**
   * The dock's own act, beside Close — Helm's *Start fresh*, put there by the
   * owner on 18 Sep 2026: it ends the conversation the dock holds, which is
   * the head's business rather than the composer's, and the composer's own row
   * had run out of width for it. **One control**, and the head is the same
   * layout without it. Drawn in the folded sheet's head too, through `Sheet`'s
   * own `controls` slot — folding the dock must not take an act away.
   */
  action?: ReactNode;
  /** Absent draws one quiet line until the questions (#935) and the conversation (#944) arrive. */
  children?: ReactNode;
};

export function TheShell({
  places,
  wordmark,
  repository,
  menu,
  repositoryMenu,
  onSearch,
  onDispatch,
  dispatchDisabled,
  sleep,
  planUsage,
  onTeach,
  children,
  dock,
  fleet,
}: TheShellProps) {
  return (
    <ShortcutRevealProvider>
      <div className="armada-shell">
        <TitleBar
          {...(menu === undefined ? {} : { menu })}
          {...(repositoryMenu === undefined ? {} : { repositoryMenu })}
          onSearch={onSearch}
          onDispatch={onDispatch}
          dispatchDisabled={dispatchDisabled}
          helm={helmButtonOf(dock)}
          {...(sleep === undefined ? {} : { sleep })}
          {...(planUsage === undefined ? {} : { planUsage })}
          {...(onTeach === undefined ? {} : { onTeach })}
          {...(places === undefined ? {} : { places })}
          {...(wordmark === undefined ? {} : { wordmark })}
          {...(repository === undefined ? {} : { repository })}
          fleet={{ state: fleet.state, label: fleet.label, panel: <FleetInAPopover {...fleet} /> }}
        />
        <div className="armada-shell__body">
          <div className="armada-shell__work">
            <div className="armada-shell__panel">
              <div className="armada-shell__mount">{children}</div>
            </div>
            {dock === undefined ? null : <Dock {...dock} />}
          </div>
        </div>
      </div>
    </ShortcutRevealProvider>
  );
}

/** Fleet's panel as the title row's dot opens it: its own open state, since nothing else holds one. */
function FleetInAPopover(props: Omit<FleetPanelProps, "narrow" | "open" | "onOpenChange">) {
  const [open, setOpen] = useState(true);
  return <FleetPanel {...props} open={open} onOpenChange={setOpen} />;
}

const DOCK_TITLE = "Helm";

/**
 * The dock's drag range — `dockWidthRange` (`dock-handle.tsx`), given the room
 * the window leaves. `availableWidth` is the caller's `window.innerWidth`,
 * minus the chrome around the dock that isn't the panel or the dock itself:
 * the gap, the handle's hit area and the dock's own margin, four `--space-4`
 * gutters, all from `TheShell.css`.
 *
 * **The arithmetic did not move when the dock did** (#1583). `--w-work-min`
 * was the panel's floor beside the dock; it is what stays uncovered under it.
 */
function dockWidthBounds(availableWidth: number): { min: number; max: number } {
  return dockWidthRange(availableWidth - 4 * tokenPx("--space-4"));
}

/** The window's own width, read live — the one figure here no CSS token can
 *  name, and the reason the dock's ceiling has to be computed rather than
 *  read off the theme. Updates on resize so a dock already open shrinks with
 *  the window instead of pushing it into overflow. */
function useWindowWidth(): number {
  const [width, setWidth] = useState(() => (typeof window === "undefined" ? 0 : window.innerWidth));
  useEffect(() => {
    function read(): void {
      setWidth(window.innerWidth);
    }
    window.addEventListener("resize", read);
    return () => window.removeEventListener("resize", read);
  }, []);
  return width;
}

export { defaultDockWidth };

/**
 * Clamped to the drag range `dockWidthBounds` computes from `availableWidth`
 * — the caller's `window.innerWidth` — so a width read back from storage —
 * stale, from a narrower window, or from a build that carried different
 * tokens — never draws past what today's window and today's tokens allow.
 */
export function clampDockWidth(width: number, availableWidth: number): number {
  return clampToRange(width, dockWidthBounds(availableWidth));
}

/** `TheShell.css` gives the shell Helm's glow while a dock is drawn. */
const DOCKED: readonly Mark[] = [{ of: ".armada-shell", as: "data-dock" }];

function Dock({
  open,
  folded = false,
  binding,
  width,
  onResize,
  onOpen,
  action,
  children,
}: TheShellDock) {
  const availableWidth = useWindowWidth();
  const docked = useMarking(DOCKED);
  const body = children ?? (
    <BoardEmptyState quiet>Questions waiting on you, and Helm, will be here.</BoardEmptyState>
  );

  if (open && !folded) {
    // Clamped before it reaches the handle, not after: the window can narrow
    // between one render and the next, and a `width` still carrying what fit
    // the old one would show the handle's own aria-valuenow, its drag anchor
    // and its arrow-key math a number the aside on screen already disagrees
    // with. Clamping once here keeps every reader of `restingWidth` — the
    // aside's style included — looking at what is actually drawn.
    const restingWidth = clampDockWidth(width ?? defaultDockWidth(), availableWidth);
    // **The handle rides on the layer with the dock, not in the flow beside
    // it.** Left behind in `__work` it would still be a 16px track, so the
    // content would move by that much on every open — the same defect as the
    // dock's own 380, one order of magnitude down and harder to see.
    return (
      <div ref={docked} className="armada-shell__dock-layer">
        {onResize === undefined ? null : (
          <DockHandle
            width={restingWidth}
            {...dockWidthBounds(availableWidth)}
            label={DOCK_TITLE}
            onResize={onResize}
          />
        )}
        <aside
          className="armada-shell__dock armada-glass"
          aria-label={DOCK_TITLE}
          style={width === undefined ? undefined : { width: `${restingWidth}px` }}
        >
          <div className="armada-shell__dock-head">
            <span className="armada-shell__dock-chip" aria-hidden><MessageSquare size={16} strokeWidth={2} /></span>
            <h2 className="armada-shell__dock-title">{DOCK_TITLE}</h2>
            {action}
            <Button variant="secondary" size="sm" ground="card" onClick={() => onOpen(false)}>
              Close
              {binding === undefined ? null : <KbdCmd shortcut={binding} />}
            </Button>
          </div>
          <div className="armada-shell__dock-body">{body}</div>
        </aside>
      </div>
    );
  }

  // Closed, at any width: nothing. The title row's Helm button is the one way back.
  if (!folded) return null;

  return (
    <HelmSheet
      open={open && folded}
      title={DOCK_TITLE}
      binding={binding}
      controls={action}
      width={width}
      onResize={onResize}
      onClose={() => onOpen(false)}
    >
      {body}
    </HelmSheet>
  );
}

/**
 * Helm's title-row reopen control, from the same `dock` shape the dock reads —
 * so the caller states the dock once and the button and the dock agree on it.
 *
 * **Absent whenever the full dock is showing over the content.** Drawn while
 * folded, open or shut, and while closed outright: it is the one way back to
 * Helm at every width, since no edge strip is drawn at any.
 */
function helmButtonOf(
  dock: TheShellDock | undefined,
): { questions: number; calling?: boolean; binding?: string; onOpen: () => void } | undefined {
  if (dock === undefined) return undefined;
  if (dock.open && dock.folded !== true) return undefined;
  return {
    questions: dock.questions ?? 0,
    ...(dock.calling === true ? { calling: true } : {}),
    ...(dock.binding === undefined ? {} : { binding: dock.binding }),
    onOpen: () => dock.onOpen(true),
  };
}
