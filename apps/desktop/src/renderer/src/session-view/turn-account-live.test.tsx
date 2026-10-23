// The live view in Progress, in Electron: a picture of the page that main takes and takes again, scaled to the frame, with Open on it. A press opens the page itself.
// The bridge is stubbed; a real window's main holds the page out of sight. Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { PageShot, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionsFrom } from "../sessions-draft";
import { TurnAccountBlock } from "./TurnAccountBlock";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const URL_OF_PAGE = "http://127.0.0.1:5199/settings";
const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const PNG = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="), (one) => one.charCodeAt(0));

/** A draft whose page view is a stub: what it was asked, and a way to hand the window a picture. */
function bridge() {
  const heard = new Set<(shot: PageShot) => void>();
  const snapshot = { watch: vi.fn(), stop: vi.fn(), onShot: (on: (shot: PageShot) => void) => (heard.add(on), () => void heard.delete(on)) };
  const page = { show: vi.fn(), move: vi.fn(), hide: vi.fn(), onEscape: () => () => undefined, snapshot };
  const none: Session[] = [];
  const draft = { get: () => none, subscribe: () => () => undefined, page } as unknown as SessionsDraft;
  return { draft, page, snapshot, shoot: (shot: PageShot) => act(() => heard.forEach((on) => on(shot))) };
}

function mount(draft: SessionsDraft, onOpen: (url: string) => void): HTMLElement {
  const host = document.createElement("div");
  host.style.width = "400px";
  document.body.append(host);
  const root = createRoot(host);
  const session = { id: "s1", attachments: [], rows: [], turn: { state: "running" }, live: { url: URL_OF_PAGE, title: "Notification settings" } } as unknown as Session;
  act(() =>
    root.render(
      <SessionsFrom held={draft}>
        <TurnAccountBlock session={session} lines={{ done: [], now: "Drawing the settings page" }} live={{ open: false, onOpen }} />
      </SessionsFrom>,
    ),
  );
  roots.push({ root, host });
  return host;
}

test("asks main for a picture of the page rather than a view over the panel, and ends it when the frame goes", () => {
  const { draft, page, snapshot } = bridge();
  const host = mount(draft, () => undefined);
  expect(snapshot.watch).toHaveBeenCalledWith("s1", URL_OF_PAGE);
  expect(page.show).not.toHaveBeenCalled();
  act(() => roots[0]!.root.unmount());
  expect(snapshot.stop).toHaveBeenCalled();
  host.remove();
});

test("draws the picture main sends for this page, scaled in the frame, and not one for another page", () => {
  const { draft, shoot } = bridge();
  const host = mount(draft, () => undefined);
  expect(host.querySelector(".sv-account__frame img")).toBeNull();
  shoot({ address: "http://127.0.0.1:5199/elsewhere", png: PNG });
  expect(host.querySelector(".sv-account__frame img")).toBeNull();
  shoot({ address: URL_OF_PAGE, png: PNG });
  const drawn = host.querySelector<HTMLImageElement>(".sv-account__frame img");
  expect(drawn?.src.startsWith("blob:")).toBe(true);
});

test("a press on the frame opens the page itself", () => {
  const { draft, shoot } = bridge();
  const opened = vi.fn();
  const host = mount(draft, opened);
  shoot({ address: URL_OF_PAGE, png: PNG });
  act(() => host.querySelector<HTMLButtonElement>('button[aria-label="Open Notification settings"]')!.click());
  expect(opened).toHaveBeenCalledExactlyOnceWith(URL_OF_PAGE);
});
