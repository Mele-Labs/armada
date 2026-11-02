// A Job's thread look is one copy Fleet keeps under `thread.look`, read and written by the Aa panel in the Job's head and by
// System › Thread alike. A window with no scenario seed holds it: the defaults when nothing is saved, a saved value laid over them.
// Through `App` over the `session-head` scenario, which hands no look.

import { DEFAULT_LOOK } from "@armada/components";
import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s235SessionHead } from "./scenarios/session-head";
import { mount, onScreen, toSession, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

const KEY = "thread.look";

async function job() {
  const app = mount("session-head");
  await onScreen();
  const view = await toSession(/How does armada interface/);
  return {
    app,
    view,
  };
}

test("with no seed and nothing saved, a Job's head draws Aa and the panel reads the defaults", async () => {
  const { view } = await job();
  await userEvent.click(
    view.getByRole("button", {
      name: "Thread look",
    }),
  );
  const panel = page.getByRole("dialog", {
    name: "Thread look",
  });
  await expect
    .element(
      panel.getByRole("textbox", {
        name: "Agent name",
      }),
    )
    .toHaveValue(DEFAULT_LOOK.agentName);
  await expect
    .element(
      panel.getByRole("switch", {
        name: "Show times",
      }),
    )
    .toBeChecked();
  await expect
    .element(
      panel.getByRole("switch", {
        name: "Show agent name",
      }),
    )
    .toBeChecked();
  await expect
    .element(
      panel.getByRole("combobox", {
        name: "Text size",
      }),
    )
    .toHaveDisplayValue("15 px");
});

test("a change in Aa is saved to thread.look, whole, after the typing stops", async () => {
  const { app, view } = await job();
  const save = vi.spyOn(app.api, "saveSettings");
  await userEvent.click(
    view.getByRole("button", {
      name: "Thread look",
    }),
  );
  await userEvent.type(
    page
      .getByRole("dialog", {
        name: "Thread look",
      })
      .getByRole("textbox", {
        name: "Agent name",
      }),
    "Xavier",
  );
  await expect.poll(() => save.mock.calls.length).toBe(1);
  expect(save).toHaveBeenCalledWith({
    [KEY]: {
      ...DEFAULT_LOOK,
      agentName: "Xavier",
    },
  });
});

test("a saved look is drawn: the agent's rows carry its name, and a field it lacks takes the default", async () => {
  const { app, view } = await job();
  await app.api.saveSettings({
    [KEY]: {
      agentName: "Xavier",
    },
  });
  await expect.element(view.getByText("Xavier").first()).toBeVisible();
  await userEvent.click(
    view.getByRole("button", {
      name: "Thread look",
    }),
  );
  const panel = page.getByRole("dialog", {
    name: "Thread look",
  });
  await expect
    .element(
      panel.getByRole("textbox", {
        name: "Agent name",
      }),
    )
    .toHaveValue("Xavier");
  await expect
    .element(
      panel.getByRole("switch", {
        name: "Show times",
      }),
    )
    .toBeChecked();
});

test("System › Thread reads and writes the same copy", async () => {
  const { app } = await job();
  await app.api.saveSettings({
    [KEY]: {
      agentName: "Xavier",
    },
  });
  const save = vi.spyOn(app.api, "saveSettings");
  await toSystemPage("Thread");
  const name = page.getByRole("textbox", {
    name: "Agent name",
  });
  await expect.element(name).toHaveValue("Xavier");
  await userEvent.clear(name);
  await userEvent.type(name, "Odile");
  await expect.poll(() => save.mock.calls.length).toBe(1);
  expect(save).toHaveBeenCalledWith({
    [KEY]: {
      ...DEFAULT_LOOK,
      agentName: "Odile",
    },
  });
});

test("Talk hears through this Mac's voice with the engine the look names, into the box it hands focus to", async () => {
  const listen = vi.fn((_engine: string, onWord: (word: string) => void) => {
    onWord("ship");
    onWord("it");
    return () => undefined;
  });
  const voice = {
    listen,
    whisper: async () => ({
      state: "absent" as const,
    }),
    onWhisper: () => () => undefined,
    installWhisper: async () => ({
      ok: true as const,
    }),
  };
  const app = mount({
    ...s235SessionHead,
    behaves: (handle) => ({
      ...s235SessionHead.behaves!(handle),
      voice,
    }),
  });
  await onScreen();
  await app.api.saveSettings({
    [KEY]: {
      voice: "dictation",
    },
  });
  const view = await toSession(/How does armada interface/);
  await userEvent.click(
    view.getByRole("button", {
      name: "Talk",
    }),
  );
  expect(listen).toHaveBeenCalledWith("dictation", expect.any(Function));
  const box = view.getByRole("textbox", {
    name: "Message",
  });
  await expect.element(box).toHaveFocus();
  await expect.element(box).toHaveTextContent("ship it");
});
