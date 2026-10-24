// A right press on the title bar opens Teach, and the bar's groups stay where they were.

import { TitleBar } from "@armada/components";
import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";

test("opening Teach on the title bar does not move its groups", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  root.render(<TitleBar repository="armada" fleet={{ state: "running", label: "Running" }} onSearch={() => undefined} onTeach={() => undefined} />);
  await expect.element(page.getByRole("button", { name: /^Search/ })).toBeVisible();
  const bar = host.querySelector<HTMLElement>(".armada-title-bar");
  // What draws: a menu root that holds no box has none to measure.
  const groups = () =>
    Array.from(bar?.children ?? [])
      .map((one) => one.getBoundingClientRect())
      .filter((box) => box.width > 0)
      .map((box) => box.left);
  const before = groups();

  bar?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 10 }));
  await expect.element(page.getByRole("menuitem", { name: "Teach" })).toBeVisible();

  expect(groups()).toEqual(before);
  root.unmount();
  host.remove();
});
