// A project's card list on its keys and the done ask on a card, over a Fleet that answers each write with the projects as they
// stand. Browser: focus, the keys and the drawing are read. The whole tab on the mock Fleet is `mock/projects-wired.test.tsx`.

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

const task = (id: string, title: string, state: "open" | "done" | "asked" = "open") => ({ kind: "task", id, title, notes: "", state }) as const;
const SEED = (): Projects => ({
  manifest_id: "armada",
  projects: [{ id: "p1", name: "Pocket launch", rows: [task("t1", "Pair a phone"), task("t2", "Sign the build", "asked"), task("t3", "Pick the icon")] }],
});

/** Fleet as far as the card list reads it: each write is noted, applied to what Fleet holds, and answered with the projects whole. */
function fleet() {
  const held = SEED();
  const rows = () => held.projects[0]!.rows;
  const answer = async () => ({ ok: true, value: structuredClone(held) }) as const;
  return {
    listProjects: vi.fn(answer),
    tickProjectTask: vi.fn(async (_manifest: string, _project: string, row: string, body: { done: boolean }) => {
      for (const one of rows()) if (one.id === row && one.kind === "task") one.state = body.done ? "done" : "open";
      return answer();
    }),
    removeProjectRow: vi.fn(async (_manifest: string, _project: string, row: string) => {
      held.projects[0]!.rows = rows().filter((one) => one.id !== row);
      return answer();
    }),
    moveProjectRow: vi.fn(async (_manifest: string, _project: string, row: string, body: { position: number }) => {
      const from = rows().findIndex((one) => one.id === row);
      rows().splice(body.position, 0, rows().splice(from, 1)[0]!);
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
  store.load(["armada"]);
  await expect.poll(() => source.listProjects.mock.calls.length).toBe(1);
  await expect.poll(() => store.get().projects.filter((one) => one.own !== true).length).toBe(1);
  store.open("p1");
  await expect.element(card("Pair a phone")).toBeVisible();
  return store;
}

const card = (title: string) => page.getByRole("group", { name: title });
const focus = (title: string) => card(title).element().focus();

test("j and k move between cards, and Space selects the one under focus", async () => {
  await mount(fleet());
  focus("Pair a phone");

  await userEvent.keyboard("j");
  expect(document.activeElement).toBe(card("Sign the build").element());
  await userEvent.keyboard("k");
  expect(document.activeElement).toBe(card("Pair a phone").element());

  await userEvent.keyboard(" ");
  await expect.poll(() => card("Pair a phone").element().hasAttribute("data-selected")).toBe(true);
  expect(card("Pick the icon").element().hasAttribute("data-selected")).toBe(false);
});

test("d ticks the card under focus and the tick is written through Fleet", async () => {
  const source = fleet();
  await mount(source);
  focus("Pick the icon");

  await userEvent.keyboard("d");

  await expect.element(page.getByRole("checkbox", { name: "Done: Pick the icon" })).toBeVisible();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("armada", "p1", "t3", { done: true });
});

test("Backspace deletes the card, writes it through Fleet and offers it back", async () => {
  const source = fleet();
  await mount(source);
  focus("Pick the icon");

  await userEvent.keyboard("{Backspace}");

  await expect.element(card("Pick the icon")).not.toBeInTheDocument();
  await expect.poll(() => source.removeProjectRow.mock.calls.length).toBe(1);
  expect(source.removeProjectRow).toHaveBeenCalledWith("armada", "p1", "t3");
});

test("Option-down moves the card a place and the place is written through Fleet", async () => {
  const source = fleet();
  await mount(source);
  focus("Pair a phone");

  await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");

  await expect.poll(() => source.moveProjectRow.mock.calls.length).toBe(1);
  expect(source.moveProjectRow).toHaveBeenCalledWith("armada", "p1", "t1", { position: 1 });
});

test("a task Fleet asks about draws as Done? and D answers it done, through Fleet", async () => {
  const source = fleet();
  await mount(source);
  await expect.element(page.getByRole("checkbox", { name: "Done?: Sign the build" })).toBeVisible();
  focus("Sign the build");

  await userEvent.keyboard("D");

  await expect.element(page.getByRole("checkbox", { name: "Done: Sign the build" })).toBeVisible();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("armada", "p1", "t2", { done: true });
});

test("K keeps the asked task open, and writes that through Fleet", async () => {
  const source = fleet();
  await mount(source);
  focus("Sign the build");

  await userEvent.keyboard("K");

  await expect.element(page.getByRole("checkbox", { name: "Open: Sign the build" })).toBeVisible();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("armada", "p1", "t2", { done: false });
});
