// The workflows switched off, as settings.json holds them under `workflows.off`: the ids off on the machine, and the ids off for each repository by its
// Manifest id. `{ "machine": ["prototype"], "repositories": { "<manifest id>": ["bug"] } }`. A mock's value for now; Fleet serves it in the settings list.

import type { SettingValue } from "@armada/protocol";

export const WORKFLOWS_OFF_KEY = "workflows.off";

export type WorkflowsOff = { machine: readonly string[]; repositories: Readonly<Record<string, readonly string[]>> };

const idsOf = (value: unknown): string[] => (Array.isArray(value) ? value.filter((one): one is string => typeof one === "string") : []);

/** What the setting holds, with whatever is missing or the wrong shape read as nothing off. */
export function workflowsOffOf(value: SettingValue | undefined): WorkflowsOff {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return { machine: [], repositories: {} };
  const held = value as { machine?: unknown; repositories?: unknown };
  const repositories = typeof held.repositories === "object" && held.repositories !== null ? Object.entries(held.repositories as Record<string, unknown>) : [];
  return { machine: idsOf(held.machine), repositories: Object.fromEntries(repositories.map(([manifest, ids]) => [manifest, idsOf(ids)])) };
}

/** Whether a Job in this repository may not take the workflow: off on the machine, or off for the repository. */
export function isOff(off: WorkflowsOff, workflowId: string, manifestId: string | undefined): boolean {
  return off.machine.includes(workflowId) || (manifestId !== undefined && (off.repositories[manifestId] ?? []).includes(workflowId));
}

/** The setting's value with one workflow switched on or off, on the machine where no repository is named. */
export function switched(off: WorkflowsOff, workflowId: string, on: boolean, manifestId: string | null): SettingValue {
  const without = (ids: readonly string[]) => ids.filter((one) => one !== workflowId);
  const with_ = (ids: readonly string[]) => [...without(ids), workflowId];
  if (manifestId === null) return { machine: on ? without(off.machine) : with_(off.machine), repositories: off.repositories };
  const there = off.repositories[manifestId] ?? [];
  const next = on ? without(there) : with_(there);
  const { [manifestId]: _was, ...rest } = off.repositories;
  return { machine: off.machine, repositories: next.length === 0 ? rest : { ...rest, [manifestId]: next } };
}
