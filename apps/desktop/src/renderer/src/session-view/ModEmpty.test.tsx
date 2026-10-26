// An empty Mod Job lists what a Mod Job can make as six aims to pick from. The pick is kept by the Job, so it survives leaving and coming
// back; pressing the picked aim again clears it; and the arrows move between them.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { forgetAllKept } from "@armada/components";

import "../styles/index.css";
import { ModEmpty } from "./ModEmpty";
import { aimOf } from "./mod-aim";

const roots: { root: Root; host: HTMLElement }[] = [];

beforeEach(forgetAllKept);
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  forgetAllKept();
});

function mount(id: string): () => void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<ModEmpty sessionId={id} />);
  return () => root.unmount();
}

const AIMS = [
  ["Colours", "theme"],
  ["Panels", "layout"],
  ["Words", "words"],
  ["Notices", "fleet"],
  ["Issue trackers", "tracker"],
  ["Anything else", "source"],
] as const;

test("draws the six aims as one radio group, none picked", async () => {
  mount("job-1");
  await expect.element(page.getByRole("radiogroup", { name: "Aim" })).toBeVisible();
  for (const [name] of AIMS) await expect.element(page.getByRole("radio", { name })).toHaveAttribute("aria-checked", "false");
  expect(page.getByRole("radio").elements()).toHaveLength(AIMS.length);
  expect(aimOf("job-1")).toBeUndefined();
});

test("each press picks its aim and unpicks the one before", async () => {
  mount("job-1");
  for (const [name, aim] of AIMS) {
    await userEvent.click(page.getByRole("radio", { name }));
    await expect.element(page.getByRole("radio", { name })).toHaveAttribute("aria-checked", "true");
    expect(aimOf("job-1")).toBe(aim);
    expect(page.getByRole("radio", { checked: true }).elements()).toHaveLength(1);
  }
});

test("pressing the picked aim again clears it", async () => {
  mount("job-1");
  await userEvent.click(page.getByRole("radio", { name: "Words" }));
  await userEvent.click(page.getByRole("radio", { name: "Words" }));
  await expect.element(page.getByRole("radio", { name: "Words" })).toHaveAttribute("aria-checked", "false");
  expect(aimOf("job-1")).toBeUndefined();
});

test("the pick is kept for the Job: leaving and coming back finds it, and another Job does not share it", async () => {
  const leave = mount("job-1");
  await userEvent.click(page.getByRole("radio", { name: "Panels" }));
  leave();
  mount("job-1");
  await expect.element(page.getByRole("radio", { name: "Panels" })).toHaveAttribute("aria-checked", "true");
  expect(aimOf("job-1")).toBe("layout");
  expect(aimOf("job-2")).toBeUndefined();
});

test("the arrows move focus between the aims, two to a row, and Tab reaches one stop", async () => {
  mount("job-1");
  await expect.element(page.getByRole("radio", { name: "Colours" })).toBeVisible();
  page.getByRole("radio", { name: "Colours" }).element().focus();
  await userEvent.keyboard("{ArrowRight}");
  expect(document.activeElement).toBe(page.getByRole("radio", { name: "Panels" }).element());
  await userEvent.keyboard("{ArrowDown}");
  expect(document.activeElement).toBe(page.getByRole("radio", { name: "Notices" }).element());
  await userEvent.keyboard("{ArrowLeft}{ArrowUp}");
  expect(document.activeElement).toBe(page.getByRole("radio", { name: "Colours" }).element());
  expect(page.getByRole("radio").elements().filter((one) => one.getAttribute("tabindex") === "0")).toHaveLength(1);
});
