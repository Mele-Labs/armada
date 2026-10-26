import { useEffect, useRef, useState } from "react";
import { ExternalLink, RotateCcw, Square } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { TabPanel, Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ConsoleOutput } from "../ConsoleOutput/ConsoleOutput";
import { RunPhaseMark, RunWindowMark } from "../RunControl/RunPhaseMark";
import { SETUP_SAID, linkBrief } from "../RunControl/runs";
import type { RunCheckout, RunLink, RunningApp } from "../RunControl/runs";
import { RunLinkLine, RunLinkMark, RunPinned } from "../RunLinkLine/RunLinkLine";

/** Setup's tab, kept apart from a process's id. */
const SETUP_TAB = ":setup";

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
 * log for each process under tabs. **Every row keeps its five columns**, so a desktop app, which has no address, shows its window mark where a
 * browser run has Open, and a process with no port leaves the column empty. **A link found** (a process that expects another on a fixed port it is not
 * on) puts its mark on that process's row and, under the rows, the fact with Fix with a Job and Pin the port. A process whose port is pinned shows the pin,
 * and where `onUnpin` is given a press on it undoes the pin. **A checkout that needed setup** lists its setup Commands above the processes in the same
 * columns, setup's output under a tab of its own before the processes', and the processes wait until it ends. A Command that failed stops the run:
 * the line under the rows names it with Restart.
 */
export function RunningPanel({
  app,
  onOpen,
  onFix,
  onPin,
  onUnpin,
  onRestart,
}: {
  app: RunningApp;
  onOpen: (url: string) => void;
  /** Fix with a Job, with the brief for it. */
  onFix?: (brief: string) => void;
  onPin?: (link: RunLink) => void;
  onUnpin?: (process: string) => void;
  /** Restart, from the line that names a failed setup Command. */
  onRestart?: () => void;
}) {
  const setup = app.setup;
  const settingUp = setup !== undefined && setup.steps.some((one) => one.phase !== "ready");
  const [shown, setShown] = useState(settingUp ? SETUP_TAB : app.processes[0]?.id);
  const log =
    setup !== undefined && shown === SETUP_TAB ? { id: SETUP_TAB, name: "Setup", log: setup.log } : (app.processes.find((one) => one.id === shown) ?? app.processes[0]);
  const failed = setup?.steps.find((one) => one.phase === "failed");
  const tail = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);
  // A log that is being read goes on following its end until it is scrolled up from there.
  useEffect(() => {
    const well = tail.current;
    if (well !== null && following) well.scrollTop = well.scrollHeight;
  }, [log?.log.length, log?.id, following]);
  const links = (app.links ?? []).flatMap((link) => {
    const from = app.processes.find((one) => one.id === link.from);
    const to = app.processes.find((one) => one.id === link.to);
    return from === undefined || to === undefined ? [] : [{ link, from, to }];
  });
  return (
    <div className="armada-running">
      <RunCheckoutFact checkout={app.checkout} />
      {setup === undefined ? null : (
        <>
          <h3 className="armada-running__caption">Setup</h3>
          <ul className="armada-running__rows" aria-label="Setup">
            {setup.steps.map((one) => (
              <li key={one.id} className="armada-running__row" aria-label={one.name}>
                <RunPhaseMark phase={one.phase} names={SETUP_SAID} />
                <span className="armada-running__name">{one.name}</span>
                <span />
                <span />
                <span />
              </li>
            ))}
          </ul>
          {failed === undefined ? null : (
            <div className="armada-running__failed" role="group" aria-label="Setup failed">
              <p className="armada-running__failed-fact">
                <code>{failed.command}</code>
                {failed.folder === "." ? null : (
                  <>
                    {" "}in <span className="armada-running__failed-name">{failed.folder}</span>
                  </>
                )}{" "}
                exited with code <code>{failed.code ?? 1}</code>
              </p>
              {onRestart === undefined ? null : (
                <Button variant="secondary" size="sm" onClick={onRestart}>
                  Restart
                </Button>
              )}
            </div>
          )}
          <h3 className="armada-running__caption">Processes</h3>
        </>
      )}
      <ul className="armada-running__rows" aria-label="Processes">
        {app.processes.map((one) => (
          <li key={one.id} className="armada-running__row" aria-label={one.name}>
            <RunPhaseMark phase={one.phase} />
            <span className="armada-running__name">{one.name}</span>
            <span className="armada-running__link">
              {links
                .filter((found) => found.from.id === one.id)
                .map(({ link, to }) => (
                  <RunLinkMark key={link.to} link={link} to={to.name} />
                ))}
            </span>
            {one.pinned === true && one.port !== undefined ? (
              <span className="armada-running__port">
                <RunPinned port={one.port} name={one.name} {...(onUnpin === undefined ? {} : { onUnpin: () => onUnpin(one.id) })} />
              </span>
            ) : (
              <code className="armada-running__port">{one.port ?? ""}</code>
            )}
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
      {links.length === 0 ? null : (
        <div className="armada-running__links">
          {links.map(({ link, from, to }) => (
            <RunLinkLine
              key={`${link.from}:${link.to}`}
              link={link}
              from={from.name}
              to={to.name}
              marked={false}
              onFix={() => onFix?.(linkBrief(link, from.name, to.name))}
              onPin={() => onPin?.(link)}
            />
          ))}
        </div>
      )}
      {app.processes.length < 2 && setup === undefined ? null : (
        <Tabs items={[...(setup === undefined ? [] : [{ id: SETUP_TAB, label: "Setup" }]), ...app.processes.map((one) => ({ id: one.id, label: one.name }))]} value={log?.id} onChange={setShown} />
      )}
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
