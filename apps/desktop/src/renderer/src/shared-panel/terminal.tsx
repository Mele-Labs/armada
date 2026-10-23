// A Job's own shell in the side panel, where `terminal.openJobTerminalIn` says so. The shell is the window's (`job-shell.ts`), so
// Close only puts the panel away: the shell keeps running, and Terminal in the Job's header opens it again with all it printed.

import { useEffect, useRef } from "react";
import { Sheet, ShellPanelPane, XtermScreen } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { shellsOf, useShellOf } from "../job-shell";
import { saveJobTerminalPlace } from "../job-terminal-place";
import { useSessions } from "../sessions-draft";
import { useBack } from "./panels";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Item = Extract<PanelItem, { kind: "terminal" }>;

export function TerminalPanel({ item }: { item: Item }) {
  const shells = shellsOf(window.armada);
  const shell = useShellOf(shells, item.sessionId);
  const title = useSessions().find((one) => one.id === item.sessionId)?.title;
  const floor = useAtFloor();
  const back = useBack();
  const screen = useRef<HTMLDivElement>(null);
  // The sheet hands its Close the focus as it opens; typing goes to the shell, as it does where the shell docks.
  useEffect(() => {
    screen.current?.querySelector("textarea")?.focus({ preventScroll: true });
  }, []);
  if (shells === null || shell === undefined) return null;
  const running = !shell.ended;
  const { sessionId } = item;
  return (
    <Sheet
      kind="side-panel"
      open
      floating
      floor={floor}
      title="Terminal"
      {...(title === undefined ? {} : { subtitle: title })}
      size="panel"
      bleed
      {...(back === undefined ? {} : { back })}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={closePanel}
    >
      <ShellPanelPane
        phase={running ? { state: "running" } : { state: "ended" }}
        onEnd={() => shells.stop(sessionId)}
        onOpen={() => shells.fresh(sessionId)}
        moves={[
          {
            to: "docked",
            onMove: () => {
              saveJobTerminalPlace("docked");
              shells.show("page", sessionId, true);
              closePanel();
            },
          },
          {
            to: "window",
            onMove: () => {
              saveJobTerminalPlace("window");
              if (shells.toWindow(sessionId)) closePanel();
            },
          },
        ]}
      >
        {/* Esc typed into a running shell is the shell's, as vim and less take it. */}
        <div ref={screen} className="side-terminal" {...(running ? { "data-keeps-esc": "" } : {})}>
          <XtermScreen key={shell.screen} out={shell.out} run={shell.screen} running={running} write={(data) => shells.write(sessionId, data)} onSize={(cols, rows) => shells.sized(sessionId, cols, rows)} />
        </div>
      </ShellPanelPane>
    </Sheet>
  );
}
