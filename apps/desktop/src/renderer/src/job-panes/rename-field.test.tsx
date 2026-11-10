// A tab's rename field: what is typed stays while the Job's tab re-renders around it, as it does each time Fleet sends the Job again.

import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { SessionFrame } from "@armada/components";

import { RenameField } from "./PaneView";

let mounted:
  | {
      root: Root;
      host: HTMLElement;
    }
  | undefined;
let update: () => void = () => undefined;
const done = vi.fn();
const cancel = vi.fn();

/** The tab as Fleet's updates draw it: the same title each time, with callbacks that are new on every render. */
function Tab() {
  const [, setCount] = useState(0);
  update = () => setCount((count) => count + 1);
  return (
    <RenameField label="Fix the build" onDone={(value) => done(value)} onCancel={() => cancel()} />
  );
}

const field = () => document.querySelector<HTMLInputElement>("input")!;

beforeEach(() => {
  done.mockClear();
  cancel.mockClear();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Tab />));
  mounted = {
    root,
    host,
  };
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

test("what is typed stays while the tab re-renders with the same title", async () => {
  expect(field().value).toBe("Fix the build");
  await userEvent.keyboard("Rename");
  act(() => update());
  await userEvent.keyboard(" me");
  act(() => update());
  expect(field().value).toBe("Rename me");
  expect(document.activeElement).toBe(field());
});

test("Enter keeps what was typed and Escape leaves the name as it was", async () => {
  await userEvent.keyboard("Rename");
  act(() => update());
  await userEvent.keyboard("{Enter}");
  expect(done).toHaveBeenCalledWith("Rename");
  expect(cancel).not.toHaveBeenCalled();
});

test("Escape cancels without saving", async () => {
  await userEvent.keyboard("Rename");
  act(() => update());
  await userEvent.keyboard("{Escape}");
  expect(cancel).toHaveBeenCalledTimes(1);
  expect(done).not.toHaveBeenCalled();
});

test("the Job's head, outside panes, keeps what is typed while Fleet sends the same title again", async () => {
  act(() => mounted?.root.render(null));
  const renamed = vi.fn();
  let again: () => void = () => undefined;
  function Head() {
    const [, setCount] = useState(0);
    again = () => setCount((count) => count + 1);
    return (
      <SessionFrame
        state="working"
        said="Working"
        id="job-1"
        title="Fix the build"
        onRename={(title) => renamed(title)}
      >
        {null}
      </SessionFrame>
    );
  }
  act(() => mounted!.root.render(<Head />));
  await page
    .getByRole("button", {
      name: "Fix the build, rename",
    })
    .click();
  document.querySelector<HTMLInputElement>('input[aria-label="Job name"]')!.select();
  await userEvent.keyboard("Rename");
  act(() => again());
  await userEvent.keyboard(" me");
  act(() => again());
  expect(document.querySelector<HTMLInputElement>('input[aria-label="Job name"]')!.value).toBe(
    "Rename me",
  );
  await userEvent.keyboard("{Enter}");
  expect(renamed).toHaveBeenCalledWith("Rename me");
});
