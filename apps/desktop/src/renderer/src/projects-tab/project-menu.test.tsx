// A project's row menu in the left column: Rename edits the name in place, Delete asks on a card beside the row first. Over a Fleet that
// answers each write with the projects as they stand. Browser: the right press, focus and the drawing are read.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Projects } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { ProjectsApi } from "../../../shared/api/projects";
import { ProjectsFrom, createProjectsStore } from "./store";
import { ProjectsSurface } from "./surface";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const ARMADA = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

function fleet() {
  const held: Projects = { projects: [{ id: "p1", name: "Pocket launch", rows: [] }], loose: [] };
  const answer = async () => ({ ok: true, value: structuredClone(held) }) as const;
  return {
    listProjects: vi.fn(answer),
    renameProject: vi.fn(async (id: string, body: { name: string }) => {
      for (const one of held.projects) if (one.id === id) one.name = body.name;
      return answer();
    }),
    deleteProject: vi.fn(async (id: string) => {
      held.projects = held.projects.filter((one) => one.id !== id);
      return answer();
    }),
    onProjectsChanged: () => () => {},
  };
}

async function mount(source: ReturnType<typeof fleet>) {
  const store = createProjectsStore(source as unknown as ProjectsApi);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <ProjectsFrom store={store}>
      <ProjectsSurface picked={ARMADA} repositories={[ARMADA]} onOpenSession={() => {}} onOpenLink={() => {}} />
    </ProjectsFrom>,
  );
  store.load();
  await expect.poll(() => store.get().projects.length).toBe(1);
  await expect.element(row()).toBeInTheDocument();
}

const row = () => page.getByRole("button", { name: "Pocket launch", exact: true });
// The page has no layout of its own here, so a press is sent to the row by hand and the menu is driven by its keys.
const openMenu = () => row().element().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 20, clientY: 20 }));
const choose = async (name: string) => {
  openMenu();
  await expect.element(page.getByRole("menuitem", { name })).toBeVisible();
  await userEvent.keyboard(name === "Rename" ? "{Enter}" : "{ArrowDown}{Enter}");
};

test("a right press on a project offers Rename and Delete", async () => {
  await mount(fleet());

  openMenu();

  await expect.element(page.getByRole("menuitem", { name: "Rename" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Delete" })).toBeVisible();
});

test("Rename opens the name selected, and Enter writes the new one through Fleet", async () => {
  const source = fleet();
  await mount(source);

  await choose("Rename");
  const field = page.getByRole("textbox", { name: "Project name" });
  await expect.element(field).toHaveFocus();
  const input = field.element() as HTMLInputElement;
  expect([input.selectionStart, input.selectionEnd]).toEqual([0, input.value.length]);
  await userEvent.keyboard("Pocket v2{Enter}");

  await expect.poll(() => source.renameProject.mock.calls.length).toBe(1);
  expect(source.renameProject).toHaveBeenCalledWith("p1", { name: "Pocket v2" });
});

test("Delete asks first, Cancel keeps the project and nothing is written", async () => {
  const source = fleet();
  await mount(source);

  await choose("Delete");
  await expect.element(page.getByRole("alertdialog", { name: "Delete Pocket launch?" })).toBeVisible();
  await userEvent.keyboard("{Enter}");

  await expect.element(page.getByRole("alertdialog")).not.toBeInTheDocument();
  await expect.element(row()).toBeVisible();
  expect(source.deleteProject).not.toHaveBeenCalled();
});

test("confirming Delete removes the project and writes it through Fleet", async () => {
  const source = fleet();
  await mount(source);

  await choose("Delete");
  await userEvent.keyboard("{Tab}{Enter}");

  await expect.element(row()).not.toBeInTheDocument();
  await expect.poll(() => source.deleteProject.mock.calls.length).toBe(1);
  expect(source.deleteProject).toHaveBeenCalledWith("p1");
});
