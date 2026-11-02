// The Layout menu: the layout rows are this Job's own choice and carry no tag; the layout new Jobs open on is the bottom row's value, and its submenu changes it, by press or by → ↑↓ and ↵.

import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { LayoutControl } from "./LayoutControl";
import { forgetPanes, saveLayoutAs, takeLayout, useJobPanes } from "./store";

const JOB = "layout-job";
let mounted:
  | {
      root: Root;
      host: HTMLElement;
    }
  | undefined;

function Control() {
  const { layout, edited, layouts, forAll } = useJobPanes(JOB);
  const [open, setOpen] = useState(true);
  const [naming, setNaming] = useState(false);
  return (
    <LayoutControl
      jobId={JOB}
      layout={layout}
      edited={edited}
      layouts={layouts}
      forAll={forAll}
      open={open}
      onOpen={setOpen}
      naming={naming}
      onNaming={setNaming}
    />
  );
}

beforeEach(() => {
  forgetPanes();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Control />));
  mounted = {
    root,
    host,
  };
  // A second layout to take, saved from the Job's own.
  act(() => saveLayoutAs(JOB, "Review"));
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  forgetPanes();
});

const row = (name: string | RegExp) =>
  page.getByRole("menuitem", {
    name,
  });
const choice = (name: string) =>
  page.getByRole("menuitemradio", {
    name,
  });
const newJobs = () => row(/^New Jobs open on/);
const reopen = () =>
  page
    .getByRole("button", {
      name: /^Layout:/,
    })
    .click();

test("no layout row carries a tag, and the Job's own layout is the one checked", async () => {
  await expect.element(row(/^Default/)).not.toHaveTextContent("new Jobs");
  await expect.element(row(/^Review/)).not.toHaveTextContent("new Jobs");
  // Saving as Review put the Job on it.
  await expect.element(row(/^Review/)).toHaveAttribute("aria-current", "true");
  await expect.element(row(/^Default/)).not.toHaveAttribute("aria-current");
  act(() => takeLayout(JOB, "Default"));
  await expect.element(row(/^Default/)).toHaveAttribute("aria-current", "true");
});

test("the bottom row reads New Jobs open on and the layout they open on", async () => {
  await expect.element(newJobs()).toHaveTextContent("Default");
  // Taking another layout for this Job leaves what new Jobs open on alone.
  act(() => takeLayout(JOB, "Default"));
  await expect.element(newJobs()).toHaveTextContent("Default");
  act(() => takeLayout(JOB, "Review"));
  await expect.element(newJobs()).toHaveTextContent("Default");
});

test("its submenu lists every layout with the chosen one checked, and a press sets it", async () => {
  await newJobs().click();
  await expect.element(choice("Default")).toHaveAttribute("aria-checked", "true");
  await expect.element(choice("Review")).toHaveAttribute("aria-checked", "false");
  await choice("Review").click();
  // The menu shuts with the pick; the pin is the store's, and the menu opens again to show it.
  await reopen();
  await expect.element(newJobs()).toHaveTextContent("Review");
  await expect.element(row(/^Review/)).not.toHaveTextContent("new Jobs");
});

test("→ opens the submenu, ↓ and ↵ pick, and ← goes back to the row", async () => {
  newJobs().element().focus();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(choice("Default")).toHaveFocus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(newJobs()).toHaveFocus();
  await expect.element(choice("Default")).not.toBeInTheDocument();

  await userEvent.keyboard("{ArrowRight}");
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(choice("Review")).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await reopen();
  await expect.element(newJobs()).toHaveTextContent("Review");
});

test("Esc leaves the submenu first and the menu next", async () => {
  newJobs().element().focus();
  await userEvent.keyboard("{ArrowRight}");
  await userEvent.keyboard("{Escape}");
  await expect.element(choice("Review")).not.toBeInTheDocument();
  await expect.element(newJobs()).toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  await expect.element(newJobs()).not.toBeInTheDocument();
});
