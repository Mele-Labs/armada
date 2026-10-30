// Every End asks the same way. A Job holding a deploy that is rolling out lists it first and ends only on End job, which also runs what the
// press wanted done after (the Cockpit's call clears itself); Keep going ends nothing. A Job holding nothing ends at once.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import "./styles/index.css";
import { useEffect } from "react";
import { askToEnd, dropAsk } from "./end-job";
import { SessionsFrom, useSessionsDraft } from "./sessions-draft";

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
});

/** A Job holding a deploy that is rolling out, as the lanes carry it. */
const HOLDING = [
  {
    id: "s-1",
    lanes: {
      job: {
        copy: { lifecycles: [{ id: "deploy", states: [{ id: "rolling_out", label: "Rolling out" }] }] },
        instances: [{ id: "deploy:Deploy", lifecycle: "deploy", label: "Deploy", state: "rolling_out" }],
      },
    },
  },
];
const NOTHING = [{ id: "s-1" }];

const serving = (sessions: readonly object[], close: (id: string) => void) => ({ get: () => sessions, subscribe: () => () => undefined, close }) as unknown as SessionsDraft;

function End({ then }: { then: () => void }) {
  const draft = useSessionsDraft();
  return <button onClick={() => draft !== undefined && askToEnd(draft, "s-1", then)}>End</button>;
}

function mount(draft: SessionsDraft, then: () => void): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  root.render(
    <SessionsFrom held={draft}>
      <End then={then} />
    </SessionsFrom>,
  );
}

test("a Job holding something in flight lists it first, and ends only on End job", async () => {
  const close = vi.fn();
  const then = vi.fn();
  mount(serving(HOLDING, close), then);

  await page.getByRole("button", { name: "End", exact: true }).click();
  await expect.element(page.getByText("Deploy: Rolling out")).toBeVisible();
  expect(close).not.toHaveBeenCalled();

  await page.getByRole("button", { name: "Keep going" }).click();
  await expect.element(page.getByText("Deploy: Rolling out")).not.toBeInTheDocument();
  expect(close).not.toHaveBeenCalled();
  expect(then).not.toHaveBeenCalled();

  await page.getByRole("button", { name: "End", exact: true }).click();
  await page.getByRole("button", { name: "End job" }).click();
  expect(close).toHaveBeenCalledTimes(1);
  expect(close).toHaveBeenCalledWith("s-1");
  expect(then).toHaveBeenCalledTimes(1);
  await expect.element(page.getByText("Deploy: Rolling out")).not.toBeInTheDocument();
});

test("a Job holding nothing in flight ends at once, with no list", async () => {
  const close = vi.fn();
  const then = vi.fn();
  mount(serving(NOTHING, close), then);

  await page.getByRole("button", { name: "End", exact: true }).click();

  expect(close).toHaveBeenCalledTimes(1);
  expect(close).toHaveBeenCalledWith("s-1");
  expect(then).toHaveBeenCalledTimes(1);
  expect(document.querySelector(".sv-inflight")).toBeNull();
});

test("the list shuts when the Job it was asked on leaves the screen", async () => {
  const close = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  // The Job's view, as `sessions.tsx` drops the ask when it unmounts.
  const View = () => {
    useEffect(() => () => dropAsk("s-1"), []);
    return <End then={() => undefined} />;
  };
  const render = (shown: boolean) => root.render(<SessionsFrom held={serving(HOLDING, close)}>{shown ? <View /> : null}</SessionsFrom>);
  render(true);

  await page.getByRole("button", { name: "End", exact: true }).click();
  await expect.element(page.getByText("Deploy: Rolling out")).toBeVisible();

  render(false);
  await expect.element(page.getByText("Deploy: Rolling out")).not.toBeInTheDocument();
  expect(close).not.toHaveBeenCalled();
});
