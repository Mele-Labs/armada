// armada-21wq: an update loop above an open dialog with a text box was reported
// as Dialog's, because Dialog set state after every render and so was the last
// to schedule an update when React gave up. The looper must be the one named.

import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { Dialog } from "@armada/components";
import "./styles/index.css";

let value = 0;
const hear = new Set<() => void>();
const subscribe = (heard: () => void) => {
  hear.add(heard);
  return () => void hear.delete(heard);
};

/** Bumps a store after every commit and remounts the dialog under it each time. */
function Remounter() {
  const v = useSyncExternalStore(subscribe, () => value);
  useEffect(() => {
    value += 1;
    for (const heard of hear) heard();
  });
  return (
    <Dialog key={v} open title="Redirect" confirmLabel="Send">
      <textarea />
    </Dialog>
  );
}

const hosts: HTMLElement[] = [];
afterEach(() => {
  for (const host of hosts.splice(0)) host.remove();
});

function mount(
  node: ReactNode,
  onUncaughtError?: (error: unknown, componentStack: string) => void,
) {
  const host = document.createElement("div");
  document.body.append(host);
  hosts.push(host);
  const root = createRoot(host, {
    onUncaughtError: (error, info) => onUncaughtError?.(error, info.componentStack ?? ""),
  });
  root.render(node);
  return {
    host,
    root,
  };
}

test("a loop above an open dialog with a text box is blamed on the looper, not Dialog", async () => {
  let blamed: string | null = null;
  const { root } = mount(<Remounter />, (_error, stack) => {
    blamed = /at (\S+)/.exec(stack)?.[1] ?? "";
  });
  await vi.waitFor(() => expect(blamed).not.toBeNull(), {
    timeout: 3000,
  });
  root.unmount();
  expect(blamed).toBe("Remounter");
});

function keys(host: HTMLElement) {
  const shown = (button: Element | null) =>
    [
      ...(button?.querySelectorAll(".armada-dialog__kbd") ?? []),
    ].some((kbd) => getComputedStyle(kbd).display !== "none");
  const [cancel, confirm] = host.querySelectorAll(".armada-dialog__actions button");
  return {
    cancel: shown(cancel ?? null),
    confirm: shown(confirm ?? null),
  };
}

test("↵ is drawn on Cancel in a plain confirmation and on Confirm in one with a text box", async () => {
  const plain = mount(
    <Dialog open title="End it" confirmLabel="End">
      Ends the Job.
    </Dialog>,
  );
  const typed = mount(
    <Dialog open title="Redirect" confirmLabel="Send">
      <textarea />
    </Dialog>,
  );
  await vi.waitFor(() => expect(typed.host.querySelector("textarea")).not.toBeNull());
  expect(keys(plain.host)).toEqual({
    cancel: true,
    confirm: false,
  });
  expect(keys(typed.host)).toEqual({
    cancel: false,
    confirm: true,
  });
  plain.root.unmount();
  typed.root.unmount();
});

test("Enter sends a dialog with a text box, and leaves a plain one to the focused button", async () => {
  const sent = vi.fn();
  const typed = mount(
    <Dialog open title="Redirect" confirmLabel="Send" onConfirm={sent}>
      <textarea />
    </Dialog>,
  );
  await vi.waitFor(() => expect(typed.host.querySelector("textarea")).not.toBeNull());
  window.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
    }),
  );
  expect(sent).toHaveBeenCalledTimes(1);
  typed.root.unmount();

  const ended = vi.fn();
  const plain = mount(
    <Dialog open title="End it" confirmLabel="End" onConfirm={ended}>
      Ends the Job.
    </Dialog>,
  );
  await vi.waitFor(() => expect(plain.host.querySelector("[role=dialog]")).not.toBeNull());
  window.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
    }),
  );
  expect(ended).not.toHaveBeenCalled();
  plain.root.unmount();
});
