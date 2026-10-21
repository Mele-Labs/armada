// One listener at the shell for a paste or a drop, handing it to whichever claims it: a package first, then a link,
// whatever order the two were mounted in. Browser: real events dispatched on the page.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

import { useClaim, usePasteAndDrop, type Claimant } from "./paste-and-drop";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: Root[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) act(() => root.unmount());
  vi.restoreAllMocks();
});

function Shell({ claims }: { claims: [name: "package" | "link", claimant: Claimant][] }) {
  usePasteAndDrop();
  return (
    <>
      {claims.map(([name, claimant]) => (
        <Claims key={name} name={name} claimant={claimant} />
      ))}
    </>
  );
}

function Claims({ name, claimant }: { name: "package" | "link"; claimant: Claimant }) {
  useClaim(name, claimant);
  return null;
}

function mount(claims: [name: "package" | "link", claimant: Claimant][]) {
  const root = createRoot(document.createElement("div"));
  roots.push(root);
  act(() => root.render(<Shell claims={claims} />));
}

const paste = (text: string) => {
  const data = new DataTransfer();
  data.setData("text/plain", text);
  const event = new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true });
  document.body.dispatchEvent(event);
  return event;
};

const heard: string[] = [];
const claimant = (name: string, claims: boolean): Claimant => ({
  paste: (event) => (heard.push(`${name} paste`), claims && (event.preventDefault(), true)),
  over: () => (heard.push(`${name} over`), claims),
  drop: () => (heard.push(`${name} drop`), claims),
});

test("the window is listened on once for each of paste, dragover and drop, however many claim them", () => {
  const listening = vi.spyOn(window, "addEventListener");
  mount([
    ["link", claimant("link", false)],
    ["package", claimant("package", false)],
  ]);
  const kinds = listening.mock.calls.map(([kind]) => kind).filter((kind) => ["paste", "dragover", "drop"].includes(kind));
  expect(kinds.sort()).toEqual(["dragover", "drop", "paste"]);
});

test("a package is offered a paste first, mounted after the link or not, and a claim ends it", () => {
  heard.length = 0;
  mount([
    ["link", claimant("link", true)],
    ["package", claimant("package", true)],
  ]);
  paste("version = 1");
  expect(heard).toEqual(["package paste"]);
});

test("what the package leaves goes to the link", () => {
  heard.length = 0;
  mount([
    ["link", claimant("link", true)],
    ["package", claimant("package", false)],
  ]);
  paste("https://impeccable.style/");
  const data = new DataTransfer();
  document.body.dispatchEvent(new DragEvent("drop", { dataTransfer: data, bubbles: true, cancelable: true }));
  expect(heard).toEqual(["package paste", "link paste", "package drop", "link drop"]);
});

test("a paste something on the page already took is offered to nobody", () => {
  heard.length = 0;
  mount([["package", claimant("package", true)]]);
  const field = document.createElement("textarea");
  document.body.append(field);
  field.addEventListener("paste", (event) => event.preventDefault());
  const data = new DataTransfer();
  data.setData("text/plain", "version = 1");
  field.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  field.remove();
  expect(heard).toEqual([]);
});
