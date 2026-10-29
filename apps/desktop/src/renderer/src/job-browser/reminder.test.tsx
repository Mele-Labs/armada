// The thread's line where a Job reached for a browser outside Bridge: its press tells the Job once, and the line then says so.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { Session, SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { useBrowserReminders } from "./JobBrowserPane";

const REMINDER: SessionRow = { id: "r1", at: "2026-10-28T10:00:00Z", kind: "browser_reminder", tool: "Playwright", address: "http://localhost:5173/checkout" };

const sessionWith = (rows: SessionRow[]): Session => ({ id: "s1", title: "Fix the checkout total", turn: { state: "idle" }, rows, attachments: [] }) as unknown as Session;

/** A draft that holds the thread as given and keeps what was sent. */
function fleet(rows: SessionRow[]) {
  const sent: string[] = [];
  const sessions = [sessionWith(rows)];
  const draft = {
    get: () => sessions,
    subscribe: () => () => undefined,
    send: async (_sessionId: string, message: { text: string }) => {
      sent.push(message.text);
      return true;
    },
  } as unknown as SessionsDraft;
  return { draft, sent };
}

function Cards() {
  return <>{useBrowserReminders("s1").map((one) => <div key={one.key}>{one.node}</div>)}</>;
}

let mounted: { root: Root; host: HTMLElement } | undefined;
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const next = () => new Promise<void>((done) => setTimeout(done, 20));

async function mount(draft: SessionsDraft): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={draft}>
      <Cards />
    </SessionsFrom>,
  );
  await next();
  return host;
}

const button = (host: HTMLElement) => [...host.querySelectorAll("button")].find((one) => one.textContent === "Open Job browser");

test("a press tells the Job once, as the owner, where it was, and the line then reads Sent to the job", async () => {
  const { draft, sent } = fleet([REMINDER]);
  const host = await mount(draft);
  const press = button(host);
  expect(press).toBeDefined();
  press!.click();
  press!.click();
  await next();
  expect(sent).toEqual(["Use the Job browser for this. You were on http://localhost:5173/checkout."]);
  expect(button(host)).toBeUndefined();
  expect(host.textContent).toContain("Sent to the job");
});

test("reads Sent to the job from the thread, where the owner already told the Job", async () => {
  const told: SessionRow = { id: "m1", at: "2026-10-28T10:01:00Z", kind: "message", from: { kind: "you" }, text: "Use the Job browser for this." } as SessionRow;
  const { draft } = fleet([{ ...REMINDER, address: undefined } as SessionRow, told]);
  const host = await mount(draft);
  expect(button(host)).toBeUndefined();
  expect(host.textContent).toContain("Sent to the job");
});
