// Helm's message box on a Fleet that serves it, through `App` on the `helm-rich-composer` moment: what an ask
// carries to Fleet, what the thread's row keeps of it from the wire, and the box's model, effort and `/` list,
// which are the conversation's facts. The look is the walk's; this holds what the walk cannot see, which is
// what was sent and where each part of the box came from.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { HelmThread } from "@armada/protocol";
import { s211HelmRichComposer } from "./scenarios/helm-rich-composer";
import type { Scenario } from "./scenario";
import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

const helm = () => page.getByRole("complementary", { name: "Helm" });
const box = () => helm().getByRole("textbox", { name: "Ask Helm" });
const picture = () => new File(["png"], "shot.png", { type: "image/png" });

/** The moment with Helm's facts named: the commands its `/` lists, and the model and effort picked. */
function withFacts(facts: Extract<HelmThread, { state: "open" }>["facts"]): Scenario {
  const held = s211HelmRichComposer.state.helm;
  if (held.state !== "open") throw new Error("the moment holds an open conversation");
  return { ...s211HelmRichComposer, state: { ...s211HelmRichComposer.state, helm: { ...held, facts } } };
}

test("words, a picture, a sketch and an issue go to Helm as one ask, and the row keeps them from the wire", async () => {
  const app = mount("helm-rich-composer");
  const asked = vi.spyOn(app.api, "askHelm");
  await openHelm();

  await userEvent.upload(helm().getByLabelText("Files to attach"), picture());
  await userEvent.click(helm().getByRole("button", { name: "Draw sketch" }));
  const pad = page.getByRole("dialog", { name: "Draw a sketch" });
  await userEvent.click(pad.getByRole("button", { name: "Add a box" }));
  await userEvent.fill(page.getByRole("textbox", { name: "The words in this box" }), "pin the clock");
  await userEvent.click(pad.getByRole("button", { name: "Attach sketch" }));

  await userEvent.click(box());
  await userEvent.keyboard("see #2214");
  await userEvent.click(helm().getByRole("option", { name: /Sessions list drops a quiet Session/ }));
  await userEvent.keyboard("is it this?{Enter}");

  // The sketch is drawn to a picture before it is sent, which takes a moment.
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(1), { timeout: 5000 });
  const [text, context, carry] = asked.mock.calls[0]!;
  expect(text).toContain("is it this?");
  expect(context).toMatchObject({ screen: expect.any(String) });
  expect(carry?.attachments).toHaveLength(2);
  expect(carry?.attachments?.[0]).toEqual({ name: "shot.png", media_type: "image/png", data: btoa("png") });
  // A sketch goes as the picture it renders to.
  expect(carry?.attachments?.[1]).toMatchObject({ name: "pin the clock.png", media_type: "image/png" });
  expect(atob(carry!.attachments![1]!.data).startsWith("\x89PNG")).toBe(true);
  // An issue goes with its kind, its number and the forge's address.
  expect(carry?.mentions).toEqual([{ kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session on reload", url: "https://forge.invalid/issues/2214" }]);

  // The row is Fleet's: the picture is read back from it, the sketch is drawn as the sketch this window drew, and the tag is a link.
  const thumbnail = helm().getByRole("img", { name: /shot\.png/ });
  await expect.element(thumbnail).toBeVisible();
  expect((thumbnail.element() as HTMLImageElement).src.startsWith("blob:")).toBe(true);
  await expect.element(helm().getByText("pin the clock")).toBeVisible();
  expect(helm().getByText("pin the clock.png").query()).toBeNull();
  await expect.element(helm().getByRole("link", { name: /#2214 Sessions list drops a quiet Session/ })).toBeVisible();
  // The box was given back empty and nothing waits in it.
  await expect.element(box()).toHaveTextContent("");
  expect(helm().getByRole("group", { name: "Attached" }).query()?.children.length ?? 0).toBe(0);
});

test("a picture alone goes straight to Helm, without being read as a thought", async () => {
  const app = mount("helm-rich-composer");
  const asked = vi.spyOn(app.api, "askHelm");
  const read = vi.spyOn(app.api, "readThought");
  await openHelm();

  await userEvent.upload(helm().getByLabelText("Files to attach"), picture());
  await userEvent.click(helm().getByRole("button", { name: "Send" }));

  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(1));
  const [text, , carry] = asked.mock.calls[0]!;
  expect(text).toBe("");
  expect(carry).toEqual({ attachments: [{ name: "shot.png", media_type: "image/png", data: btoa("png") }] });
  expect(read).not.toHaveBeenCalled();
  await expect.element(helm().getByRole("img", { name: /shot\.png/ })).toBeVisible();
});

test("the model and effort are the conversation's: a pick is tuned there, shown from its facts, and another conversation keeps its own", async () => {
  const app = mount("helm-rich-composer");
  const tuned = vi.spyOn(app.api, "tuneHelm");
  await openHelm();

  const model = () => helm().getByRole("combobox", { name: "Model" });
  const effort = () => helm().getByRole("combobox", { name: "Effort" });
  await expect.element(model()).toHaveValue("");
  await expect.element(effort()).toHaveValue("");

  await userEvent.selectOptions(model(), "opus");
  await vi.waitFor(() => expect(tuned).toHaveBeenCalledWith("armada", "opus", null));
  await expect.element(model()).toHaveValue("opus");

  await userEvent.selectOptions(effort(), "high");
  await vi.waitFor(() => expect(tuned).toHaveBeenLastCalledWith("armada", "opus", "high"));
  await expect.element(effort()).toHaveValue("high");
  await expect.element(model()).toHaveValue("opus");

  // Another repository's conversation holds none of it.
  await userEvent.click(helm().getByRole("button", { name: /^Point Helm at a different repository/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await expect.element(model()).toHaveValue("");
  await expect.element(effort()).toHaveValue("");
  expect(tuned).toHaveBeenCalledTimes(2);
});

test("what the facts hold is what the box shows, with Auto where they name nothing", async () => {
  mount(withFacts({ model: "sonnet", effort: "low" }));
  await openHelm();

  await expect.element(helm().getByRole("combobox", { name: "Model" })).toHaveValue("sonnet");
  await expect.element(helm().getByRole("combobox", { name: "Effort" })).toHaveValue("low");
});

test("a pick Fleet refused is taken back, and Fleet's words are said", async () => {
  const app = mount("helm-rich-composer");
  app.api.tuneHelm = async () => ({
    ok: false,
    outcome: { ok: false, why: "refused", error: { code: "fleet.unacceptable_effort", message: "Effort is low, medium or high", run_id: "mock", fields: {}, chain: [] } },
  });
  await openHelm();

  await userEvent.selectOptions(helm().getByRole("combobox", { name: "Effort" }), "high");
  await expect.element(page.getByText("Effort is low, medium or high").first()).toBeVisible();
  await expect.element(helm().getByRole("combobox", { name: "Effort" })).toHaveValue("");
});

test("`/` lists the names the facts hold, and not the Sessions' own", async () => {
  mount(withFacts({ commands: ["only-helm", "also-helm"] }));
  await openHelm();

  await userEvent.click(box());
  await userEvent.keyboard("/");
  const list = helm().getByRole("listbox", { name: "Skills and commands" });
  await expect.element(list).toBeVisible();
  const names = [...list.element().querySelectorAll('[role="option"]')].map((one) => one.textContent);
  expect(names).toEqual(["/only-helm", "/also-helm"]);
});

test("a conversation whose facts name no commands offers none", async () => {
  mount(withFacts({}));
  await openHelm();

  await userEvent.click(box());
  await userEvent.keyboard("/");
  expect(helm().getByRole("listbox", { name: "Skills and commands" }).query()).toBeNull();
});

test("a thought sent on to a Session records what it went with, and the row Fleet kept draws it", async () => {
  const app = mount("helm-rich-composer");
  const recorded = vi.spyOn(app.api, "recordThoughtSent");
  const delivered = vi.spyOn(app.api, "sendSessionMessage");
  await openHelm();

  await userEvent.upload(helm().getByLabelText("Files to attach"), picture());
  await userEvent.click(box());
  await userEvent.keyboard("I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land{Enter}");
  await expect.element(helm().getByRole("group", { name: "Send to", exact: true })).toBeVisible();
  await userEvent.keyboard("{Enter}");

  await expect.element(helm().getByRole("button", { name: "Sent to Session Sessions list views" })).toBeVisible();
  expect(delivered).toHaveBeenCalledTimes(1);
  expect(recorded).toHaveBeenCalledTimes(1);
  expect(recorded.mock.calls[0]![3]).toEqual({ attachments: [{ name: "shot.png", media_type: "image/png", data: btoa("png") }] });
  // The picture is the row's own, read back from Fleet.
  const thumbnail = helm().getByRole("img", { name: /shot\.png/ });
  await expect.element(thumbnail).toBeVisible();
  expect((thumbnail.element() as HTMLImageElement).src.startsWith("blob:")).toBe(true);
});
