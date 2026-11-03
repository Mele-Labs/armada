// The notes held on a Job, in its thread. On a real Bridge main keeps them and tells them as they change, and the thread acts on them through main; the mock keeps them in `store.ts`.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import type { PanelHeldNote } from "@armada/studios/api";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { SessionsFrom } from "../sessions-draft";
import { useTrailingNotes } from "./held";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const job = {
  id: "job",
  title: "Billing",
  rows: [],
  attachments: [],
} as unknown as Session;
const jobs = [
  job,
];
const held = {
  get: () => jobs,
  subscribe: () => () => undefined,
} as unknown as SessionsDraft;

function Thread() {
  return <>{useTrailingNotes(job)}</>;
}

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  (
    window as {
      armada?: unknown;
    }
  ).armada = undefined;
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

test("on a real Bridge the thread shows the notes main tells, and removing one or sending them goes through main", async () => {
  const note: PanelHeldNote = {
    id: "n1",
    at: "2026-11-03T10:00:00Z",
    said: "The plan card needs the renewal date",
    on: "Billing",
  };
  let tell: (notes: PanelHeldNote[]) => void = () => undefined;
  const off = vi.fn();
  const api = {
    held: vi.fn((_: string, onChanged: (notes: PanelHeldNote[]) => void) => {
      tell = onChanged;
      return off;
    }),
    editNote: vi.fn(async () => ({
      ok: true as const,
    })),
    removeNote: vi.fn(async () => ({
      ok: true as const,
    })),
    sendNotes: vi.fn(async () => ({
      ok: true as const,
    })),
  };
  (
    window as {
      armada?: unknown;
    }
  ).armada = {
    jobWalk: api,
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  await act(async () =>
    root.render(
      <SessionsFrom held={held}>
        <Thread />
      </SessionsFrom>,
    ),
  );
  expect(api.held).toHaveBeenCalledWith("job", expect.any(Function));
  expect(document.querySelector(".armada-held-notes")).toBeNull();
  await act(async () =>
    tell([
      note,
    ]),
  );
  await expect
    .element(
      page.getByRole("listitem", {
        name: note.said,
      }),
    )
    .toBeVisible();
  await page
    .getByRole("button", {
      name: `Remove the note “${note.said}”`,
    })
    .click();
  expect(api.removeNote).toHaveBeenCalledWith("job", "n1");
  await page
    .getByRole("button", {
      name: "Send notes",
    })
    .click();
  expect(api.sendNotes).toHaveBeenCalledWith("job");
  await act(async () => root.unmount());
  expect(off).toHaveBeenCalled();
});
