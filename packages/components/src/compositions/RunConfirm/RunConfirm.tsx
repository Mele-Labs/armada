import { useEffect, useRef } from "react";
import { Trash2 } from "lucide-react";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { RunDraft, RunProcessSpec } from "../RunControl/runs";

/** A process as it is edited: the port is the text in the field until the run is saved. */
type Row = { id: string; name: string; folder: string; command: string; port: string };

const rowOf = (one: RunProcessSpec): Row => ({ id: one.id, name: one.name, folder: one.folder, command: one.command, port: one.port === undefined ? "" : String(one.port) });
const specOf = (one: Row): RunProcessSpec => {
  const port = Number(one.port);
  return { id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim(), ...(one.port.trim() === "" || Number.isNaN(port) ? {} : { port }) };
};

/**
 * The first run of a repository that has none saved: the run Fleet drafted from what Jobs ran there and from the repository's files,
 * each process's name, folder, command and port editable, with a process added or removed. **One act, Save and run**, and where it saves
 * is a fact under the processes. What is typed is kept under `keep` until it is saved.
 */
export function RunConfirm({
  open,
  repository,
  draft,
  onSave,
  onCancel,
}: {
  open: boolean;
  repository: string;
  draft: RunDraft;
  onSave: (processes: readonly RunProcessSpec[]) => void;
  onCancel: () => void;
}) {
  const [rows, setRows, forget] = useKept<Row[]>(`run-confirm:${repository}`, draft.processes.map(rowOf));
  const first = useRef<HTMLInputElement>(null);
  // The dialog gives Cancel the first focus; a form that collects is typed into, so the first field takes it back.
  useEffect(() => {
    if (!open) return;
    const later = setTimeout(() => first.current?.focus(), 0);
    return () => clearTimeout(later);
  }, [open]);
  const change = (id: string, field: keyof Omit<Row, "id">, value: string) => setRows((was) => was.map((one) => (one.id === id ? { ...one, [field]: value } : one)));
  const ready = rows.length > 0 && rows.every((one) => one.name.trim() !== "" && one.command.trim() !== "");
  return (
    <Dialog
      open={open}
      title={`Run ${repository}`}
      tone="neutral"
      width="wide"
      confirmLabel="Save and run"
      confirmDisabled={!ready}
      onConfirm={() => {
        onSave(rows.map(specOf));
        forget();
      }}
      onCancel={onCancel}
    >
      <div className="armada-run-confirm">
        <ul className="armada-run-confirm__rows">
          {rows.map((row, at) => (
            <li key={row.id} className="armada-run-confirm__row">
              <Input ref={at === 0 ? first : undefined} label="Name" aria-label={`Process ${at + 1} name`} value={row.name} onChange={(event) => change(row.id, "name", event.target.value)} />
              <Input label="Folder" aria-label={`Process ${at + 1} folder`} mono value={row.folder} onChange={(event) => change(row.id, "folder", event.target.value)} />
              <Input label="Port" aria-label={`Process ${at + 1} port`} mono inputMode="numeric" value={row.port} onChange={(event) => change(row.id, "port", event.target.value.replace(/\D/g, ""))} />
              <Tooltip label="Remove">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Remove process ${at + 1}`} onClick={() => setRows((was) => was.filter((one) => one.id !== row.id))}>
                  <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
                </Button>
              </Tooltip>
              <div className="armada-run-confirm__command">
                <Input label="Command" aria-label={`Process ${at + 1} command`} mono value={row.command} onChange={(event) => change(row.id, "command", event.target.value)} />
              </div>
            </li>
          ))}
        </ul>
        <div className="armada-run-confirm__foot">
          <Button variant="secondary" size="sm" onClick={() => setRows((was) => [...was, { id: `new-${was.length + 1}-${Date.now()}`, name: "", folder: ".", command: "", port: "" }])}>
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
