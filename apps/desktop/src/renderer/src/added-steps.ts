// What a Job's surface is handed for steps added to it: the Commands a Script may name, the two
// acts on Fleet, and the save that keeps one for every Job. Nothing is built for a Job nobody has
// open, and the Commands are the picked repository's `armada.yml`, which Fleet checks again when
// the step fires.

import { useEffect, useMemo, useState } from "react";

import type { AddedBinding } from "@armada/jobs";

import type { AddingStep, AddStepAnswer, RemovingStep, RemoveStepAnswer } from "../../shared/added-steps";
import { readManifestFile, saveTrigger } from "./commands";
import { savedAs } from "./trigger-answers";

type Acts = {
  addJobStep: (adding: AddingStep) => Promise<AddStepAnswer>;
  removeJobStep: (removing: RemovingStep) => Promise<RemoveStepAnswer>;
};

export function useAddedBinding(commands: Acts, openJob: string | null): AddedBinding | undefined {
  const [declared, setDeclared] = useState<string[]>([]);
  useEffect(() => {
    if (openJob === null) return;
    void readManifestFile().then((answer) => {
      if (answer.ok) setDeclared((answer.file.declared?.commands ?? []).filter((one) => one.command.serve === undefined).map((one) => one.name));
    });
  }, [openJob]);
  const { addJobStep, removeJobStep } = commands;
  return useMemo<AddedBinding | undefined>(
    () =>
      openJob === null
        ? undefined
        : {
            commands: declared,
            onAdd: async (jobId, body) => {
              const answer = await addJobStep({ jobId, body });
              return answer.ok ? { ok: true, added: answer.added } : { ok: false };
            },
            onRemove: async (jobId, id) => (await removeJobStep({ jobId, body: { id } })).ok,
            onKeep: async (scope, definition, overwrite, keptFrom) =>
              savedAs(await saveTrigger({ manifestId: null, body: { scope, definition, ...(overwrite ? { overwrite: true } : {}), ...(keptFrom === undefined ? {} : { kept_from: keptFrom }) } })),
          },
    [openJob, declared, addJobStep, removeJobStep],
  );
}
