import { useEffect, useRef } from "react";
import { Trash2 } from "lucide-react";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { linkBrief, listens, portVariable } from "../RunControl/runs";
import type { RunDraft, RunProcessSpec, RunSetupSpec } from "../RunControl/runs";
import { RunLinkLine, RunPinned } from "../RunLinkLine/RunLinkLine";

/** A process as it is edited. Its port is not here: Fleet leases it, so there is nothing to type. */
type Row = { id: string; name: string; folder: string; command: string; pin?: number };

const rowOf = (one: RunProcessSpec): Row => ({ id: one.id, name: one.name, folder: one.folder, command: one.command, ...(one.pin === undefined ? {} : { pin: one.pin }) });
const specOf = (one: Row): RunProcessSpec => ({ id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim(), ...(one.pin === undefined ? {} : { pin: one.pin }) });
const setupOf = (one: RunSetupSpec): RunSetupSpec => ({ id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim() });
const filled = (one: { name: string; command: string }): boolean => one.name.trim() !== "" && one.command.trim() !== "";

/** Where a folder is on disk: the root with the folder under it, `.` and an empty field being the root itself. An absolute or `~` folder stands as typed. */
function fullPath(root: string, folder: string): string {
  const typed = folder.trim();
  if (typed.startsWith("/") || typed.startsWith("~")) return typed;
  const parts = typed.split("/").filter((part) => part !== "" && part !== ".");
  return [root.replace(/\/+$/, ""), ...parts].join("/");
}

/**
 * The first run of a repository that has none saved: the run Fleet drafted from what Jobs ran there and from the repository's files,
 * each process's name, folder and command editable, with a process added or removed, and its leased port shown as a fact. **One act, Save and run**, and where it saves
 * is a fact under the processes. **A link Fleet found** (one process expecting another on a fixed port) is a fact under the rows with two acts, Fix with a Job and
 * Pin the port, which shows on the row as the port and a pin in place of `leased`; neither is needed to save. What is typed is kept under `keep` until it is saved. **`name` present adds a field for the run's own name**,
 * for a repository that already has runs and is given another. **The repository's setup** (the Commands that make a checkout ready to run) is listed
 * above the processes and edited the same way; Save and run hands it back with them.
 */
