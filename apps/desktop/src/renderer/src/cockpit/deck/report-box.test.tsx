// Report opens the Job's own message box in a wide dialog: a file attached and a tag chosen go to the new Session with the owner's words
// and the brief. Browser: a press and a typed report are what is read, from a fake `window.armada`.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { NOTHING_YET } from "../../../../shared/bridge";
import "../../styles/index.css";
import { SessionTiles } from "./SessionTiles";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function mount() {
  const state = { ...NOTHING_YET, connection: { state: "connected" } } as unknown as BridgeState;
  (window as unknown as { armada: unknown }).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: state, seq: 1 }),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: { version: 1, job: [], deck: [] } } }),
    getRepositoryFlightControls: async () => ({ ok: true, value: { repository: "/Users/user/widgets", place: "this_mac", read: { what: "read", controls: { version: 1, job: [], deck: [] } }, in_repository: { what: "read", controls: { version: 1, job: [], deck: [] } }, asking: [] } }),
    searchFiles: async () => [],
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const session = { id: "s1", rows: [], attachments: [], turn: { state: "idle" } } as unknown as Session;
  const draft = { commands: [], get: () => [], start: vi.fn(async () => "new1"), send: vi.fn(async () => true) } as unknown as SessionsDraft & { send: ReturnType<typeof vi.fn> };
  root.render(<SessionTiles session={session} draft={draft} repository={{ root: "/Users/user/widgets", name: "acme/widgets" }} />);
  return draft;
}

test("a file attached to a report goes to the new Session with the brief", async () => {
  const draft = mount();
  await userEvent.click(page.getByRole("button", { name: "Report issue with job" }));
  await expect.element(page.getByRole("button", { name: "Draw sketch" })).not.toBeInTheDocument();
  await userEvent.upload(page.getByLabelText("Files to attach"), new File(["x"], "shot.png", { type: "image/png" }));
  await userEvent.click(page.getByRole("textbox", { name: "Report" }));
  await userEvent.keyboard("it pushed to main{Enter}");
  await expect.poll(() => draft.send.mock.calls.length).toBe(1);
  const [id, sent] = draft.send.mock.calls[0]! as [string, { text: string; files: { name: string }[] }];
  expect(id).toBe("new1");
  expect(sent.text.startsWith("it pushed to main")).toBe(true);
  expect(sent.files.map((one) => one.name)).toEqual(["shot.png"]);
});
