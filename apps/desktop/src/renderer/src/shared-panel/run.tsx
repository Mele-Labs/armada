// A command run in a repository's checkout with no Job behind it, in the side panel: a Commands mod's shell row. The run starts when
// the panel opens and is the panel's own (`terminal-run.ts`), so closing the panel stops it. Apart from `terminal.tsx`, whose shell is a Job's.

import { useEffect, useRef } from "react";
import { Sheet, ShellPanelPane, XtermScreen } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { useBack } from "./panels";
import { usePanelReads } from "./reads";
import { closePanel } from "./store";
import type { PanelItem } from "./store";
import { useTerminalRun } from "./terminal-run";

type Item = Extract<PanelItem, { kind: "run" }>;

export function RunPanel({ item }: { item: Item }) {
  const reads = usePanelReads();
  const run = useTerminalRun(reads?.terminalRun, item.dir);
  const floor = useAtFloor();
  const back = useBack();
  const screen = useRef<HTMLDivElement>(null);
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    run.start(item.command);
  });
  // The sheet hands its Close the focus as it opens; typing goes to the run, as it does in a Job's shell.
  useEffect(() => {
    screen.current?.querySelector("textarea")?.focus({ preventScroll: true });
  }, [run.run?.screen]);
  const running = run.run?.running === true;
  return (
    <Sheet
      kind="side-panel"
      open
      floating
      floor={floor}
      title="Terminal"
      subtitle={`${item.repository} · ${item.command}`}
      size="panel"
      bleed
      {...(back === undefined ? {} : { back })}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={closePanel}
    >
      <ShellPanelPane phase={running || run.run === undefined ? { state: "running" } : { state: "ended" }} onEnd={run.stop} onOpen={() => run.start(item.command)}>
        <div ref={screen} className="side-terminal" {...(running ? { "data-keeps-esc": "" } : {})}>
          {run.run === undefined ? null : <XtermScreen key={run.run.screen} out={run.run.out} run={run.run.screen} running={running} write={run.write} onSize={run.sized} />}
          {run.said === undefined ? null : <p className="armada-pr-panel__said" role="alert">{run.said}</p>}
        </div>
      </ShellPanelPane>
    </Sheet>
  );
}
