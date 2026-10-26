import { useEffect, useRef } from "react";

import { useKept } from "../../keep";
import { Input } from "../../primitives/Input/Input";
import type { RunDraft, RunProcessSpec, RunSetupSpec } from "../RunControl/runs";

/** A setup Command or a process as it is edited. A process's port is not here: Fleet leases it, so there is nothing to type. */
export type RunEditRow = { id: string; name: string; folder: string; command: string; pin?: number };

/** Which of the panel's two lists a row is in. */
export type RunEditList = "setup" | "processes";

/** What the running panel edits in place, and the run's name, which the pane's head draws. */
export type RunEdit = {
  draft: RunDraft;
  name: string;
  setName: (name: string) => void;
  setup: readonly RunEditRow[];
  processes: readonly RunEditRow[];
  change: (list: RunEditList, id: string, field: "name" | "folder" | "command", value: string) => void;
  add: (list: RunEditList) => void;
  remove: (list: RunEditList, id: string) => void;
  /** Pins a process's port, or releases the pin where `port` is absent. */
  pin: (id: string, port: number | undefined) => void;
  /** Every row has a name and a command, there is a process, and the run has a name. */
  ready: boolean;
  /** What saving hands back, trimmed, an empty folder being the root. */
  saved: () => { name: string; processes: RunProcessSpec[]; setup: RunSetupSpec[] };
  /** What was typed is thrown away: the next edit starts from the run again. */
  forget: () => void;
};

const rowOf = (one: RunEditRow): RunEditRow => ({ id: one.id, name: one.name, folder: one.folder, command: one.command, ...(one.pin === undefined ? {} : { pin: one.pin }) });
const specOf = (one: RunEditRow): RunProcessSpec => ({ id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim(), ...(one.pin === undefined ? {} : { pin: one.pin }) });
const setupOf = (one: RunEditRow): RunSetupSpec => ({ id: one.id, name: one.name.trim(), folder: one.folder.trim() === "" ? "." : one.folder.trim(), command: one.command.trim() });
const filled = (one: RunEditRow): boolean => one.name.trim() !== "" && one.command.trim() !== "";

/** Where a folder is on disk: the root with the folder under it, `.` and an empty field being the root itself. An absolute or `~` folder stands as typed. */
export function fullPath(root: string, folder: string): string {
  const typed = folder.trim();
  if (typed.startsWith("/") || typed.startsWith("~")) return typed;
  const parts = typed.split("/").filter((part) => part !== "" && part !== ".");
  return [root.replace(/\/+$/, ""), ...parts].join("/");
}

/** A run being edited from `draft` and `name`. What is typed is kept under `keep` until it is saved or forgotten, so leaving the panel and coming back finds it. */
export function useRunEdit({ draft, name, keep }: { draft: RunDraft; name: string; keep: string }): RunEdit {
  const [processes, setProcesses, forgetProcesses] = useKept<RunEditRow[]>(`run-edit:${keep}`, draft.processes.map(rowOf));
  const [runName, setName, forgetName] = useKept<string>(`run-edit-name:${keep}`, name);
  const [setup, setSetup, forgetSetup] = useKept<RunEditRow[]>(`run-edit-setup:${keep}`, (draft.setup ?? []).map(rowOf));
  const listOf = (list: RunEditList) => (list === "setup" ? setSetup : setProcesses);
  return {
    draft,
    name: runName,
    setName,
    setup,
    processes,
    change: (list, id, field, value) => listOf(list)((was) => was.map((one) => (one.id === id ? { ...one, [field]: value } : one))),
    add: (list) => listOf(list)((was) => [...was, { id: `${list === "setup" ? "setup-" : ""}new-${was.length + 1}-${Date.now()}`, name: "", folder: ".", command: "" }]),
    remove: (list, id) => listOf(list)((was) => was.filter((one) => one.id !== id)),
    pin: (id, port) => setProcesses((was) => was.map((one) => (one.id !== id ? one : port === undefined ? { id: one.id, name: one.name, folder: one.folder, command: one.command } : { ...one, pin: port }))),
    ready: runName.trim() !== "" && processes.length > 0 && processes.every(filled) && setup.every(filled),
    saved: () => ({ name: runName.trim(), processes: processes.map(specOf), setup: setup.map(setupOf) }),
    forget: () => {
      forgetProcesses();
      forgetName();
      forgetSetup();
    },
  };
}

/** The run's name as a field, for the head of the pane the panel is in while it is edited. It takes the cursor when it is drawn, the first field of the edit. */
export function RunNameField({ edit }: { edit: RunEdit }) {
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => field.current?.focus(), []);
  return (
    <span className="armada-running__name-field">
      <Input ref={field} aria-label="Run name" placeholder="Name" value={edit.name} onChange={(event) => edit.setName(event.target.value)} />
    </span>
  );
}
