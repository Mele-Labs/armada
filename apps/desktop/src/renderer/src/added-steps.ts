// What a Job's surface is handed for steps added to it: the Commands a Script may name, the two
// acts on Fleet, and the save that keeps one for every Job. Nothing is built for a Job nobody has
// open, and the Commands are the picked repository's `armada.yml`, which Fleet checks again when
// the step fires.

import { useCallback, useEffect, useMemo, useState } from "react";

import type { AddedBinding } from "@armada/jobs";
import type { TriggerSummary } from "@armada/protocol";

import type { AddingStep, AddStepAnswer, EditingStep, EditStepAnswer, RemovingStep, RemoveStepAnswer } from "../../shared/added-steps";
import { readManifestFile, readTriggers, saveTrigger } from "./commands";
import { savedAs } from "./trigger-answers";

export { useAlerts } from "./alerts";

type Acts = {
  addJobStep: (adding: AddingStep) => Promise<AddStepAnswer>;
  removeJobStep: (removing: RemovingStep) => Promise<RemoveStepAnswer>;
  editJobStep: (editing: EditingStep) => Promise<EditStepAnswer>;
};

export function useAddedBinding(commands: Acts, openJob: string | null): AddedBinding | undefined {
  const [declared, setDeclared] = useState<string[]>([]);
  const [saved, setSaved] = useState<TriggerSummary[]>([]);
  const reread = useCallback(async () => {
    const answer = await readTriggers();
    if (answer.ok) setSaved(answer.triggers);
  }, []);
  useEffect(() => {
    if (openJob === null) return;
    void reread();
    void readManifestFile().then((answer) => {
      if (answer.ok) setDeclared((answer.file.declared?.commands ?? []).filter((one) => one.command.serve === undefined).map((one) => one.name));
    });
  }, [openJob, reread]);
  const { addJobStep, removeJobStep, editJobStep } = commands;
  return useMemo<AddedBinding | undefined>(
    () =>
      openJob === null
        ? undefined
        : {
            commands: declared,
            triggers: saved,
            onAdd: async (jobId, body) => {
              const answer = await addJobStep({ jobId, body });
              return answer.ok ? { ok: true, added: answer.added } : { ok: false };
            },
            onRemove: async (jobId, id) => (await removeJobStep({ jobId, body: { id } })).ok,
            onEdit: async (jobId, id, next) => {
              const answer = await editJobStep({ jobId, body: { id, ...next } });
              return answer.ok ? { ok: true, edited: answer.edited } : { ok: false };
            },
            onKeep: async (scope, definition, overwrite, keptFrom) => {
              const answer = savedAs(await saveTrigger({ manifestId: null, body: { scope, definition, ...(overwrite ? { overwrite: true } : {}), ...(keptFrom === undefined ? {} : { kept_from: keptFrom }) } }));
              // The next one kept takes its free name against this one too.
              if (answer.ok) void reread();
              return answer;
            },
          },
    [openJob, declared, saved, reread, addJobStep, removeJobStep, editJobStep],
  );
}
