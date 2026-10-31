// The title row's picker rows, on their own: the acts below the repositories, drawn only where the window hands one.

import { expect, test } from "vitest";
import { ADD, NEW, repositoryEntries } from "@armada/shell";
import { repository } from "@armada/screens/src/fixtures/repository";

const ids = (entries: ReturnType<typeof repositoryEntries>) => entries.map((one) => one.id);

test("New project sits above Add a repository, under one rule", () => {
  const entries = repositoryEntries([repository()], { all: "All repositories", add: true, create: true });
  expect(ids(entries).slice(-3)).toEqual(["add-repository-rule", NEW, ADD]);
  expect(entries.find((one) => one.id === NEW)).toMatchObject({ kind: "item", label: "New project" });
});

test("with no act for it, no New project row is drawn", () => {
  expect(ids(repositoryEntries([repository()], { add: true }))).not.toContain(NEW);
});

test("a Workspace picked shows its heading selected and All not", () => {
  const entries = repositoryEntries([repository()], { all: "All repositories", scope: null, workspaces: [{ id: "my-store", name: "my-store", repositories: [repository().root] }], workspaceScope: "my-store" });
  expect(entries.find((one) => one.id === "workspace:my-store")).toMatchObject({ selected: true });
  expect(entries.find((one) => one.id === "")).toMatchObject({ selected: false });
});
