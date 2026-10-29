// The band over a Job's message box: put away, it stays away until a piece it did not cover goes missing.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import type { Session, ToolsPiece } from "@armada/screens/src/draft/sessions";

import { useToolsBand } from "./tools-fixes";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const job = (id: string, missing?: ToolsPiece[]): Session => ({
  id,
  attachments: [],
  rows: [],
  turn: { state: "idle" },
  ...(missing === undefined ? {} : { toolsMissing: { harness_version: "2.1.310", missing } }),
});

function mount(id: string) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  let dismiss = () => {};
  function Probe({ session }: { session: Session }) {
    const band = useToolsBand(session);
    dismiss = band.dismiss;
    return <output data-shown={String(band.shown)} />;
  }
  const show = (missing?: ToolsPiece[]) => act(() => root.render(<Probe session={job(id, missing)} />));
  const shown = () => host.querySelector("output")!.dataset.shown === "true";
  return { show, shown, dismiss: () => act(() => dismiss()) };
}

test("the band shows while something is missing and not when the Job came up whole", () => {
  const { show, shown } = mount("band-shows");
  show();
  expect(shown()).toBe(false);
  show(["door"]);
  expect(shown()).toBe(true);
});

test("a dismissed band stays away while what is missing is no more than what it was put away for", () => {
  const { show, shown, dismiss } = mount("band-away");
  show(["door", "task_tools"]);
  dismiss();
  expect(shown()).toBe(false);

  show(["door"]);
  expect(shown()).toBe(false);
  show(["door", "task_tools"]);
  expect(shown()).toBe(false);
});

test("a dismissed band returns when a piece it did not cover goes missing", () => {
  const { show, shown, dismiss } = mount("band-returns");
  show(["task_tools"]);
  dismiss();
  expect(shown()).toBe(false);

  show(["task_tools", "door"]);
  expect(shown()).toBe(true);
});

test("a Job that came up whole forgets the dismissal, so the next thing missing shows the band", () => {
  const { show, shown, dismiss } = mount("band-forgets");
  show(["door"]);
  dismiss();
  show();
  expect(shown()).toBe(false);

  show(["door"]);
  expect(shown()).toBe(true);
});

test("one Job's dismissal is not another's", () => {
  const one = mount("band-one");
  const other = mount("band-other");
  one.show(["door"]);
  other.show(["door"]);

  one.dismiss();

  expect(one.shown()).toBe(false);
  expect(other.shown()).toBe(true);
});
