// The tasks of his projects a Job holds, drawn on its rail under the brief as task chips, and a press that opens one on Projects.
// Browser: the drawing and the press are read, over a Projects store fed by hand.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Projects } from "@armada/protocol";
import type { Session, SessionWork } from "@armada/screens/src/draft/sessions";

import type { ProjectsApi } from "../../../shared/api/projects";
import { ProjectsFrom, createProjectsStore } from "../projects-tab/store";
import { BriefZone } from "./BriefZone";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const JOB = { session_id: "s1", title: "Port the clock" };
const task = (id: string, title: string, more: object = {}) => ({ kind: "task", id, manifest_id: "armada", title, notes: "", state: "open", ...more }) as const;

const session = { id: "s1", rows: [], attachments: [], turn: { state: "idle" } } as unknown as Session;

/** The brief of Job `s1` over a Projects store that holds `projects`, or over none where `projects` is undefined. */
async function mount(projects?: Projects) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const source = { listProjects: vi.fn(async () => ({ ok: true, value: structuredClone(projects) }) as const), onProjectsChanged: () => () => {} };
  const store = projects === undefined ? undefined : createProjectsStore(source as unknown as ProjectsApi);
  root.render(
    <ProjectsFrom store={store}>
      <BriefZone session={session} work={{} as SessionWork} text="Port the clock widget" onOpenSession={() => undefined} />
    </ProjectsFrom>,
  );
  await expect.element(page.getByRole("region", { name: "Brief" })).toBeVisible();
  if (store !== undefined) {
    store.load();
    await expect.poll(() => source.listProjects.mock.calls.length).toBe(1);
    await expect.poll(() => store.get().projects.length + store.get().loose.length).toBeGreaterThan(0);
  }
}

test("a Job's rail lists the one task it holds as a chip with its project, and a press asks to show that task", async () => {
  const shown = vi.fn();
  window.addEventListener("armada:show-task", (event) => shown((event as CustomEvent<string>).detail), { once: true });
  await mount({
    projects: [{ id: "p1", name: "Pocket launch", rows: [task("t1", "Pair a phone", { state: "held", held_by: JOB }), task("t2", "Sign the build", { state: "held", held_by: { session_id: "other" } }), task("t3", "Pick the icon")] }],
    loose: [],
  });

  const list = page.getByRole("list", { name: "Task", exact: true });
  await expect.element(list).toBeVisible();
  const chip = list.getByRole("button", { name: "Task Pair a phone" });
  await expect.element(chip).toBeVisible();
  await expect.element(chip.getByText("Pocket launch")).toBeVisible();
  await expect.element(chip.getByRole("img", { name: "In a Job" })).toBeVisible();
  // Another Job's task and an open one are not this Job's.
  await expect.element(page.getByRole("button", { name: "Task Sign the build" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "Task Pick the icon" })).not.toBeInTheDocument();

  await userEvent.click(chip);

  expect(shown).toHaveBeenCalledExactlyOnceWith("t1");
});

test("a Job that holds several tasks lists them as Tasks, a loose one under No project and one it is done with asking", async () => {
  await mount({
    projects: [{ id: "p1", name: "Pocket launch", rows: [task("t1", "Pair a phone", { state: "asked", held_by: JOB })] }],
    loose: [task("t4", "Water the plants", { state: "held", held_by: JOB })],
  });

  const list = page.getByRole("list", { name: "Tasks", exact: true });
  await expect.element(list).toBeVisible();
  await expect.element(page.getByRole("list", { name: "Task", exact: true })).not.toBeInTheDocument();
  const loose = list.getByRole("button", { name: "Task Water the plants" });
  await expect.element(loose.getByText("No project")).toBeVisible();
  const asking = list.getByRole("button", { name: "Task Pair a phone" });
  await expect.element(asking.getByRole("img", { name: "Done?" })).toBeVisible();
});

test("a Job that holds no task draws no list", async () => {
  await mount({ projects: [{ id: "p1", name: "Pocket launch", rows: [task("t1", "Pair a phone", { state: "held", held_by: { session_id: "other" } })] }], loose: [] });
  await expect.element(page.getByRole("list", { name: /^Tasks?$/ })).not.toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: /^Task / })).not.toBeInTheDocument();
});

test("a window with no Projects store draws no list of tasks", async () => {
  await mount(undefined);
  await expect.element(page.getByRole("list", { name: /^Tasks?$/ })).not.toBeInTheDocument();
});
