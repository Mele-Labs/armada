// A Trigger as the editor holds it and the file Fleet keeps. What `get_trigger` answers is read into
// a draft and what a save sends is written from one, so a Trigger opened and saved untouched is the
// same file, and what the editor changes is all that moves.

import { describe, expect, it } from "vitest";
import type { TriggerDefinition, TriggerSummary } from "@armada/protocol";

import { blankDraft, definitionOf, draftOf, firingAt, freeNameOf, whenSaid } from "@armada/components/src/compositions/WorkflowTriggers/triggers";

const read = (definition: object, level: TriggerDefinition["level"] = "repository"): TriggerDefinition => ({
  name: String((definition as { name: string }).name),
  when: (definition as { when: TriggerDefinition["when"] }).when,
  level,
  file: "t.yml",
  definition: JSON.stringify(definition),
});

describe("a Trigger's file as a draft and back", () => {
  it("comes back as it was read", () => {
    const file = { name: "gate", when: "step_passes", workflow: "feature", step: "tests", command: "gate", on_failure: { block: true } };
    expect(JSON.parse(definitionOf(draftOf(read(file))))).toEqual(file);
  });

  it("takes the machine's level for its scope, and a skill for what it runs", () => {
    const draft = draftOf(read({ name: "notes", when: "pr_opened", skill: "qa-notes" }, "machine"));
    expect(draft).toMatchObject({ scope: "machine", runs: "skill", with: "qa-notes", block: false, repair: false });
  });

  it("is named for what it runs until it is saved, writes no step on a pull request, and no failure keys it did not switch on", () => {
    const written = JSON.parse(definitionOf({ name: "", when: "pr_opened", step: "ignored", workflow: "", runs: "command", with: "deploy", block: false, repair: false, scope: "repository" }));
    expect(written).toEqual({ name: "deploy", when: "pr_opened", command: "deploy" });
  });

  it("reads YAML as well, since a person may have written the file", () => {
    const draft = draftOf({ name: "fmt", when: "step_passes", level: "repository", file: "fmt.yml", definition: "name: fmt\nwhen: step_passes\ncommand: fmt\non_failure:\n  repair: true\n" });
    expect(draft).toMatchObject({ with: "fmt", repair: true, block: false });
  });
});

describe("which Triggers fire at a step", () => {
  const trigger = (over: Partial<TriggerSummary>): TriggerSummary => ({
    name: "t",
    when: "step_passes",
    runs: { kind: "command", name: "t" },
    block: false,
    repair: false,
    level: "repository",
    file: "t.yml",
    ...over,
  });
  const listed = [
    trigger({ name: "every_step" }),
    trigger({ name: "one_step", step: "tests" }),
    trigger({ name: "other_workflow", workflow: "bug" }),
    trigger({ name: "opens", when: "pr_opened" }),
  ];

  it("names the step it fires at, or every step where it names none, and the delivering step for a pull request", () => {
    expect(firingAt(listed, "feature", { id: "tests" }).map((one) => one.name)).toEqual(["every_step", "one_step"]);
    expect(firingAt(listed, "feature", { id: "plan" }).map((one) => one.name)).toEqual(["every_step"]);
    expect(firingAt(listed, "feature", { id: "handoff", delivers: true }).map((one) => one.name)).toEqual(["every_step", "opens"]);
    expect(firingAt(listed, "bug", { id: "fix" }).map((one) => one.name)).toEqual(["every_step", "other_workflow"]);
  });

  it("says a moment shortly", () => {
    expect(whenSaid("pr_opened")).toBe("PR opened");
    expect(whenSaid("step_starts", "tests")).toBe("tests starts");
    expect(whenSaid("step_passes")).toBe("a step passes");
  });
});

const heldDrone = (name: string, level: TriggerSummary["level"]): TriggerSummary => ({
  name,
  when: "pr_opened",
  runs: { kind: "drone", brief: "x" },
  block: false,
  repair: false,
  level,
  file: `${name}.yml`,
});

it("two prompts sharing their first four words take the next free name, and a named draft keeps its own", () => {
  const draft = blankDraft({ runs: "drone", with: "Add a changelog line for this Job", scope: "repository" });
  expect(freeNameOf(draft, [])).toBe("add-a-changelog-line");
  const one = heldDrone("add-a-changelog-line", "repository");
  expect(freeNameOf(draft, [one])).toBe("add-a-changelog-line-2");
  expect(freeNameOf(draft, [one, heldDrone("add-a-changelog-line-2", "repository")])).toBe("add-a-changelog-line-3");
  // Another level's copy is a different Trigger.
  expect(freeNameOf(draft, [heldDrone("add-a-changelog-line", "machine")])).toBe("add-a-changelog-line");
  // Editing the same Trigger is a Replace and keeps its name.
  expect(freeNameOf({ ...draft, name: "add-a-changelog-line" }, [one])).toBe("add-a-changelog-line");
});
