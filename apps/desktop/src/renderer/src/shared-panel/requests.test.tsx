// Environments: a variable's Remove drops its line from what Save sends, and Delete takes an environment of the person's own
// out after a second press, never a repository's or a server's. Browser: a rendering is read.

import { useState } from "react";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Requests } from "@armada/protocol";

import "../styles/index.css";
import type { RequestsAnswer, RequestsApi } from "../../../shared/api/requests";
import { Environments } from "./requests";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const READ: Requests = {
  repository: "/work/tidewater",
  place: "this_mac",
  collections: [],
  environments: [
    {
      name: "test",
      variables: [
        { name: "base", secret: false, value: "https://api.example.test" },
        { name: "token", secret: true, set: true },
      ],
      place: "yours",
      hides: false,
    },
    { name: "local", variables: [{ name: "base", secret: false, value: "http://localhost:5173" }], place: "repository", hides: false },
    { name: "server:web", variables: [{ name: "base", secret: false, value: "http://127.0.0.1:5173" }], place: "server", hides: false },
  ],
  not_read: [],
};

const answered = (value: Requests): RequestsAnswer<Requests> => ({ ok: true, value });

function fakeApi() {
  const save = vi.fn(async (..._: Parameters<RequestsApi["saveRequestEnvironment"]>) => answered(READ));
  const remove = vi.fn(async (name: string, ..._: [string | undefined, string | undefined]) => answered({ ...READ, environments: READ.environments.filter((one) => one.name !== name) }));
  const api: RequestsApi = {
    readRequests: async () => answered(READ),
    saveRequestCollection: async () => answered(READ),
    saveRequestEnvironment: save,
    deleteRequestEnvironment: remove,
    sendRequest: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }),
  };
  return { api, save, remove };
}

/** Holds the read as the panel does, so an answer that comes back redraws the list. */
function Held({ api }: { api: RequestsApi }) {
  const [read, setRead] = useState(READ);
  return <Environments api={api} read={read} sessionId="session-1" repository={undefined} locked={false} onRead={setRead} />;
}

let root: Root | undefined;
let host: HTMLElement | undefined;
async function mount(api: RequestsApi) {
  host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
  await act(async () => root!.render(<Held api={api} />));
}

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
});

const group = (name: string) => document.querySelector<HTMLElement>(`form[role="group"][aria-label="${name}"], section[role="group"][aria-label="${name}"]`);
const buttonIn = (where: HTMLElement | null, label: string) =>
  [...(where?.querySelectorAll<HTMLButtonElement>("button") ?? [])].find((one) => one.getAttribute("aria-label") === label || one.textContent?.trim() === label);
const press = (button: HTMLButtonElement | undefined) => act(async () => button?.click());

test("Remove on a variable's row drops that variable from what Save sends", async () => {
  const { api, save } = fakeApi();
  await mount(api);
  const test = group("test");
  await press(buttonIn(test, "Remove variable 2"));
  expect(group("test")?.querySelector('[aria-label="Variable 2"]')).toBeNull();
  await press(buttonIn(test, "Save"));
  expect(save).toHaveBeenCalledTimes(1);
  expect(save.mock.calls[0]?.[0]).toEqual({ name: "test", variables: [{ name: "base", secret: false, value: "https://api.example.test" }] });
});

test("Delete asks again before it deletes, then the environment leaves the list", async () => {
  const { api, remove } = fakeApi();
  await mount(api);
  await press(buttonIn(group("test"), "Delete"));
  expect(remove).not.toHaveBeenCalled();
  expect(buttonIn(group("test"), "Press again to delete")).toBeDefined();
  await press(buttonIn(group("test"), "Press again to delete"));
  expect(remove).toHaveBeenCalledTimes(1);
  expect(remove.mock.calls[0]).toEqual(["test", "session-1", undefined]);
  expect(group("test")).toBeNull();
  expect(group("local")).not.toBeNull();
});

test("Delete is offered on the person's own environment and on no other", async () => {
  const { api } = fakeApi();
  await mount(api);
  expect(buttonIn(group("test"), "Delete")).toBeDefined();
  expect(buttonIn(group("local"), "Delete")).toBeUndefined();
  expect(group("server:web")).not.toBeNull();
  expect(buttonIn(group("server:web"), "Delete")).toBeUndefined();
});
