// A Session counts as on screen only while he is at the machine: an agent's open of a panel waits when he is away.

import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, test } from "vitest";

import { AWAY_MS, useViewing, viewingNow } from "./viewed";

function Probe() {
  useViewing("s1");
  return null;
}

test("a Session nobody has open is not on screen", () => {
  expect(viewingNow("s1")).toBe(false);
});

test("an open Session stops counting after a minute with no key or pointer", () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const mounted = createRoot(document.createElement("div"));
  act(() => mounted.render(<Probe />));
  expect(viewingNow("s1")).toBe(true);
  expect(viewingNow("s1", Date.now() + AWAY_MS + 1)).toBe(false);
  act(() => void window.dispatchEvent(new Event("pointerdown")));
  expect(viewingNow("s1", Date.now() + AWAY_MS - 1000)).toBe(true);
  act(() => mounted.unmount());
});
