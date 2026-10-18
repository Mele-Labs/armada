// Helm's message box on a Fleet that serves it, through `App` on the `helm-rich-composer` moment: what an ask
// carries to Fleet, what the Helm Session's thread keeps of it from the wire, and the box's model, effort and `/`
// list, which are a Session's. The look is the walk's; this holds what the walk cannot see, which is what was
// sent and where each part of the box came from.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

const helm = () => page.getByRole("complementary", { name: "Helm" });
const box = () => helm().getByRole("textbox", { name: "Ask Helm" });
const picture = () => new File(["png"], "shot.png", { type: "image/png" });

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

  // The row is the Session's: each picture is read back from Fleet as a Session's is, and the tag is a chip in the row.
  const thumbnail = helm().getByRole("img", { name: /shot\.png/ });
  await expect.element(thumbnail).toBeVisible();
  expect((thumbnail.element() as HTMLImageElement).src.startsWith("blob:")).toBe(true);
  await expect.element(helm().getByRole("img", { name: /pin the clock\.png/ })).toBeVisible();
  await expect.element(helm().getByText(/#2214 Sessions list drops a quiet Session/).first()).toBeVisible();
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

/** Send a message to Helm so the repository has its Session, and wait for the reply to be in the thread. */
async function startTheSession(): Promise<void> {
  await userEvent.click(box());
  await userEvent.keyboard("why did Job 12 fail?{Enter}");
  await expect.element(helm().getByText(/Neither touches the Sessions page/)).toBeVisible();
}

test("before the Session exists the pick is held and goes with the first message, and afterwards it is tuned as a Session's is", async () => {
  const app = mount("helm-rich-composer");
  const asked = vi.spyOn(app.api, "askHelm");
  const tuned = vi.spyOn(app.api, "tuneSession");
  await openHelm();

  const model = () => helm().getByRole("combobox", { name: "Model" });
  const effort = () => helm().getByRole("combobox", { name: "Effort" });
  await expect.element(model()).toHaveValue("");
  await expect.element(effort()).toHaveValue("");

  // No Session to tune yet: the pick is the box's and nothing is sent.
  await userEvent.selectOptions(model(), "opus");
  await userEvent.selectOptions(effort(), "high");
  await expect.element(model()).toHaveValue("opus");
  await expect.element(effort()).toHaveValue("high");
  expect(tuned).not.toHaveBeenCalled();

  // The message that starts the Session carries it.
  await startTheSession();
  expect(asked.mock.calls[0]![2]).toEqual({ model: "opus", effort: "high" });
  await expect.element(model()).toHaveValue("opus");
  await expect.element(effort()).toHaveValue("high");

  // A live Session is tuned through `tune_session`, keeping the mode it runs in.
  await userEvent.selectOptions(effort(), "low");
  await vi.waitFor(() => expect(tuned).toHaveBeenCalledTimes(1));
  expect(tuned.mock.calls[0]![0]).toMatchObject({ model: "opus", effort: "low", mode: "auto" });
  await expect.element(effort()).toHaveValue("low");

  // Another repository's Helm holds none of it.
  await userEvent.click(helm().getByRole("button", { name: /^Point Helm at a different repository/ }));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await expect.element(model()).toHaveValue("");
  await expect.element(effort()).toHaveValue("");
});

test("a pick Fleet refused is taken back, and Fleet's words are said", async () => {
  const app = mount("helm-rich-composer");
  await openHelm();
  await startTheSession();
  app.api.tuneSession = async () => ({
    ok: false,
    outcome: { ok: false, why: "refused", error: { code: "fleet.unacceptable_effort", message: "Effort is low, medium or high", run_id: "mock", fields: {}, chain: [] } },
  });

  await userEvent.selectOptions(helm().getByRole("combobox", { name: "Effort" }), "high");
  await expect.element(page.getByText("Effort is low, medium or high").first()).toBeVisible();
  await expect.element(helm().getByRole("combobox", { name: "Effort" })).toHaveValue("");
});

test("`/` lists the names a Session lists, as the Sessions box does", async () => {
  mount("helm-rich-composer");
  await openHelm();

  await userEvent.click(box());
  await userEvent.keyboard("/");
  const list = helm().getByRole("listbox", { name: "Skills and commands" });
  await expect.element(list).toBeVisible();
  const names = [...list.element().querySelectorAll('[role="option"]')].map((one) => one.textContent);
  expect(names).toEqual(["/review", "/simplify", "/security-review", "/init", "/compact", "/clear", "/model"]);
});

test("Start fresh ends the Helm Session, and the next message starts a new one", async () => {
  const app = mount("helm-rich-composer");
  const asked = vi.spyOn(app.api, "askHelm");
  app.api.readThought = async () => ({ ok: true, reading: { kind: "question" } });
  await openHelm();
  await startTheSession();

  await userEvent.click(helm().getByRole("button", { name: "Start fresh" }));
  await expect.element(helm().getByText(/Neither touches the Sessions page/)).not.toBeInTheDocument();

  await userEvent.click(box());
  await userEvent.keyboard("and now?{Enter}");
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(2));
  await expect.element(helm().getByText("and now?")).toBeVisible();
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

  await expect.element(helm().getByRole("button", { name: "Sent to Job Sessions list views" })).toBeVisible();
  expect(delivered).toHaveBeenCalledTimes(1);
  expect(recorded).toHaveBeenCalledTimes(1);
  expect(recorded.mock.calls[0]![3]).toEqual({ attachments: [{ name: "shot.png", media_type: "image/png", data: btoa("png") }] });
  // The picture is the row's own, read back from Fleet.
  const thumbnail = helm().getByRole("img", { name: /shot\.png/ });
  await expect.element(thumbnail).toBeVisible();
  expect((thumbnail.element() as HTMLImageElement).src.startsWith("blob:")).toBe(true);
});
