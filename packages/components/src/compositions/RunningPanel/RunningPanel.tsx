import { useEffect, useRef, useState } from "react";
import { ExternalLink, RotateCcw, Square } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { TabPanel, Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ConsoleOutput } from "../ConsoleOutput/ConsoleOutput";
import { RunPhaseMark, RunWindowMark } from "../RunControl/RunPhaseMark";
import type { RunCheckout, RunningApp } from "../RunControl/runs";

/** Which checkout a run is on, as a fact: `main`, or the Job and its branch. */
export function RunCheckoutFact({ checkout }: { checkout: RunCheckout }) {
  return (
    <dl className="armada-running__facts">
      <div className="armada-running__fact">
        <dt>{checkout.kind === "main" ? "On" : "Job"}</dt>
        <dd>{checkout.kind === "main" ? "main" : checkout.title}</dd>
      </div>
      {checkout.kind === "main" ? null : (
        <>
          <div className="armada-running__fact">
            <dt>Branch</dt>
            <dd data-mono="">{checkout.branch}</dd>
          </div>
          <div className="armada-running__fact">
            <dt>Bay</dt>
            <dd data-mono="">{checkout.bay}</dd>
          </div>
        </>
      )}
    </dl>
  );
}

/** Restart and Stop for the whole run, for the head of the pane that holds it. */
export function RunActs({ onRestart, onStop, stoppable = true }: { onRestart: () => void; onStop: () => void; stoppable?: boolean }) {
  return (
    <>
      <Tooltip label="Restart" shortcut="r">
        <Button variant="ghost" size="sm" iconOnly aria-label="Restart" onClick={onRestart}>
          <RotateCcw size={16} strokeWidth={2} aria-hidden="true" />
        </Button>
      </Tooltip>
      <Tooltip label="Stop" shortcut="s">
        <Button variant="ghost" size="sm" iconOnly aria-label="Stop" disabled={!stoppable} onClick={onStop}>
          <Square size={16} strokeWidth={2} aria-hidden="true" />
        </Button>
      </Tooltip>
    </>
  );
}

/**
 * A run that is starting or running: the checkout it is on, a row for each process with its state, its port and the way to open it, and a
 * log for each process under tabs. **Every row keeps its four columns**, so a desktop app, which has no address, shows its window mark where a
 * browser run has Open, and a process with no port leaves the column empty.
 */
export function RunningPanel({ app, onOpen }: { app: RunningApp; onOpen: (url: string) => void }) {
  const [shown, setShown] = useState(app.processes[0]?.id);
  const log = app.processes.find((one) => one.id === shown) ?? app.processes[0];
  const tail = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);
  // A log that is being read goes on following its end until it is scrolled up from there.
  useEffect(() => {
    const well = tail.current;
    if (well !== null && following) well.scrollTop = well.scrollHeight;
  }, [log?.log.length, log?.id, following]);
  return (
    <div className="armada-running">
      <RunCheckoutFact checkout={app.checkout} />
      <ul className="armada-running__rows" aria-label="Processes">
        {app.processes.map((one) => (
          <li key={one.id} className="armada-running__row" aria-label={one.name}>
            <RunPhaseMark phase={one.phase} />
            <span className="armada-running__name">{one.name}</span>
            <code className="armada-running__port">{one.port ?? ""}</code>
            <span className="armada-running__open">
              {app.kind === "app" ? (
                <RunWindowMark phase={one.phase} />
              ) : one.url === undefined || one.phase !== "ready" ? null : (
                <Tooltip label={`Open ${one.url}`} shortcut="o">
                  <Button variant="ghost" size="sm" iconOnly aria-label={`Open ${one.name}`} onClick={() => onOpen(one.url!)}>
                    <ExternalLink size={16} strokeWidth={2} aria-hidden="true" />
                  </Button>
                </Tooltip>
              )}
            </span>
          </li>
        ))}
      </ul>
      {app.processes.length < 2 ? null : <Tabs items={app.processes.map((one) => ({ id: one.id, label: one.name }))} value={log?.id} onChange={setShown} />}
      <div
        ref={tail}
        className="armada-running__well"
        tabIndex={0}
        role="region"
        aria-label={log === undefined ? "Log" : `${log.name} log`}
        onScroll={(event) => {
          const well = event.currentTarget;
          setFollowing(well.scrollHeight - well.scrollTop - well.clientHeight < 24);
        }}
      >
        <TabPanel tab={log?.id} role="tabpanel">
          {log === undefined ? null : <ConsoleOutput following rows={log.log.map((text, at) => ({ row: "line" as const, at: at + 1, text }))} />}
        </TabPanel>
      </div>
    </div>
  );
}
