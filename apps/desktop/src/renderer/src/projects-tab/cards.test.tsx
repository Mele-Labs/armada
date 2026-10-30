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

const task = (id: string, title: string, state: "open" | "done" | "asked" = "open") => ({ kind: "task", id, manifest_id: "armada", title, notes: "", state }) as const;
const SEED = (): Projects => ({
  projects: [{ id: "p1", name: "Pocket launch", rows: [task("t1", "Pair a phone"), task("t2", "Sign the build", "asked"), task("t3", "Pick the icon")] }],
  loose: [],
});

/** Fleet as far as the card list reads it: each write is noted, applied to what Fleet holds, and answered with the projects whole. */
function fleet() {
  const held = SEED();
  const rows = () => held.projects[0]!.rows;
  const answer = async () => ({ ok: true, value: structuredClone(held) }) as const;
  return {
    listProjects: vi.fn(answer),
    tickProjectTask: vi.fn(async (row: string, body: { done: boolean }) => {
      for (const one of rows()) if (one.id === row && one.kind === "task") one.state = body.done ? "done" : "open";
      return answer();
    }),
    removeProjectRow: vi.fn(async (row: string) => {
      held.projects[0]!.rows = rows().filter((one) => one.id !== row);
      return answer();
    }),
    moveProjectRow: vi.fn(async (row: string, body: { position: number }) => {
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
  store.load();
  await expect.poll(() => source.listProjects.mock.calls.length).toBe(1);
  await expect.poll(() => store.get().projects.length).toBe(1);
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

  // Done cards are hidden until he shows them, so the card leaves the list.
  await expect.element(card("Pick the icon")).not.toBeInTheDocument();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("t3", { done: true });
});

test("Backspace deletes the card, writes it through Fleet and offers it back", async () => {
  const source = fleet();
  await mount(source);
  focus("Pick the icon");

  await userEvent.keyboard("{Backspace}");

  await expect.element(card("Pick the icon")).not.toBeInTheDocument();
  await expect.poll(() => source.removeProjectRow.mock.calls.length).toBe(1);
  expect(source.removeProjectRow).toHaveBeenCalledWith("t3");
});

test("Option-down moves the card a place and the place is written through Fleet", async () => {
  const source = fleet();
  await mount(source);
  focus("Pair a phone");

  await userEvent.keyboard("{Alt>}{ArrowDown}{/Alt}");

  await expect.poll(() => source.moveProjectRow.mock.calls.length).toBe(1);
  expect(source.moveProjectRow).toHaveBeenCalledWith("t1", { position: 1 });
});

test("a task Fleet asks about draws as Done? and D answers it done, through Fleet", async () => {
  const source = fleet();
  await mount(source);
  await expect.element(page.getByRole("checkbox", { name: "Done?: Sign the build" })).toBeVisible();
  focus("Sign the build");

  await userEvent.keyboard("D");

  // Done cards are hidden until he shows them, so the card leaves the list.
  await expect.element(card("Sign the build")).not.toBeInTheDocument();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("t2", { done: true });
});

test("K keeps the asked task open, and writes that through Fleet", async () => {
  const source = fleet();
  await mount(source);
  focus("Sign the build");

  await userEvent.keyboard("K");

  await expect.element(page.getByRole("checkbox", { name: "Open: Sign the build" })).toBeVisible();
  await expect.poll(() => source.tickProjectTask.mock.calls.length).toBe(1);
  expect(source.tickProjectTask).toHaveBeenCalledWith("t2", { done: false });
});

// ---- Two repositories: a card's repository is a menu that moves the task, and a link made elsewhere says where. ----

const SITE = { ...ARMADA, root: "/Users/user/site", manifest: { ...ARMADA.manifest, id: "site", repository: "site" } };
const row = (id: string, title: string, manifest_id: string, more: Record<string, unknown> = {}) => ({ kind: "task", id, manifest_id, title, notes: "", state: "open", ...more }) as const;

/** A task with two subtasks and a pull request linked in site, one with one subtask, and one in site with none. */
const MANY = (): Projects => ({
  projects: [
    {
      id: "p1",
      name: "Pocket launch",
      rows: [
        row("m1", "Pair a phone", "armada", { pulls: [{ number: 2874, title: "Fit the screen", state: "open" }, { number: 2901, title: "Pair over the tunnel", state: "open", manifest_id: "site" }] }),
        row("m1a", "Draw the code", "armada", { parent: "m1" }),
        row("m1b", "Scan it", "armada", { parent: "m1" }),
        row("m2", "Sign the build", "armada"),
        row("m2a", "Find the key", "armada", { parent: "m2" }),
        row("m3", "Write the pairing page", "site"),
      ],
    },
  ],
  loose: [],
});

async function mountMany() {
  const held = MANY();
  const source = {
    listProjects: vi.fn(async () => ({ ok: true, value: structuredClone(held) }) as const),
    moveProjectTaskRepository: vi.fn(async (id: string, body: { manifest_id: string }) => {
      for (const one of held.projects[0]!.rows) if (one.kind === "task" && (one.id === id || one.parent === id)) one.manifest_id = body.manifest_id;
      return { ok: true, value: structuredClone(held) } as const;
    }),
    onProjectsChanged: () => () => {},
  };
  const opened = vi.fn();
  const getPullRequest = vi.fn(async (_manifestId: string, number: number) => ({ ok: true, value: { address: `https://forge.example/pull/${number}` } }) as const);
  vi.stubGlobal("armada", { getPullRequest });
  const store = createProjectsStore(source as unknown as ProjectsApi);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <ProjectsFrom store={store}>
      <ProjectsSurface picked={null} repositories={[ARMADA, SITE]} onOpenSession={() => {}} onOpenLink={opened} />
    </ProjectsFrom>,
  );
  store.load();
  await expect.poll(() => store.get().projects.length).toBe(1);
  store.open("p1");
  await expect.element(card("Pair a phone")).toBeVisible();
  return { source, opened, getPullRequest };
}

const menuOf = (title: string, repository: string) => card(title).getByRole("button", { name: `Repository ${repository}`, exact: true });

/** Keys only: these cards are drawn without the app's stylesheet, so a press has no place to land. */
const pressed = async (button: { element: () => Element }) => {
  (button.element() as HTMLElement).focus();
  await userEvent.keyboard("{Enter}");
};

test("a card's repository is a menu of the repositories with its own checked, headed by what moves with it", async () => {
  await mountMany();

  await pressed(menuOf("Pair a phone", "armada"));
  await expect.element(page.getByText("Move task and its 2 subtasks to")).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "armada" })).toHaveAttribute("aria-current", "true");
  await expect.element(page.getByRole("menuitem", { name: "site" })).not.toHaveAttribute("aria-current");
  await userEvent.keyboard("{Escape}");

  await pressed(menuOf("Sign the build", "armada"));
  await expect.element(page.getByText("Move task and its 1 subtask to")).toBeVisible();
  await userEvent.keyboard("{Escape}");

  await pressed(menuOf("Write the pairing page", "site"));
  await expect.element(page.getByText("Move task to", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "site" })).toHaveAttribute("aria-current", "true");
});

