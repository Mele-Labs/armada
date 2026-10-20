// A file the agent shows opens in the panel for the Session on screen, and for no other: a `shown` seen when the Session came
// on screen, or on a Session that is not, opens nothing. Browser: the hook runs in a rendering.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { Session } from "@armada/screens/src/draft/sessions";
import { afterEach, beforeEach, expect, test } from "vitest";

import { useShownFiles } from "./session-shown-file";
import { forgetPanel, hostsPanel, usePanel } from "./shared-panel/store";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Held = Pick<Session, "id" | "attachments">;

const shown = (path: string, at: string): Session["attachments"][number] => ({ kind: "artifact", form: "file", id: path, title: path, shown: at, by: "agent" });
const OLD = shown("/work/old.md", "2026-10-19T14:00:00.000Z");
const NEW = shown("/work/new.md", "2026-10-19T14:05:00.000Z");

let root: Root;
let leave: () => void;
let open: ReturnType<typeof usePanel>["open"];

function Screen({ session }: { session: Held }) {
  useShownFiles(session);
  open = usePanel().open;
  return null;
}

const show = (session: Held) => act(async () => root.render(<Screen session={session} />));

beforeEach(() => {
  root = createRoot(document.createElement("div"));
  leave = hostsPanel();
});
afterEach(() => {
  act(() => root.unmount());
  leave();
  forgetPanel();
});

test("a show that arrives while the Session is on screen opens the file", async () => {
  await show({ id: "s1", attachments: [OLD] });
  expect(open).toBeUndefined();
  await show({ id: "s1", attachments: [OLD, NEW] });
  expect(open).toEqual({ kind: "file", path: "/work/new.md", from: "session", sessionId: "s1" });
});

test("a show the Session already held when it came on screen opens nothing, and nor does a Session switched to", async () => {
  await show({ id: "s1", attachments: [OLD] });
  expect(open).toBeUndefined();
  await show({ id: "s2", attachments: [NEW] });
  expect(open).toBeUndefined();
});

test("the same path shown again is a new show", async () => {
  await show({ id: "s1", attachments: [OLD] });
  await show({ id: "s1", attachments: [shown("/work/old.md", "2026-10-19T14:09:00.000Z")] });
  expect(open).toMatchObject({ kind: "file", path: "/work/old.md" });
});
