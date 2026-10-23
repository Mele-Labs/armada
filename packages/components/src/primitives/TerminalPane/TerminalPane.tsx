import { Fragment, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { CircleCheck, CircleX, PanelBottom, PanelBottomClose, PanelRight, PowerOff, RotateCcw, Square, SquareTerminal, X } from "lucide-react";

import { Button } from "../Button/Button";
import { Tabs } from "../Tabs/Tabs";
import { Tooltip } from "../Tooltip/Tooltip";

/** How a terminal run stands. `code` is null where it was stopped. */
export type TerminalPhase = { state: "running" } | { state: "ended"; code: number | null };

const exitSaid = (phase: TerminalPhase) => (phase.state === "running" ? "Running" : phase.code === null ? "Stopped" : `Exit ${phase.code}`);

/** A run's mark: the terminal glyph, animated while it runs, then passed, failed or stopped. Its tooltip says which. */
export function TerminalMark({ phase }: { phase: TerminalPhase }) {
  const said = exitSaid(phase);
  const result = phase.state === "running" ? "running" : phase.code === 0 ? "passed" : phase.code === null ? "stopped" : "failed";
  return (
    <Tooltip label={said}>
      <span className="armada-terminal-mark" data-result={result} role="img" aria-label={result === "running" ? "Running" : result === "passed" ? "Passed" : result === "stopped" ? "Stopped" : "Failed"}>
        {result === "running" ? <SquareTerminal size={16} strokeWidth={2} aria-hidden /> : result === "passed" ? <CircleCheck size={16} strokeWidth={2} aria-hidden /> : <CircleX size={16} strokeWidth={2} aria-hidden />}
      </span>
    </Tooltip>
  );
}

export type TerminalPaneProps = {
  command: string;
  phase: TerminalPhase;
  onStop: () => void;
  onRerun: () => void;
  onClose: () => void;
  /** The body: the screen the run is drawn on. */
  children: ReactNode;
};

/** What a pane's header holds: what leads it, what follows on the line, and its keys. */
type PaneHead = { lead: ReactNode; line: ReactNode; keys: ReactNode };

/**
 * The frame every docked pane is drawn in. One component for the run and the shell, so the dock's tabs
 * stay mounted, and keep the focus, as the pane in front changes.
 */
function PaneFrame({ state, head, tabs, placed, children }: { state: string; head: PaneHead; tabs?: ReactNode; placed?: "panel"; children: ReactNode }) {
  return (
    <section className="armada-terminal" role="group" aria-label="Terminal" data-state={state} data-placed={placed}>
      <header className="armada-terminal__head">
        {tabs === undefined ? head.lead : <span className="armada-terminal__tabs">{tabs}</span>}
        <span className="armada-terminal__command">{head.line}</span>
        <span className="armada-terminal__keys">{head.keys}</span>
      </header>
      <div className="armada-terminal__body">{children}</div>
    </section>
  );
}

function runHead({ command, phase, onStop, onRerun, onClose }: Omit<TerminalPaneProps, "children">, tabbed: boolean): PaneHead {
  const mark = <TerminalMark phase={phase} />;
  return {
    lead: mark,
    // Beside the shell the command is its tab, and the run's mark follows the tabs.
    line: tabbed ? (
      mark
    ) : (
      <Tooltip label={command}>
        <code className="armada-terminal__text">{command}</code>
      </Tooltip>
    ),
    keys:
      phase.state === "running" ? (
        <Tooltip label="Stop">
          <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={onStop}>
            <Square size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      ) : (
        <>
          <Tooltip label="Run again">
            <Button variant="ghost" size="sm" iconOnly aria-label="Run again" onClick={onRerun}>
              <RotateCcw size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          <Tooltip label="Close">
            <Button variant="ghost" size="sm" iconOnly aria-label="Close" onClick={onClose}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </>
      ),
  };
}

/**
 * **A command running in a terminal of its own**, docked in a Session's view. The frame is this and the
 * body is the caller's, the terminal screen: the mark, the command on one line, and the keys that end
 * or repeat the run. Stop while it runs; Run again and Close once it has ended.
 */
export function TerminalPane({ children, ...run }: TerminalPaneProps) {
  return (
    <PaneFrame state={run.phase.state} head={runHead(run, false)}>
      {children}
    </PaneFrame>
  );
}

/** How a Job's shell stands. It ends without a code worth showing. */
export type ShellPhase = { state: "running" } | { state: "ended" };

export type ShellPaneProps = {
  phase: ShellPhase;
  /** Puts the pane away. The shell keeps running. */
  onHide: () => void;
  /** Stops the shell. */
  onEnd: () => void;
  /** Opens a new shell in place of the one that ended. */
  onOpen: () => void;
  /** Puts away the pane of a shell that has ended. */
  onClose: () => void;
  /** Moves the shell to the other place a Job's shell opens in, docked or the side panel. Absent where it has no other. */
  onMove?: () => void;
  /** The body: the screen the shell is drawn on. */
  children: ReactNode;
};

/** The shell in the side panel, which names it and closes it. */
export type ShellPanelPaneProps = Omit<ShellPaneProps, "onHide" | "onClose">;

/** An act drawn as its glyph alone, which the tooltip names. */
function IconAct({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return (
    <Tooltip label={label}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} onClick={onClick}>
        {children}
      </Button>
    </Tooltip>
  );
}

/** `docked` holds the pane's own way away, which a panel's shell leaves to the panel. */
function shellHead({ phase, onEnd, onOpen, onMove }: Omit<ShellPanelPaneProps, "children">, docked: Pick<ShellPaneProps, "onHide" | "onClose"> | null): PaneHead {
  const running = phase.state === "running";
  // The glyph pictures the place the shell goes.
  const move =
    onMove === undefined ? null : docked === null ? (
      <IconAct label="Dock under the thread" onClick={onMove}>
        <PanelBottom size={16} strokeWidth={2} aria-hidden />
      </IconAct>
    ) : (
      <IconAct label="Move to the side panel" onClick={onMove}>
        <PanelRight size={16} strokeWidth={2} aria-hidden />
      </IconAct>
    );
  return {
    lead: docked === null ? null : <span className="armada-terminal__name">Terminal</span>,
    line: running ? null : <span className="armada-terminal__said">Shell ended</span>,
    keys: running ? (
      <>
        {move}
        {docked === null ? null : (
          <IconAct label="Hide" onClick={docked.onHide}>
            <PanelBottomClose size={16} strokeWidth={2} aria-hidden />
          </IconAct>
        )}
        <IconAct label="End shell" onClick={onEnd}>
          <PowerOff size={16} strokeWidth={2} aria-hidden />
        </IconAct>
      </>
    ) : (
      <>
        {move}
        <Button variant="ghost" size="sm" onClick={onOpen}>
          New shell
        </Button>
        {docked === null ? null : (
          <IconAct label="Close" onClick={docked.onClose}>
            <X size={16} strokeWidth={2} aria-hidden />
          </IconAct>
        )}
      </>
    ),
  };
}

/**
 * **The Job's own shell**, which stays open and which the person and the Job's agent both type into.
 * Unlike a run it has no exit to mark and is never run again: Hide and End shell while it runs; once it
 * has ended, the pane says so and offers a new shell and Close. Either way, `onMove` moves it to the side panel.
 */
export function ShellPane({ children, ...shell }: ShellPaneProps) {
  return (
    <PaneFrame state={shell.phase.state} head={shellHead(shell, shell)}>
      {children}
    </PaneFrame>
  );
}

/**
 * **The Job's shell in the side panel**, filling it. The panel names it and its Close stands for Hide
 * and Close, so the pane keeps End shell while it runs and New shell once it has ended.
 */
export function ShellPanelPane({ children, ...shell }: ShellPanelPaneProps) {
  return (
    <PaneFrame state={shell.phase.state} head={shellHead(shell, null)} placed="panel">
      {children}
    </PaneFrame>
  );
}

/** One `--space-4` per arrow press, as a dock's handle moves. */
const HEIGHT_STEP = 16;
/** The least the pane keeps, and the least it leaves the thread above it. */
const HEIGHT_MIN = 96;
const THREAD_MIN = 160;

/**
 * **The pane docked at the foot of a view**, about two fifths of it, with a handle on its top edge. A
 * drag moves the edge; the arrow keys do too, and Home puts it back.
 */
export function TerminalDock(props: TerminalPaneProps) {
  return (
    <DockFrame>
      <TerminalPane {...props} />
    </DockFrame>
  );
}

/** Which pane of a Job's dock is in front. */
export type ShellDockPane = "shell" | "run";

export type ShellDockProps = {
  /** The Job's shell, while its pane is open. */
  shell: ShellPaneProps | null;
  /** A block's run, while its pane is open. */
  run: TerminalPaneProps | null;
  /** Which pane is drawn where both are open. */
  front: ShellDockPane;
  onFront: (pane: ShellDockPane) => void;
};

/** The most of a command its tab spells out. The tab strip wraps rather than cuts, so the label is cut here. */
const TAB_COMMAND_MOST = 32;

const tabCommand = (command: string) => (command.length <= TAB_COMMAND_MOST ? command : `${command.slice(0, TAB_COMMAND_MOST - 1).trimEnd()}…`);

/**
 * **A Job's docked place**, holding its shell, a block's run, or both. With both open they are tabs,
 * Terminal and the run's command, cut short where it is long and whole in the tab's tooltip; the pane
 * in front keeps its own keys. The pane behind is not mounted: its screen draws again from what it
 * printed when it comes back.
 */
export function ShellDock({ shell, run, front, onFront }: ShellDockProps) {
  const tabs =
    shell !== null && run !== null ? (
      <Tabs
        items={[
          { id: "shell", label: "Terminal" },
          { id: "run", label: tabCommand(run.command), hint: run.command },
        ]}
        value={front}
        onChange={(id) => onFront(id === "run" ? "run" : "shell")}
      />
    ) : undefined;
  let pane: { key: ShellDockPane; state: string; head: PaneHead; body: ReactNode };
  if (run !== null && (shell === null || front === "run")) {
    const { children, ...rest } = run;
    pane = { key: "run", state: rest.phase.state, head: runHead(rest, tabs !== undefined), body: children };
  } else if (shell !== null) {
    const { children, ...rest } = shell;
    pane = { key: "shell", state: rest.phase.state, head: shellHead(rest, rest), body: children };
  } else return null;
  return (
    <DockFrame>
      <PaneFrame state={pane.state} head={pane.head} tabs={tabs}>
        {/* Keyed, so a screen is never handed the other pane's output. */}
        <Fragment key={pane.key}>{pane.body}</Fragment>
      </PaneFrame>
    </DockFrame>
  );
}

function DockFrame({ children }: { children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  const drag = useRef<{ pointerId: number; startY: number; startHeight: number } | null>(null);
  const [height, setHeight] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);

  const room = () => {
    const parent = box.current?.parentElement;
    return parent === null || parent === undefined ? 600 : Math.max(HEIGHT_MIN, parent.clientHeight - THREAD_MIN);
  };
  const clamp = (value: number) => Math.min(room(), Math.max(HEIGHT_MIN, value));
  const now = () => box.current?.getBoundingClientRect().height ?? HEIGHT_MIN;

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startY: event.clientY, startHeight: now() };
    setDragging(true);
    event.currentTarget.focus();
    event.preventDefault();
  }
  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    setHeight(clamp(drag.current.startHeight + (drag.current.startY - event.clientY)));
  }
  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
  }
  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "ArrowUp") setHeight(clamp(now() + HEIGHT_STEP));
    else if (event.key === "ArrowDown") setHeight(clamp(now() - HEIGHT_STEP));
    else if (event.key === "Home") setHeight(null);
    else return;
    event.preventDefault();
  }

  return (
    <div ref={box} className="armada-terminal-dock" {...(height === null ? {} : { style: { flexBasis: height } })}>
      <div
        className="armada-terminal-dock__handle"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize terminal"
        aria-valuemin={HEIGHT_MIN}
        {...(height === null ? {} : { "aria-valuenow": Math.round(height) })}
        tabIndex={0}
        data-dragging={dragging || undefined}
        onDoubleClick={() => setHeight(null)}
        onPointerDown={pointerDown}
        onPointerMove={pointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onKeyDown={keyDown}
      />
      {children}
    </div>
  );
}
