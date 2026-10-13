// New Session on All asks which repository it starts in: a session starts in one, and Fleet refuses
// to guess where it serves several.

import { SessionList } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function list(startIn: readonly { root: string; name: string }[] | undefined): (string | undefined)[] {
  const started: (string | undefined)[] = [];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <SessionList
      groups={[]}
      query=""
      onQuery={() => undefined}
      onOpen={() => undefined}
      onStart={(at) => started.push(at)}
      {...(startIn === undefined ? {} : { startIn })}
    />,
  );
  return started;
}

test("on All with two repositories, New Session starts in the one chosen", async () => {
  const started = list([
    { root: "/r/armada", name: "armada" },
    { root: "/r/ynap", name: "ynap" },
  ]);
  await userEvent.click(page.getByRole("button", { name: "New Session" }));
  await userEvent.click(page.getByRole("menuitem", { name: "ynap" }));
  expect(started).toEqual(["/r/ynap"]);
});

test("with one repository to start in, New Session starts there without asking", async () => {
  const started = list([{ root: "/r/armada", name: "armada" }]);
  await userEvent.click(page.getByRole("button", { name: "New Session" }));
  expect(started).toEqual(["/r/armada"]);
});

test("on a picked repository, New Session starts where the window is", async () => {
  const started = list(undefined);
  await userEvent.click(page.getByRole("button", { name: "New Session" }));
  expect(started).toEqual([undefined]);
});
