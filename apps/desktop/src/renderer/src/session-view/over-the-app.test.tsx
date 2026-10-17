// A sheet opened from inside a Session's pane is drawn on the body, not inside the glass pane that would cut it to its box, and Esc gives
// focus back to what opened it. Browser: where an element is in the document is what is read.

import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { Sheet } from "@armada/components";

import { OverTheApp } from "./OverTheApp";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function Pane() {
  const [open, setOpen] = useState(false);
  return (
    <section className="deck-pane" style={{ backdropFilter: "blur(1px)" }}>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      <OverTheApp open={open}>
        <Sheet kind="over-the-app-test" open={open} floating title="Reading" closeLabel="Close" closeBinding="Esc" onClose={() => setOpen(false)}>
          Body
        </Sheet>
      </OverTheApp>
    </section>
  );
}

function mount(): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Pane />));
  roots.push({ root, host });
  return host;
}

test("a sheet opened from inside a deck pane is not inside the pane in the DOM", () => {
  const host = mount();
  const open = host.querySelector("button")!;
  open.focus();
  act(() => open.click());
  const sheet = document.querySelector(".armada-sheet");
  expect(sheet).not.toBeNull();
  expect(sheet!.closest(".deck-pane")).toBeNull();
  expect(sheet!.closest(".armada-sheet-scrim")!.parentElement).toBe(document.body);
});

test("Esc closes the sheet and focus goes back to the button that opened it", async () => {
  const host = mount();
  const open = host.querySelector("button")!;
  open.focus();
  act(() => open.click());
  await act(async () => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
    await new Promise((done) => window.setTimeout(done, 10));
  });
  expect(document.querySelector(".armada-sheet")).toBeNull();
  expect(document.activeElement).toBe(open);
});
