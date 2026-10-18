// ⌘K, "debug", ↵ is Debug Bridge: the owner's three keys from any screen. Copy debug info also
// matches "debug" and draws first, so it is left out while no failure is on screen.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { Palette } from "@armada/shell";
import { mount, unmount } from "@armada/screens/src/mounted";

import { absentIn, carryOut, dormantIn, type PaletteHands } from "./palette";

afterEach(unmount);

const hands = (debugBridge: () => void, copyDebugInfo: () => void = () => {}): PaletteHands => ({
  openSession: () => {},
  compose: () => {},
  surface: () => {},
  run: () => {},
  copyDebugInfo,
  openSetting: () => {},
  debugBridge,
  refresh: () => {},
  pickRepository: () => {},
  studioRun: () => {},
  studioAdd: () => {},
});

function palette(failing: boolean, act: PaletteHands, debuggable = true) {
  const where = { failing };
  return (
    <Palette
      open
      onClose={() => {}}
      context="board"
      on={null}
      surfaces={[]}
      settings={[]}
      dormant={dormantIn({ ...where, live: true, debuggable, studioRun: null, studioAdd: null })}
      absent={absentIn(where)}
      onChoose={(choice) => carryOut(choice, act)}
    />
  );
}

test('"debug" ↵ with nothing failing is Debug Bridge, and Copy debug info is not offered', async () => {
  const debug = vi.fn();
  mount(palette(false, hands(debug)));
  await userEvent.keyboard("debug");
  await expect.element(page.getByRole("option", { name: /Debug Bridge/ })).toBeVisible();
  await expect.element(page.getByRole("option", { name: /Copy debug info/ })).not.toBeInTheDocument();
  await userEvent.keyboard("{Enter}");
  expect(debug).toHaveBeenCalledTimes(1);
});

test("with a failure on screen Copy debug info is back, first, and still copies", async () => {
  const debug = vi.fn();
  const copy = vi.fn();
  mount(palette(true, hands(debug, copy)));
  await userEvent.keyboard("debug");
  await expect.element(page.getByRole("option", { name: /Copy debug info/ })).toBeVisible();
  await userEvent.keyboard("{Enter}");
  expect(copy).toHaveBeenCalledTimes(1);
  expect(debug).not.toHaveBeenCalled();
});

test("Debug Bridge says why it cannot run outside pnpm dev, and with Fleet down", () => {
  const base = { reading: false, cursor: null, failing: false, studioRun: null, studioAdd: null };
  expect(dormantIn({ ...base, live: true }).debug_bridge).toBe("needs Bridge under pnpm dev");
  expect(dormantIn({ ...base, live: false, debuggable: true }).debug_bridge).toBe("Fleet is not connected");
  expect(dormantIn({ ...base, live: true, debuggable: true }).debug_bridge).toBeUndefined();
});

test("Send and Queue for later say what they need, since the palette has no draft to act on", () => {
  const base = { reading: false, cursor: null, failing: false, live: true, studioRun: null, studioAdd: null };
  expect(dormantIn(base).send_message).toBe("type in a message box first");
  expect(dormantIn(base).queue_session).toBe("in a Job's dispatcher");
});
