// A note to a paused Job waits on his choice: Resume and send resumes first and sends after, Cancel sends nothing and answers false so the box
// gets its words back, and a Job that is not paused sends at once. Browser: the confirm is a rendered layer.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

import type { Session } from "@armada/screens/src/draft/sessions";

import { usePausedSend } from "./paused-send";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

function mount(session: Pick<Session, "paused">, resume: (id: string) => Promise<boolean>, go: () => Promise<boolean>, answers: boolean[]) {
  function Box() {
    const { ask, layer } = usePausedSend({ resume }, { id: "j", ...session } as Session);
    return (
      <>
        <button type="button" onClick={() => void Promise.resolve(ask(go)).then((went) => answers.push(went === true))}>
          Send
        </button>
        {layer}
      </>
    );
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Box />));
  roots.push({ root, host });
}

const press = async (name: RegExp) => {
  const button = [...document.querySelectorAll("button")].find((one) => name.test(one.textContent ?? ""))!;
  await act(async () => button.click());
};
const flush = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 20))));

test("a paused Job opens the confirm and sends nothing until he chooses; Resume and send resumes, then sends", async () => {
  const order: string[] = [];
  const answers: boolean[] = [];
  mount({ paused: true }, async () => (order.push("resume"), true), async () => (order.push("send"), true), answers);
  await press(/^Send$/);
  expect(document.querySelector('[role="dialog"]')?.getAttribute("aria-label")).toBe("Job is paused");
  expect(order).toEqual([]);
  await flush();
  expect(document.activeElement?.textContent).toMatch(/^Resume and send/);
  await press(/^Resume and send/);
  await flush();
  expect(order).toEqual(["resume", "send"]);
  expect(answers).toEqual([true]);
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

test("Cancel sends and resumes nothing, and answers false so the box keeps the draft", async () => {
  const resume = vi.fn(async () => true);
  const go = vi.fn(async () => true);
  const answers: boolean[] = [];
  mount({ paused: true }, resume, go, answers);
  await press(/^Send$/);
  await press(/^Cancel/);
  await flush();
  expect(resume).not.toHaveBeenCalled();
  expect(go).not.toHaveBeenCalled();
  expect(answers).toEqual([false]);
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

test("a refused Resume sends nothing", async () => {
  const go = vi.fn(async () => true);
  const answers: boolean[] = [];
  mount({ paused: true }, async () => false, go, answers);
  await press(/^Send$/);
  await press(/^Resume and send/);
  await flush();
  expect(go).not.toHaveBeenCalled();
  expect(answers).toEqual([false]);
});

test("a Job that is not paused sends at once, with no confirm", async () => {
  const go = vi.fn(async () => true);
  mount({}, async () => true, go, []);
  await press(/^Send$/);
  await flush();
  expect(go).toHaveBeenCalledOnce();
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});
