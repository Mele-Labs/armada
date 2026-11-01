// The Layout menu's pin: the layout new Jobs open on carries a tag, the one the Job is on offers to take the pin, and a press moves it and leaves the other row bare.

import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { LayoutControl } from "./LayoutControl";
import { forgetPanes, saveLayoutAs, setLayoutForAll, takeLayout, useJobPanes } from "./store";

const JOB = "layout-job";
let mounted: { root: Root; host: HTMLElement } | undefined;

function Control() {
  const { layout, edited, layouts, forAll } = useJobPanes(JOB);
  const [open, setOpen] = useState(true);
  const [naming, setNaming] = useState(false);
  return <LayoutControl jobId={JOB} layout={layout} edited={edited} layouts={layouts} forAll={forAll} open={open} onOpen={setOpen} naming={naming} onNaming={setNaming} />;
}

beforeEach(() => {
  forgetPanes();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Control />));
  mounted = { root, host };
  // A second layout to take, saved from the Job's own.
  act(() => saveLayoutAs(JOB, "Review"));
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
  forgetPanes();
});

const row = (name: string | RegExp) => page.getByRole("menuitem", { name });

test("new Jobs open on Default until another is pinned, and Default's row says so", async () => {
  await expect.element(row(/^Default/)).toHaveTextContent("new Jobs");
  await expect.element(row(/^Review/)).not.toHaveTextContent("new Jobs");
});

test("the layout the Job is on offers the pin, and a press moves the tag to its row", async () => {
  act(() => takeLayout(JOB, "Review"));
  await expect.element(row("Open new Jobs on Review")).toBeEnabled();
  await row("Open new Jobs on Review").click();
  // The menu shuts with the pick; the pin is the store's, and the menu opens again to show it.
  await page.getByRole("button", { name: /^Layout: Review/ }).click();
  await expect.element(row(/^Review/)).toHaveTextContent("new Jobs");
  await expect.element(row(/^Default/)).not.toHaveTextContent("new Jobs");
});

test("the layout that already holds the pin cannot be offered it again", async () => {
  act(() => setLayoutForAll("Review"));
  act(() => takeLayout(JOB, "Review"));
  await expect.element(row("Open new Jobs on Review")).toBeDisabled();
});