export function RunConfirm({
  open,
  repository,
  draft,
  name,
  keep,
  onSave,
  onCancel,
  onFix,
}: {
  open: boolean;
  repository: string;
  draft: RunDraft;
  name?: string;
  /** What the typed fields are remembered under, so leaving the confirm and coming back finds them. */
  keep: string;
  onSave: (processes: readonly RunProcessSpec[], name: string, setup: readonly RunSetupSpec[]) => void;
  onCancel: () => void;
  /** Fix with a Job, with the brief for it. */
  onFix?: (brief: string) => void;
}) {
  const [rows, setRows, forget] = useKept<Row[]>(`run-confirm:${keep}`, draft.processes.map(rowOf));
  const [runName, setRunName, forgetName] = useKept<string>(`run-confirm-name:${keep}`, name ?? "");
  const [setup, setSetup, forgetSetup] = useKept<RunSetupSpec[]>(`run-confirm-setup:${keep}`, [...(draft.setup ?? [])]);
  const first = useRef<HTMLInputElement>(null);
  // The dialog gives Cancel the first focus; a form that collects is typed into, so the first field takes it back.
  useEffect(() => {
    if (!open) return;
    const later = setTimeout(() => first.current?.focus(), 0);
    return () => clearTimeout(later);
  }, [open]);
  const change = (id: string, field: keyof Omit<Row, "id">, value: string) => setRows((was) => was.map((one) => (one.id === id ? { ...one, [field]: value } : one)));
  const pin = (id: string, port: number | undefined) => setRows((was) => was.map((one) => (one.id !== id ? one : port === undefined ? { id: one.id, name: one.name, folder: one.folder, command: one.command } : { ...one, pin: port })));
  const changeSetup = (id: string, field: keyof Omit<RunSetupSpec, "id">, value: string) => setSetup((was) => was.map((one) => (one.id === id ? { ...one, [field]: value } : one)));
  const ready = (name === undefined || runName.trim() !== "") && rows.length > 0 && rows.every(filled) && setup.every(filled);
  // A link is shown while both its processes are rows, under the names they have now.
  const links = (draft.links ?? []).flatMap((link) => {
    const from = rows.find((one) => one.id === link.from);
    const to = rows.find((one) => one.id === link.to);
    return from === undefined || to === undefined || from.name.trim() === "" || to.name.trim() === "" ? [] : [{ link, from, to }];
  });
  return (
    <Dialog
      open={open}
      title={`Run ${repository}`}
      tone="neutral"
      width="medium"
      placement="high"
      confirmLabel="Save and run"
      confirmDisabled={!ready}
      onConfirm={() => {
        onSave(rows.map(specOf), runName.trim(), setup.map(setupOf));
        forget();
        forgetName();
        forgetSetup();
      }}
      onCancel={onCancel}
    >
      <div className="armada-run-confirm">
        {name === undefined ? null : <Input ref={first} label="Run name" aria-label="Run name" value={runName} onChange={(event) => setRunName(event.target.value)} />}
        {setup.length === 0 ? null : (
          <>
            <h3 className="armada-run-confirm__caption">Setup</h3>
            <ul className="armada-run-confirm__rows" aria-label="Setup">
              {setup.map((step, at) => (
                <li key={step.id} className="armada-run-confirm__row">
                  <Input ref={at === 0 && name === undefined ? first : undefined} label="Name" aria-label={`Setup ${at + 1} name`} value={step.name} onChange={(event) => changeSetup(step.id, "name", event.target.value)} />
                  <Input label="Folder" aria-label={`Setup ${at + 1} folder`} mono value={step.folder} onChange={(event) => changeSetup(step.id, "folder", event.target.value)} />
                  <span />
                  <Tooltip label="Remove">
                    <Button variant="ghost" size="sm" iconOnly aria-label={`Remove setup ${at + 1}`} onClick={() => setSetup((was) => was.filter((one) => one.id !== step.id))}>
                      <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                  <code className="armada-run-confirm__path">{fullPath(draft.root, step.folder)}</code>
                  <div className="armada-run-confirm__command">
                    <Input label="Command" aria-label={`Setup ${at + 1} command`} mono value={step.command} onChange={(event) => changeSetup(step.id, "command", event.target.value)} />
                  </div>
                </li>
              ))}
            </ul>
            <h3 className="armada-run-confirm__caption">Processes</h3>
          </>
        )}
        <ul className="armada-run-confirm__rows" aria-label="Processes">
          {rows.map((row, at) => (
            <li key={row.id} className="armada-run-confirm__row">
              <Input ref={at === 0 && name === undefined && setup.length === 0 ? first : undefined} label="Name" aria-label={`Process ${at + 1} name`} value={row.name} onChange={(event) => change(row.id, "name", event.target.value)} />
              <Input label="Folder" aria-label={`Process ${at + 1} folder`} mono value={row.folder} onChange={(event) => change(row.id, "folder", event.target.value)} />
              <div className="armada-run-confirm__port">
                <span className="armada-run-confirm__port-label">Port</span>
                <span className="armada-run-confirm__port-value">
                  {row.pin !== undefined ? (
                    <RunPinned port={row.pin} name={row.name.trim()} onUnpin={() => pin(row.id, undefined)} />
                  ) : listens(row) ? (
                    <Tooltip label={`The other processes read it as ${portVariable(row.name)}`}>
                      <span tabIndex={0}>leased</span>
                    </Tooltip>
                  ) : null}
                </span>
              </div>
              <Tooltip label="Remove">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Remove process ${at + 1}`} onClick={() => setRows((was) => was.filter((one) => one.id !== row.id))}>
                  <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
                </Button>
              </Tooltip>
              <code className="armada-run-confirm__path">{fullPath(draft.root, row.folder)}</code>
              <div className="armada-run-confirm__command">
                <Input label="Command" aria-label={`Process ${at + 1} command`} mono value={row.command} onChange={(event) => change(row.id, "command", event.target.value)} />
              </div>
            </li>
          ))}
        </ul>
        {links.length === 0 ? null : (
          <div className="armada-run-confirm__links">
            {links.map(({ link, from, to }) => (
              <RunLinkLine
                key={`${link.from}:${link.to}`}
                link={link}
                from={from.name.trim()}
                to={to.name.trim()}
                pinned={to.pin === link.port}
                onFix={() => onFix?.(linkBrief(link, from.name.trim(), to.name.trim()))}
                onPin={() => pin(to.id, to.pin === link.port ? undefined : link.port)}
              />
            ))}
          </div>
        )}
        <div className="armada-run-confirm__foot">
          <span className="armada-run-confirm__adds">
            <Button variant="secondary" size="sm" onClick={() => setRows((was) => [...was, { id: `new-${was.length + 1}-${Date.now()}`, name: "", folder: ".", command: "" }])}>
              Add process
            </Button>
            <Button variant="secondary" size="sm" onClick={() => setSetup((was) => [...was, { id: `setup-new-${was.length + 1}-${Date.now()}`, name: "", folder: ".", command: "" }])}>
              Add setup Command
            </Button>
          </span>
          <p className="armada-run-confirm__where">
            <span className="armada-run-confirm__where-label">Saved in</span>
            <code>{draft.savedIn}</code>
          </p>
        </div>
      </div>
    </Dialog>
  );
}
