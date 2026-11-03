// The bar of a Job's walk, which draws in the tab's pane and over the page in Bridge's own window. What it says and what each press
// and key does: the takes, Approve and Send notes, Reload and Capture, the steps, and the `?`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { WalkBar, type WalkBarProps } from "@armada/components";

import "../styles/index.css";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

async function draw(props: Partial<WalkBarProps> = {}) {
  const calls = {
    onBack: vi.fn(),
    onNext: vi.fn(),
    onReload: vi.fn(),
    onCapture: vi.fn(),
    onApprove: vi.fn(),
  };
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;left:0;top:0;width:900px";
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  await act(async () =>
    root.render(<WalkBar name="Billing" at={0} of={0} said="" {...calls} {...props} />),
  );
  return calls;
}

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  document
    .querySelectorAll(".armada-popover__panel, .armada-dropdown-menu__panel")
    .forEach((one) => one.remove());
});

const group = () =>
  page.getByRole("group", {
    name: "Walk Billing",
  });
const button = (name: string) =>
  page.getByRole("button", {
    name,
    exact: true,
  });

test("a walk of one page and no steps is the group and its buttons, with no takes, no steps and no question mark", async () => {
  await draw();
  await expect.element(group()).toBeVisible();
  await expect.element(button("Reload")).toBeVisible();
  await expect.element(button("Capture")).toBeVisible();
  await expect.element(button("Approve")).toBeVisible();
  await expect.element(page.getByRole("radiogroup")).not.toBeInTheDocument();
  await expect.element(button("Next")).not.toBeInTheDocument();
  await expect.element(button("What to look at")).not.toBeInTheDocument();
});

test("Reload, Capture and Approve call what they are given, and Capture reads pressed while a note is taken", async () => {
  const calls = await draw({
    capturing: true,
  });
  await button("Reload").click();
  await button("Capture").click();
  await button("Approve").click();
  expect(calls.onReload).toHaveBeenCalledTimes(1);
  expect(calls.onCapture).toHaveBeenCalledTimes(1);
  expect(calls.onApprove).toHaveBeenCalledTimes(1);
  await expect.element(button("Capture")).toHaveAttribute("aria-pressed", "true");
});

test("takes are a radio group that names the one on screen, picks on a press, and walks with the arrow keys", async () => {
  const onPick = vi.fn();
  await draw({
    takes: {
      names: [
        "Compact",
        "Roomy",
        "Dense",
      ],
      at: 1,
      onPick,
    },
  });
  const takes = page.getByRole("radio");
  await expect.element(takes.nth(1)).toBeChecked();
  await expect.element(takes.nth(0)).not.toBeChecked();
  // Approve names the take it approves.
  await expect.element(button("Approve Roomy")).toBeVisible();
  await takes.nth(2).click();
  expect(onPick).toHaveBeenLastCalledWith(2);
  (document.querySelectorAll<HTMLElement>("[data-take]")[1] as HTMLElement).focus();
  await userEvent.keyboard("{ArrowRight}");
  expect(onPick).toHaveBeenLastCalledWith(2);
  await userEvent.keyboard("{Home}");
  expect(onPick).toHaveBeenLastCalledWith(0);
  await userEvent.keyboard("{End}");
  expect(onPick).toHaveBeenLastCalledWith(2);
});

test("steps draw their count and line, Back is off on the first and Next on the last, and each calls its own", async () => {
  const calls = await draw({
    at: 1,
    of: 3,
    said: "Open the plan card",
  });
  await expect.element(page.getByText("1 of 3")).toBeVisible();
  await expect.element(page.getByText("Open the plan card")).toBeVisible();
  await expect.element(button("Back")).toBeDisabled();
  await button("Next").click();
  expect(calls.onNext).toHaveBeenCalledTimes(1);
  roots.splice(0).forEach(({ root, host }) => {
    root.unmount();
    host.remove();
  });
  const last = await draw({
    at: 3,
    of: 3,
    said: "Done",
  });
  await expect.element(button("Next")).toBeDisabled();
  await button("Back").click();
  expect(last.onBack).toHaveBeenCalledTimes(1);
});

test("the question mark appears with a brief, and opens it", async () => {
  await draw({
    brief: "Billing now shows the plan card. Look at the price.",
  });
  await button("What to look at").click();
  await expect
    .element(
      page.getByText("Look at the price.", {
        exact: false,
      }),
    )
    .toBeVisible();
});

test("while the Job is still changing, Approve reads Not ready and is off", async () => {
  const calls = await draw({
    readiness: "working",
  });
  await expect.element(button("Not ready")).toBeDisabled();
  await expect.element(button("Approve")).not.toBeInTheDocument();
  expect(calls.onApprove).not.toHaveBeenCalled();
});

test("⌘↵ approves when no field has the cursor, and ⌘R reloads", async () => {
  const calls = await draw();
  button("Reload").element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(calls.onApprove).toHaveBeenCalledTimes(1);
  await userEvent.keyboard("{Meta>}r{/Meta}");
  expect(calls.onReload).toHaveBeenCalledTimes(1);
});

test("⌘↵ in a field is the field's, and approves nothing", async () => {
  const calls = await draw();
  const field = document.createElement("input");
  group().element().append(field);
  field.focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(calls.onApprove).not.toHaveBeenCalled();
  field.remove();
});

test("with notes held, Approve is Send notes with approving behind its caret, and ⌘↵ sends rather than approves", async () => {
  const onSend = vi.fn();
  const onSendAndApprove = vi.fn();
  const calls = await draw({
    held: {
      notes: "several",
      onSend,
      onSendAndApprove,
    },
  });
  await expect.element(button("Approve")).not.toBeInTheDocument();
  await button("Send notes").click();
  expect(onSend).toHaveBeenCalledTimes(1);
  button("Reload").element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(onSend).toHaveBeenCalledTimes(2);
  expect(calls.onApprove).not.toHaveBeenCalled();
  await button("Approve options").click();
  await page
    .getByRole("menuitem", {
      name: "Send notes and approve",
    })
    .click();
  expect(onSendAndApprove).toHaveBeenCalledTimes(1);
  await button("Approve options").click();
  await page
    .getByRole("menuitem", {
      name: "Approve without notes",
    })
    .click();
  expect(calls.onApprove).toHaveBeenCalledTimes(1);
});
