import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { ExternalLink, RotateCcw, Settings2, Square, Trash2 } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { TabPanel, Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ConsoleOutput } from "../ConsoleOutput/ConsoleOutput";
import { RunPhaseMark, RunWindowMark } from "../RunControl/RunPhaseMark";
import { SETUP_SAID, brokeOff, failureBrief, linkBrief, listens, portVariable } from "../RunControl/runs";
import type { RunCheckout, RunLink, RunningApp, RunProcess, RunSetupStep } from "../RunControl/runs";
import { RunLinkLine, RunLinkMark, RunPinned } from "../RunLinkLine/RunLinkLine";
import { fullPath, type RunEdit, type RunEditList, type RunEditRow } from "./RunEdit";

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

/** Edit the run, Restart and Stop for the whole run, for the head of the pane that holds it. */
export function RunActs({ onEdit, onRestart, onStop, stoppable = true }: { onEdit?: () => void; onRestart: () => void; onStop: () => void; stoppable?: boolean }) {
  return (
    <>
      {onEdit === undefined ? null : (
        <Tooltip label="Edit the run" shortcut="e">
          <Button variant="ghost" size="sm" iconOnly aria-label="Edit the run" onClick={onEdit}>
            <Settings2 size={16} strokeWidth={2} aria-hidden="true" />
          </Button>
        </Tooltip>
      )}
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

/** The run edited in place, and the two acts at the panel's foot. `save` is the primary act's label: Save and run, or Save and restart. */
export type RunPanelEdit = { run: RunEdit; save: string; onSave: () => void; onCancel: () => void };

/**
 * Enter in a field saves an edit that is ready, and Escape anywhere in it cancels. Handed to whatever holds the run's fields, the panel and the
 * head its name is in, and marks the event handled so a holder of both acts once.
 */
export function editKeys(edit: RunPanelEdit | undefined, event: KeyboardEvent): void {
  if (edit === undefined || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.key === "Escape") {
    event.preventDefault();
    edit.onCancel();
  } else if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
    event.preventDefault();
    if (edit.run.ready) edit.onSave();
  }
}

/**
 * A run that is starting or running: the checkout it is on, a row for each process with its state, its port and the way to open it, and a
 * log for each process under tabs. **Every row keeps its five columns**, so a desktop app, which has no address, shows its window mark where a
 * browser run has Open, and a process with no port leaves the column empty. **A link found** (a process that expects another on a fixed port it is not
 * on) puts its mark on that process's row and, under the rows, the fact with Fix with a Job and Pin the port. A process whose port is pinned shows the pin,
 * and where `onUnpin` is given a press on it undoes the pin. **A checkout that needed setup** lists its setup Commands above the processes in the same
 * columns, setup's output under a tab of its own before the processes', and the processes wait until it ends. **A Command that failed stops the run**
 * and says its exit code on its row, with Try again and Fix with a Job in an act column every row then keeps; a process that broke off has Fix with a Job
 * there too. Fix with a Job hands `onFix` a brief with what failed, its last lines and every log file.
 *
 * **`edit` turns the rows into fields in place**: each setup Command's and process's name, folder (its full path under it) and command, the port
 * pin, a way to remove the row, and Add setup Command and Add process under them. The state marks, a failed row's exit code and acts, and the logs
 * stay, so a failed Command's output sits beside the command being fixed. The links found are read from the draft and pinned on the rows; where
 * the run saves is a fact at the foot, beside Cancel and the save act. **`app` absent is a run not started yet**: the draft alone, on `checkout`.
 */
export function RunningPanel({
  app,
  checkout,
  repository,
  edit,
  onOpen,
  onFix,
  onPin,
  onUnpin,
  onTryAgain,
}: {
  app?: RunningApp;
  /** The checkout a run not started yet will start on, where `app` is absent. */
  checkout?: RunCheckout;
  /** The repository the run is of, as a Fix with a Job brief names it. */
  repository?: string;
  edit?: RunPanelEdit;
  onOpen: (url: string) => void;
  /** Fix with a Job, with the brief for it. */
  onFix?: (brief: string) => void;
  onPin?: (link: RunLink) => void;
  onUnpin?: (process: string) => void;
  /** Try again, on a failed setup Command's row: setup again from that Command, then the processes. */
  onTryAgain?: () => void;
}) {
  const setup = app?.setup;
  const processes = app?.processes ?? [];
  const settingUp = setup !== undefined && setup.steps.some((one) => one.phase !== "ready");
  const [shown, setShown] = useState(settingUp ? SETUP_TAB : processes[0]?.id);
  const log = setup !== undefined && shown === SETUP_TAB ? { id: SETUP_TAB, name: "Setup", log: setup.log } : (processes.find((one) => one.id === shown) ?? processes[0]);
  const failed = setup?.steps.find((one) => one.phase === "failed");
  // An act column is drawn on every row once any row has acts, so they line up; a run with none keeps its five columns.
  const acting = app !== undefined && onFix !== undefined && (failed !== undefined || processes.some(brokeOff));
  const fix = (brief: Parameters<typeof failureBrief>[0]["failed"]) => (app === undefined ? undefined : onFix?.(failureBrief({ repository: repository ?? app.run, app, failed: brief })));
  const exit = (code: number | undefined) => (code === undefined ? null : <span className="armada-running__exit">exited with code {code}</span>);
  const tail = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);
  // A log that is being read goes on following its end until it is scrolled up from there.
  useEffect(() => {
    const well = tail.current;
    if (well !== null && following) well.scrollTop = well.scrollHeight;
  }, [log?.log.length, log?.id, following]);
  const links = (app?.links ?? []).flatMap((link) => {
    const from = processes.find((one) => one.id === link.from);
    const to = processes.find((one) => one.id === link.to);
    return from === undefined || to === undefined ? [] : [{ link, from, to }];
  });
  const run = edit?.run;
  // While editing, a link is read from the draft and shown while both its processes are rows, under the names they have now.
  const edited = (run?.draft.links ?? []).flatMap((link) => {
    const from = run?.processes.find((one) => one.id === link.from);
    const to = run?.processes.find((one) => one.id === link.to);
    return from === undefined || to === undefined || from.name.trim() === "" || to.name.trim() === "" ? [] : [{ link, from, to }];
  });
  const setupActs = (one: RunSetupStep | undefined) =>
    !acting ? null : (
      <span className="armada-running__acts">
        {one?.phase !== "failed" ? null : (
          <>
            {onTryAgain === undefined ? null : (
              <Button variant="secondary" size="sm" onClick={onTryAgain}>
                Try again
              </Button>
            )}
            <Button variant="secondary" size="sm" onClick={() => fix({ kind: "setup", step: one })}>
              Fix with a Job
            </Button>
          </>
        )}
      </span>
    );
  const processActs = (one: RunProcess | undefined) =>
    !acting ? null : (
      <span className="armada-running__acts">
        {one !== undefined && brokeOff(one) ? (
          <Button variant="secondary" size="sm" onClick={() => fix({ kind: "process", process: one })}>
            Fix with a Job
          </Button>
        ) : null}
      </span>
    );
  const editRow = (list: RunEditList, row: RunEditRow, at: number) => {
    if (run === undefined) return null;
    const which = list === "setup" ? "Setup" : "Process";
    const step = list === "setup" ? setup?.steps.find((one) => one.id === row.id) : undefined;
    const process = list === "processes" ? processes.find((one) => one.id === row.id) : undefined;
    const state = step ?? process;
    const ended = step?.phase === "failed" ? step.code : process !== undefined && brokeOff(process) ? process.code : undefined;
    return (
      <li key={row.id} className="armada-running__row" data-edit="" aria-label={row.name.trim() === "" ? `${which} ${at + 1}` : row.name.trim()}>
        <span className="armada-running__mark">{state === undefined ? null : <RunPhaseMark phase={state.phase} {...(list === "setup" ? { names: SETUP_SAID } : {})} />}</span>
        <span className="armada-running__field" data-at="name">
          <Input aria-label={`${which} ${at + 1} name`} placeholder="Name" value={row.name} onChange={(event) => run.change(list, row.id, "name", event.target.value)} />
        </span>
        <span className="armada-running__field" data-at="folder">
          <Input aria-label={`${which} ${at + 1} folder`} placeholder="Folder" mono value={row.folder} onChange={(event) => run.change(list, row.id, "folder", event.target.value)} />
        </span>
        <span className="armada-running__field" data-at="command">
          <Input aria-label={`${which} ${at + 1} command`} placeholder="Command" mono value={row.command} onChange={(event) => run.change(list, row.id, "command", event.target.value)} />
        </span>
        <span className="armada-running__link">
          {edited
            .filter((found) => found.from.id === row.id && found.to.pin !== found.link.port)
            .map(({ link, to }) => (
              <RunLinkMark key={link.to} link={link} to={to.name.trim()} />
            ))}
        </span>
        <span className="armada-running__port">
          {row.pin !== undefined ? (
            <RunPinned port={row.pin} name={row.name.trim()} onUnpin={() => run.pin(row.id, undefined)} />
          ) : list === "processes" && listens(row) ? (
            <Tooltip label={`The other processes read it as ${portVariable(row.name)}`}>
              <span tabIndex={0}>{process?.port ?? "leased"}</span>
            </Tooltip>
          ) : null}
        </span>
        <span className="armada-running__remove">
          <Tooltip label="Remove">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Remove ${which.toLowerCase()} ${at + 1}`} onClick={() => run.remove(list, row.id)}>
              <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
            </Button>
          </Tooltip>
        </span>
        {list === "setup" ? setupActs(step) : processActs(process)}
        <span className="armada-running__under">{exit(ended)}</span>
        <code className="armada-running__path">{fullPath(run.draft.root, row.folder)}</code>
      </li>
    );
  };
  const facts = app?.checkout ?? checkout;
  const tabs = [...(setup === undefined ? [] : [{ id: SETUP_TAB, label: "Setup" }]), ...processes.map((one) => ({ id: one.id, label: one.name }))];
  const hasSetup = run === undefined ? setup !== undefined : run.setup.length > 0;
  return (
    <div className="armada-running" data-acts={acting ? "" : undefined} data-editing={run === undefined ? undefined : ""} onKeyDown={(event) => editKeys(edit, event)}>
      <div className="armada-running__top">
        {facts === undefined ? null : <RunCheckoutFact checkout={facts} />}
        {!hasSetup ? null : (
          <>
            <h3 className="armada-running__caption">Setup</h3>
            <ul className="armada-running__rows" aria-label="Setup">
              {run !== undefined
                ? run.setup.map((row, at) => editRow("setup", row, at))
                : setup?.steps.map((one) => (
                    <li key={one.id} className="armada-running__row" aria-label={one.name}>
                      <RunPhaseMark phase={one.phase} names={SETUP_SAID} />
                      <span className="armada-running__name">
                        {one.name}
                        {one.phase === "failed" ? exit(one.code) : null}
                      </span>
                      <span />
                      <span />
                      <span />
                      {setupActs(one)}
                    </li>
                  ))}
            </ul>
            <h3 className="armada-running__caption">Processes</h3>
          </>
        )}
        <ul className="armada-running__rows" aria-label="Processes">
          {run !== undefined
            ? run.processes.map((row, at) => editRow("processes", row, at))
            : processes.map((one) => (
                <li key={one.id} className="armada-running__row" aria-label={one.name}>
                  <RunPhaseMark phase={one.phase} />
                  <span className="armada-running__name">
                    {one.name}
                    {brokeOff(one) ? exit(one.code) : null}
                  </span>
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
                    {app?.kind === "app" ? (
                      <RunWindowMark phase={one.phase} />
                    ) : one.url === undefined || one.phase !== "ready" ? null : (
                      <Tooltip label={`Open ${one.url}`} shortcut="o">
                        <Button variant="ghost" size="sm" iconOnly aria-label={`Open ${one.name}`} onClick={() => onOpen(one.url!)}>
                          <ExternalLink size={16} strokeWidth={2} aria-hidden="true" />
                        </Button>
                      </Tooltip>
                    )}
                  </span>
                  {processActs(one)}
                </li>
              ))}
        </ul>
        {run === undefined ? null : (
          <span className="armada-running__adds">
            <Button variant="secondary" size="sm" onClick={() => run.add("setup")}>
              Add setup Command
            </Button>
            <Button variant="secondary" size="sm" onClick={() => run.add("processes")}>
              Add process
            </Button>
          </span>
        )}
        {run !== undefined ? (
          edited.length === 0 ? null : (
            <div className="armada-running__links">
              {edited.map(({ link, from, to }) => (
                <RunLinkLine
                  key={`${link.from}:${link.to}`}
                  link={link}
                  from={from.name.trim()}
                  to={to.name.trim()}
                  marked={false}
                  pinned={to.pin === link.port}
                  onFix={() => onFix?.(linkBrief(link, from.name.trim(), to.name.trim()))}
                  onPin={() => run.pin(to.id, to.pin === link.port ? undefined : link.port)}
                />
              ))}
            </div>
          )
        ) : links.length === 0 ? null : (
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
      </div>
      {app === undefined ? null : (
        <>
          {processes.length < 2 && setup === undefined ? null : <Tabs items={tabs} value={log?.id} onChange={setShown} />}
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
        </>
      )}
      {edit === undefined || run === undefined ? null : (
        <div className="armada-running__foot">
          <p className="armada-running__where">
            <span className="armada-running__where-label">Saved in</span>
            <code>{run.draft.savedIn}</code>
          </p>
          <Button variant="secondary" size="sm" onClick={edit.onCancel}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" disabled={!run.ready} onClick={edit.onSave}>
            {edit.save}
          </Button>
        </div>
      )}
    </div>
  );
}
