// A list of things made keeps the five most recent until he asks for the rest: Show earlier above a list that runs oldest first, Show more under
// one that runs newest first. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { File } from "lucide-react";
import { afterEach, expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Made } from "./ledger";
import { MadeList } from "./StoryParts";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const made = (n: number): Made[] => Array.from({ length: n }, (_, i) => ({ shape: "row", key: `m${i}`, bucket: "artifact", row: { key: `m${i}`, Glyph: File, said: "File", title: `Thing ${i}` } }));

function mount(newest: "first" | "last"): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<MadeList session={{ id: "j" } as unknown as Session} list={made(8)} newest={newest} />));
  roots.push({ root, host });
  return host;
}

const shown = (host: HTMLElement) => [...host.querySelectorAll("li")].flatMap((one) => /Thing \d/.exec(one.textContent ?? "")?.[0] ?? []);
const press = (host: HTMLElement, label: string) => act(() => [...host.querySelectorAll("button")].find((one) => one.textContent === label)!.click());

test("oldest first, it shows the last five and Show earlier opens the rest", () => {
  const host = mount("last");
  expect(shown(host)).toEqual(["Thing 3", "Thing 4", "Thing 5", "Thing 6", "Thing 7"]);
  expect(host.querySelector("li")?.textContent).toBe("Show earlier");
  press(host, "Show earlier");
  expect(shown(host)).toHaveLength(8);
  expect(host.textContent).not.toContain("Show earlier");
});

test("newest first, it shows the first five and Show more under them opens the rest", () => {
  const host = mount("first");
  expect(shown(host)).toEqual(["Thing 0", "Thing 1", "Thing 2", "Thing 3", "Thing 4"]);
  expect([...host.querySelectorAll("li")].pop()?.textContent).toBe("Show more");
  press(host, "Show more");
  expect(shown(host)).toHaveLength(8);
});
