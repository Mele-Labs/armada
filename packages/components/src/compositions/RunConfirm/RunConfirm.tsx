import { useEffect, useRef } from "react";
import { Trash2 } from "lucide-react";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { listens } from "../RunControl/runs";
import type { RunDraft, RunProcessSpec } from "../RunControl/runs";

/** A process as it is edited. Its port is not here: Fleet leases it, so there is nothing to type. */
type Row = { id: string; name: string; folder: string; command: string };

const rowOf = (one: RunProcessSpec): Row => ({ id: one.id, name: one.name, folder: one.folder, command: one.command });
const specOf = (one: Row): RunProcessSpec => ({ id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim() });

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
 * is a fact under the processes. What is typed is kept under `keep` until it is saved. **`name` present adds a field for the run's own name**,
 * for a repository that already has runs and is given another.
 */
export function RunConfirm({
  open,
  repository,
  draft,
  name,
  keep,
  onSave,
  onCancel,
}: {
  open: boolean;
  repository: string;
  draft: RunDraft;
  name?: string;
  /** What the typed fields are remembered under, so leaving the confirm and coming back finds them. */
  keep: string;
  onSave: (processes: readonly RunProcessSpec[], name: string) => void;
  onCancel: () => void;
}) {
  const [rows, setRows, forget] = useKept<Row[]>(`run-confirm:${keep}`, draft.processes.map(rowOf));
  const [runName, setRunName, forgetName] = useKept<string>(`run-confirm-name:${keep}`, name ?? "");
  const first = useRef<HTMLInputElement>(null);
  // The dialog gives Cancel the first focus; a form that collects is typed into, so the first field takes it back.
  useEffect(() => {
    if (!open) return;
    const later = setTimeout(() => first.current?.focus(), 0);
    return () => clearTimeout(later);
  }, [open]);
  const change = (id: string, field: keyof Omit<Row, "id">, value: string) => setRows((was) => was.map((one) => (one.id === id ? { ...one, [field]: value } : one)));
  const ready = (name === undefined || runName.trim() !== "") && rows.length > 0 && rows.every((one) => one.name.trim() !== "" && one.command.trim() !== "");
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
        onSave(rows.map(specOf), runName.trim());
        forget();
        forgetName();
      }}
      onCancel={onCancel}
    >
      <div className="armada-run-confirm">
        {name === undefined ? null : <Input ref={first} label="Run name" aria-label="Run name" value={runName} onChange={(event) => setRunName(event.target.value)} />}
        <ul className="armada-run-confirm__rows">
          {rows.map((row, at) => (
            <li key={row.id} className="armada-run-confirm__row">
              <Input ref={at === 0 && name === undefined ? first : undefined} label="Name" aria-label={`Process ${at + 1} name`} value={row.name} onChange={(event) => change(row.id, "name", event.target.value)} />
              <Input label="Folder" aria-label={`Process ${at + 1} folder`} mono value={row.folder} onChange={(event) => change(row.id, "folder", event.target.value)} />
              <div className="armada-run-confirm__port">
                <span className="armada-run-confirm__port-label">Port</span>
                <span className="armada-run-confirm__port-value">
                  {listens(row) ? (
                    <Tooltip label={`The other processes read it as ARMADA_PORT_${row.name.trim().toUpperCase().replace(/[^A-Z0-9]+/g, "_") || "NAME"}`}>
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
        <div className="armada-run-confirm__foot">
          <Button variant="secondary" size="sm" onClick={() => setRows((was) => [...was, { id: `new-${was.length + 1}-${Date.now()}`, name: "", folder: ".", command: "" }])}>
            Add process
          </Button>
          <p className="armada-run-confirm__where">
            <span className="armada-run-confirm__where-label">Saved in</span>
            <code>{draft.savedIn}</code>
          </p>
        </div>
      </div>
    </Dialog>
  );
}
