// The shared tile grid on a surface whose grid is not drawn on its first render: one that first says
// nothing is served, as the Manifest does on a fresh install. Its keys and its one Tab stop are
// picked up on the render that draws the grid, not lost because the first render had none.

import { useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { useTileGrid } from "@armada/components";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function Late() {
  const [served, setServed] = useState(false);
  const grid = useRef<HTMLUListElement>(null);
  useTileGrid(grid, { tile: "li", cursor: "button" });
  return (
    <>
      <button type="button" onClick={() => setServed(true)}>
        Serve
      </button>
      {served ? (
        <ul ref={grid}>
          {["one", "two", "three"].map((name) => (
            <li key={name}>
              <button type="button">{name}</button>
            </li>
          ))}
        </ul>
      ) : (
        <p>Nothing is served yet</p>
      )}
      <button type="button">After</button>
    </>
  );
}

test("a grid drawn after the first render is still one Tab stop and moves on the arrows", async () => {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  flushSync(() => root.render(<Late />));

  await userEvent.click(page.getByRole("button", { name: "Serve" }));
  await expect.element(page.getByRole("button", { name: "one" })).toBeVisible();

  await userEvent.keyboard("{Tab}");
  await expect.element(page.getByRole("button", { name: "one" })).toHaveFocus();
  // One stop: the next Tab leaves the grid rather than walking it.
  await userEvent.keyboard("{Tab}");
  await expect.element(page.getByRole("button", { name: "After" })).toHaveFocus();
  await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
  await expect.element(page.getByRole("button", { name: "one" })).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(page.getByRole("button", { name: "two" })).toHaveFocus();
});
