// The Mods list's tracker rows, through `ModList` over a taught source: the frame each row takes from its marks, and that Try reaches
// the source with the mod's name, is off while the mod is invalid, and is not offered where the source cannot try.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, unmount } from "@armada/screens/src/mounted";

import { ModList } from "./ModList";
import { TaughtSourceProvider } from "./taught-source";
import type { TaughtMod, TaughtSource } from "./taught-source";

afterEach(unmount);

const tracker = (name: string, tried: NonNullable<TaughtMod["tracker"]>["tried"], over: Partial<TaughtMod> & { valid?: boolean } = {}): TaughtMod => {
  const { valid = true, ...rest } = over;
  return { name, kind: "tracker", description: "", on: "", enabled: true, parts: [], tracker: { tried, valid }, ...rest };
};

const MODS: readonly TaughtMod[] = [
  tracker("used", { state: "passed", words: "Tried 14 Oct on armada" }),
  tracker("edited", { state: "stale", words: "Stale" }),
  tracker("never", { state: "never", words: "Not tried" }),
  tracker("failed", { state: "failed", words: "Failed 14 Oct: no reason given" }),
  tracker("broken", { state: "never", words: "Not tried" }, { valid: false, description: "[search] has no argv" }),
  tracker("quiet", { state: "passed", words: "Tried 14 Oct on armada" }, { enabled: false }),
];

function source(over: Partial<TaughtSource> = {}): TaughtSource {
  const state = { mods: MODS, details: {} };
  return { get: () => state, subscribe: () => () => undefined, setEnabled: () => undefined, promote: () => undefined, open: () => undefined, ...over };
}

const standing = (name: string) => page.getByRole("group", { name, exact: true });

test("a tracker row is framed by what a Manifest would do with it", async () => {
  mount(
    <TaughtSourceProvider value={source()}>
      <ModList kind="tracker" />
    </TaughtSourceProvider>,
  );
  const frames = { used: "used", edited: "waiting", never: "waiting", failed: "refused", broken: "refused", quiet: "off" };
  for (const [name, frame] of Object.entries(frames)) await expect.element(standing(name)).toHaveAttribute("data-standing", frame);
});

test("Try reaches the source with the mod's name, and is off for a mod that is not valid", async () => {
  const tryIt = vi.fn();
  mount(
    <TaughtSourceProvider value={source({ tryIt })}>
      <ModList kind="tracker" />
    </TaughtSourceProvider>,
  );
  await userEvent.click(page.getByRole("button", { name: "Try edited" }));
  expect(tryIt).toHaveBeenCalledWith("edited");
  await expect.element(page.getByRole("button", { name: "Try broken" })).toBeDisabled();
});

test("a source that cannot try draws no Try button", async () => {
  mount(
    <TaughtSourceProvider value={source()}>
      <ModList kind="tracker" />
    </TaughtSourceProvider>,
  );
  await expect.element(page.getByRole("group", { name: "edited", exact: true })).toBeVisible();
  expect(page.getByRole("button", { name: "Try edited" }).elements()).toHaveLength(0);
});

test("Create mod sits in the header and calls onCreate, and is not drawn where no one offers it", async () => {
  const create = vi.fn();
  mount(
    <TaughtSourceProvider value={source()}>
      <ModList kind="tracker" title="Trackers" onCreate={create} />
    </TaughtSourceProvider>,
  );
  await userEvent.click(page.getByRole("button", { name: "Create mod" }));
  expect(create).toHaveBeenCalledTimes(1);
  unmount();
  mount(
    <TaughtSourceProvider value={source()}>
      <ModList kind="tracker" title="Trackers" />
    </TaughtSourceProvider>,
  );
  await expect.element(page.getByRole("group", { name: "used", exact: true })).toBeVisible();
  expect(page.getByRole("button", { name: "Create mod" }).elements()).toHaveLength(0);
});

test("a press on a row's name opens the mod with its name, title and kind, and leaves its switch alone", async () => {
  const open = vi.fn();
  const setEnabled = vi.fn();
  mount(
    <TaughtSourceProvider value={source({ setEnabled })}>
      <ModList kind="tracker" onOpenMod={open} />
    </TaughtSourceProvider>,
  );
  await userEvent.click(page.getByRole("button", { name: "Open edited" }));
  expect(open).toHaveBeenCalledWith({ name: "edited", title: "edited", kind: "tracker" });
  expect(setEnabled).not.toHaveBeenCalled();
});

test("a row opens nothing where no one takes the press", async () => {
  mount(
    <TaughtSourceProvider value={source()}>
      <ModList kind="tracker" />
    </TaughtSourceProvider>,
  );
  await expect.element(page.getByRole("group", { name: "edited", exact: true })).toBeVisible();
  expect(page.getByRole("button", { name: "Open edited" }).elements()).toHaveLength(0);
});
