// The title bar's search draws the palette's key as it is bound now, not the ⌘K it ships with.

import { TitleBar, parseSlots, rebind, setKeyBindings } from "@armada/components";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  setKeyBindings({});
});

function bar() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(<TitleBar onSearch={() => {}} fleet={{ state: "running", label: "Running" }} />);
}

test("a rebound palette key is the one the search shows", async () => {
  bar();
  const search = page.getByRole("button", { name: /Search, or describe work/ });
  await expect.element(search).toHaveTextContent(/K$/);
  act(() => rebind("command_palette", parseSlots("⌘P")));
  await expect.element(search).toHaveTextContent(/P$/);
});