test("choosing another repository asks the store to move the task, and the card names it", async () => {
  const { source } = await mountMany();

  await pressed(menuOf("Pair a phone", "armada"));
  await expect.element(page.getByRole("menuitem", { name: "site" })).toBeVisible();
  await userEvent.keyboard("{ArrowDown}{Enter}");

  await expect.poll(() => source.moveProjectTaskRepository.mock.calls.length).toBe(1);
  expect(source.moveProjectTaskRepository).toHaveBeenCalledWith("m1", { manifest_id: "site" });
  await expect.element(menuOf("Pair a phone", "site")).toBeVisible();
});

test("a subtask card has no repository menu: it moves with its task", async () => {
  await mountMany();

  await expect.element(card("Draw the code")).toBeVisible();
  expect(card("Draw the code").getByRole("button", { name: /^Repository/ }).query()).toBeNull();
});

test("a link made in another repository draws muted with that repository's name, and opens in it", async () => {
  const { getPullRequest, opened } = await mountMany();
  const tunnel = page.getByRole("button", { name: "Pull request #2901 Pair over the tunnel" });

  await expect.element(card("Pair a phone").getByRole("img", { name: "Linked in site" })).toBeVisible();
  // The mute is a style on the link's wrapper, which holds the name beside its chip.
  expect(tunnel.element().closest("[data-elsewhere]")?.querySelector("[aria-label='Linked in site']")).not.toBeNull();

  await pressed(tunnel);

  await expect.poll(() => opened.mock.calls.length).toBe(1);
  expect(getPullRequest).toHaveBeenCalledWith("site", 2901);
  expect(opened).toHaveBeenCalledWith("https://forge.example/pull/2901");
});

test("a link that names no repository reads as its task's: not muted, no name, and opens in the task's", async () => {
  const { getPullRequest } = await mountMany();
  const fit = page.getByRole("button", { name: "Pull request #2874 Fit the screen" });

  await expect.element(fit).toBeVisible();
  expect(fit.element().closest("[data-elsewhere]")).toBeNull();
  expect(card("Pair a phone").getByRole("img", { name: "Linked in armada" }).query()).toBeNull();

  await pressed(fit);

  await expect.poll(() => getPullRequest.mock.calls.length).toBe(1);
  expect(getPullRequest).toHaveBeenCalledWith("armada", 2874);
});
