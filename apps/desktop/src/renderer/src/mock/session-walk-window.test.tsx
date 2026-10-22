// A Session's window, through `App`: a Session that shows a page opens it by itself, its row in the Artifacts layer
// opens it again once it is closed, and a note taken in it reaches the Session as a message from you.
// The walk `session-walk-window` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const openArtifacts = () => userEvent.click(page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts", exact: true }));

const window = () => page.getByRole("dialog", { name: "Bridge's window on Store clock findings" });

test("Session window: opens by itself, reopens from its Artifacts row, and a note goes to the Session", async () => {
  mount("session-walk-window");
  await onScreen();
  await toSession("Store clock findings");
  await expect.element(window()).not.toBeInTheDocument();

  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "Show me what you found");
  await userEvent.keyboard("{Enter}");
  await expect.element(window()).toBeVisible();
  const artifacts = page.getByRole("region", { name: "Artifacts", exact: true });
  const pages = artifacts.getByRole("region", { name: "Pages", exact: true });
  await openArtifacts();
  await expect.element(pages.getByRole("button", { name: "Clock read report", exact: true }).getByRole("img", { name: "Shown in a window" })).toBeVisible();
  await userEvent.click(artifacts.getByRole("button", { name: "Back" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByRole("button", { name: "Open window Clock read report" })).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Close window" }));
  await expect.element(window()).not.toBeInTheDocument();
  await openArtifacts();
  await userEvent.click(pages.getByRole("button", { name: "Clock read report", exact: true }));
  await userEvent.click(artifacts.getByRole("button", { name: "Back" }));
  await expect.element(window()).toBeVisible();
  // The bar names the page by its own title and the Session by its own name, which opens it.
  await expect.element(window().getByText("Clock read report", { exact: true })).toBeVisible();
  await expect.element(window().getByRole("button", { name: "Store clock findings" })).toBeVisible();

  // A press inside the page picks what is under it. The frame is blank under test, so the page is made here.
  await userEvent.click(page.getByRole("button", { name: "Capture", exact: true }));
  const frame = document.querySelector<HTMLIFrameElement>(".armada-mock-walk-window__frame iframe")!;
  const inner = frame.contentDocument!;
  const pressed = inner.createElement("button");
  pressed.textContent = "Reload";
  inner.body.append(pressed);
  pressed.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await userEvent.fill(page.getByRole("dialog", { name: "Capture a note" }).getByRole("textbox", { name: "Note" }), "The clock reads late");
  await userEvent.click(page.getByRole("dialog", { name: "Capture a note" }).getByRole("button", { name: "Capture", exact: true }));

  const thread = page.getByRole("region", { name: "Thread" });
  await expect.element(thread.getByText(/The clock reads late/)).toBeVisible();
  await expect.element(thread.getByText("Noted. I will change that.")).toBeVisible();
});
