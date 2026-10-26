// Sessions on a real Fleet's path, through `App`: the window is the one a person has, and what stands
// where main stands is `sessions-fleet.ts`, which holds the sessions and publishes as main does. The
// walks (`walks/sessions.ts`) prove what the screens draw from fixtures; this proves the same screens
// are drawn from, and act on, what Fleet serves.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { asking, FakeSessionsFleet, held, hosted, pullRequest, terminal } from "./sessions-fleet";
import { mount, onScreen, toPageByName, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const ID = "01SESSIONAAAAAAAAAAAAAAAAA";

/** The base repository, over a Fleet that serves these sessions. */
function served(fleet: FakeSessionsFleet): Scenario {
  return fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root }));
}

const work = () => page.getByRole("region", { name: "Jobs", exact: true });

test("Sessions wired: a session Fleet holds is listed by its title, and a terminal session beside it", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }),
    terminal("01TERMINALBBBBBBBBBBBBBBBB", { title: "Release notes script" }),
  ]);
  mount(served(fleet));
  await onScreen();
  await expect.element(work().getByRole("button", { name: "Fix the flaky store test" })).toBeVisible();
  await expect.element(work().getByRole("button", { name: "Release notes script" })).toBeVisible();
});

const toast = (headline: string) => page.getByText(headline, { exact: true }).first();

test("Sessions wired: a send Fleet refuses is a toast that stays, and nothing is said in the thread", async () => {
  const fleet = new FakeSessionsFleet([terminal(ID, { title: "Release notes script" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Release notes script");
  fleet.refusesSend = { code: "fleet.terminal_session_unreachable", message: "that session is not listening." };
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "Ship it");
  await userEvent.click(page.getByRole("button", { name: "Send" }));
  await expect.element(toast("Not reachable: run /reload-plugins in that job")).toBeVisible();
  await expect.element(page.getByText("that session is not listening.")).not.toBeInTheDocument();

  fleet.refusesSend = { code: "fleet.session_closed", message: "That Session is closed." };
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "Ship it again");
  // The first toast stands over Send, so the second send is Enter in the box.
  await userEvent.keyboard("{Enter}");
  await expect.element(toast("That Session is closed.")).toBeVisible();
  await expect.element(toast("Not reachable: run /reload-plugins in that job")).toBeVisible();
});

test("Sessions wired: a terminal Session offers End job, and it ends once its terminal does", async () => {
  const fleet = new FakeSessionsFleet([terminal(ID, { title: "Release notes script" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Release notes script");
  const panel = page.getByRole("region", { name: /^Job / });
  // The acts are the header's menu, opened by its More button.
  await userEvent.click(panel.getByRole("button", { name: "More" }));
  await userEvent.click(page.getByRole("menuitem", { name: /^End job/ }));
  await expect.poll(() => fleet.calls.closed).toEqual([ID]);
  // Held for its mod: the Session stays live, and offers no Fork, until the terminal says it ended.
  await userEvent.click(panel.getByRole("button", { name: "More" }));
  await expect.element(page.getByRole("menuitem", { name: "Fork" })).not.toBeInTheDocument();
  await userEvent.keyboard("{Escape}");
  fleet.changed(terminal(ID, { title: "Release notes script", state: "ended" }));
  await userEvent.click(panel.getByRole("button", { name: "More" }));
  await expect.element(page.getByRole("menuitem", { name: "Fork" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: /^End job/ })).not.toBeInTheDocument();
});

test("Sessions wired: End job on a terminal Fleet cannot reach is said, and nothing ends", async () => {
  const fleet = new FakeSessionsFleet([terminal(ID, { title: "Docs pass" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Docs pass");
  fleet.refusesClose = { code: "fleet.terminal_session_unreachable", message: "that session is not listening." };
  const panel = page.getByRole("region", { name: /^Job / });
  await userEvent.click(panel.getByRole("button", { name: "More" }));
  await userEvent.click(page.getByRole("menuitem", { name: /^End job/ }));
  await expect.element(toast("Not reachable: run /reload-plugins in that job")).toBeVisible();
  expect(fleet.calls.closed).toEqual([]);
  await userEvent.click(panel.getByRole("button", { name: "More" }));
  await expect.element(page.getByRole("menuitem", { name: /^End job/ })).toBeVisible();
});

/** A key on Flight control, which opens a layer over the open Session's thread. */
const key = (name: string) => page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name });
const layer = (name: string) => page.getByRole("region", { name, exact: true });

test("Sessions wired: rows stream into an open thread, and the first write leases a slot that is drawn only when it is wrong", async () => {
  const fleet = new FakeSessionsFleet([hosted(ID, { title: "Fix the flaky store test" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await expect.poll(() => [...new Set(fleet.calls.watched)]).toEqual([ID]);

  fleet.says(ID, "The test reads the wall clock.");
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("The test reads the wall clock.")).toBeVisible();

  fleet.row(ID, { kind: "lease", id: "lease1", at: "2026-10-07T13:48:05.000Z", slot: 3, branch: "fix/flaky-store" });
  fleet.changed(hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }));
  await expect.element(page.getByRole("region", { name: "Slot leased" })).toBeVisible();
  // The branch and the bay are drawn only when something is wrong with them: the Session has no Ledger, and the Bays key is not lit.
  await expect.element(page.getByRole("region", { name: "Ledger" })).not.toBeInTheDocument();
  expect(key("Bays").element().hasAttribute("data-lit")).toBe(false);
});

test("Sessions wired: the ask offers what Fleet will take, and the press names the offer", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", hosted: { turn: { state: "working" }, mode: "auto", running: true, asked: asking("git push origin fix/flaky-store") } }),
  ]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  // The ask is the only thing waiting, so the panel opens on its card by itself.
  const card = page.getByRole("article", { name: "Waiting on you" });
  await expect.element(card.getByText("git push origin fix/flaky-store")).toBeVisible();
  await expect.element(card.getByRole("button", { name: "Allow once" })).toBeVisible();
  await expect.element(card.getByRole("button", { name: "Refuse" })).toBeVisible();
  await userEvent.click(card.getByRole("button", { name: "Allow and remember" }));
  await expect.poll(() => fleet.calls.answered).toEqual([{ session_id: ID, call: "call-1", answer: "allow_and_remember" }]);
  await expect.element(card).not.toBeInTheDocument();
});

test("Sessions wired: mode, model and effort go to Fleet as the person set them", async () => {
  const fleet = new FakeSessionsFleet([hosted(ID, { title: "Fix the flaky store test" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Permission mode" }), "plan");
  await expect.poll(() => fleet.calls.tuned.at(-1)).toEqual({ session_id: ID, mode: "plan" });
  await userEvent.selectOptions(page.getByRole("combobox", { name: "Effort" }), "high");
  await expect.poll(() => fleet.calls.tuned.at(-1)).toEqual({ session_id: ID, effort: "high", mode: "plan" });
});

test("Sessions wired: a terminal session opens to its Blueprint and takes a message, with its mode shown and not set", async () => {
  const fleet = new FakeSessionsFleet([terminal(ID, { title: "Release notes script", attachments: [held("branch", "release/notes")], terminal: { mode: "plan", listening: true } })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Release notes script");
  await expect.element(page.getByRole("region", { name: "Blueprint", exact: true })).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  await expect.element(page.getByRole("combobox", { name: "Permission mode" })).toBeDisabled();
});

test("Sessions wired: a terminal session draws no mode until its mod has reported one", async () => {
  const fleet = new FakeSessionsFleet([terminal(ID, { title: "Release notes script" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Release notes script");
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  await expect.element(page.getByRole("combobox", { name: "Permission mode" })).not.toBeInTheDocument();
});

test("Sessions wired: search finds a session by the pull request it holds, across every session", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [pullRequest(1843)] }),
    hosted("01SESSIONCCCCCCCCCCCCCCCCCC", { title: "Store migration spike", attachments: [pullRequest(1849, { state: "draft" })] }),
  ]);
  mount(served(fleet));
  await onScreen();
  await userEvent.click(work().getByRole("button", { name: /^Filter/ }));
  await userEvent.fill(page.getByRole("searchbox", { name: "Search Jobs" }), "#1849");
  await expect.element(work().getByRole("button", { name: "Store migration spike" })).toBeVisible();
  await expect.element(work().getByRole("button", { name: "Fix the flaky store test" })).not.toBeInTheDocument();
});

test("Sessions wired: a pull request the Session holds is merged from its sheet, and Fleet's refusal is said when it will not", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [pullRequest(1847), pullRequest(1843, { checks: "failed", failing: "lint" })] }),
  ]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await key("Artifacts").click();
  await userEvent.click(layer("Artifacts").getByRole("button", { name: /^#1847/ }));
  const sheet = page.getByRole("dialog", { name: "Pull request #1847" });
  await userEvent.click(sheet.getByRole("button", { name: "Merge" }));
  await expect.poll(() => fleet.calls.pressed.filter((one) => one.press === "merge")).toEqual([{ sessionId: ID, number: 1847, press: "merge" }]);
  await expect.element(sheet.getByText("Merged")).toBeVisible();
});

test("Sessions wired: a refused press on a pull request says Fleet's words in its sheet and reads the pull request again", async () => {
  const fleet = new FakeSessionsFleet([hosted(ID, { title: "Fix the flaky store test", attachments: [pullRequest(1847, { checks: "pending" })] })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await key("Artifacts").click();
  await userEvent.click(layer("Artifacts").getByRole("button", { name: /^#1847/ }));
  const sheet = page.getByRole("dialog", { name: "Pull request #1847" });
  fleet.refusesPress = { code: "fleet.auto_merge_refused", message: "Auto merge is not allowed" };
  await userEvent.click(sheet.getByRole("button", { name: "Enable auto-merge" }));
  await expect.element(sheet.getByText("Auto merge is not allowed")).toBeVisible();
  await expect.poll(() => fleet.calls.pressed.filter((one) => one.press === "read").length).toBeGreaterThan(1);
});

test("Sessions wired: red checks on a merged pull request do not call the Session waiting on you", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), pullRequest(1843, { state: "merged", checks: "failed", failing: "lint" })] }),
  ]);
  mount(served(fleet));
  await onScreen();
  await expect.element(work().getByRole("button", { name: "Fix the flaky store test" })).toBeVisible();
  await expect.element(work().getByRole("img", { name: /^Waiting on you/ })).not.toBeInTheDocument();
});

test("Sessions wired: red checks on an open pull request leave the Session idle, not waiting on you", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), pullRequest(1843, { checks: "failed", failing: "lint" })] }),
  ]);
  mount(served(fleet));
  await onScreen();
  await expect.element(work().getByRole("button", { name: "Fix the flaky store test" })).toBeVisible();
  await expect.element(work().getByRole("img", { name: /^Waiting on you/ })).not.toBeInTheDocument();
});

test("Sessions wired: a chip on Cleanup's tile names the session that holds its branch, across repositories, and the card opens it", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store"), pullRequest(1843)] }),
  ]);
  mount({
    ...served(fleet),
    held: {
      slots: [
        {
          manifest_id: "armada",
          slot: 3,
          path: "/Users/user/armada/.armada/slots/slot-3",
          base: "main",
          warm: true,
          behind: 0,
          since: new Date().toISOString(),
          branch: "fix/flaky-store",
          held: { state: "session", holder: "Session s-01SESSIO" },
        },
      ],
    },
  });
  await onScreen();
  await toPageByName("Worktree Slots");
  await userEvent.click(page.getByRole("button", { name: "Branch fix/flaky-store" }));
  const card = page.getByRole("dialog", { name: "Owned by Fix the flaky store test" });
  await expect.element(card).toBeVisible();
  await userEvent.click(card.getByRole("button", { name: "Open Job" }));
  await expect.element(page.getByRole("region", { name: /^Job s-01SESSIO/ })).toBeVisible();
});

test("Sessions wired: a picked file goes as base64 and a drawn sketch goes as the picture it renders to", async () => {
  const fleet = new FakeSessionsFleet([hosted(ID, { title: "Fix the flaky store test" })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await userEvent.upload(page.getByLabelText("Files to attach"), new File(["hello"], "notes.txt", { type: "text/plain" }));
  await userEvent.click(page.getByRole("button", { name: "Draw sketch" }));
  const pad = page.getByRole("dialog", { name: "Draw a sketch" });
  await userEvent.click(pad.getByRole("button", { name: "Add a box" }));
  await userEvent.fill(page.getByRole("textbox", { name: "The words in this box" }), "pin the clock");
  await userEvent.click(pad.getByRole("button", { name: "Attach sketch" }));
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "See both");
  await userEvent.click(page.getByRole("button", { name: "Send" }));
  await expect.poll(() => fleet.calls.sent.length).toBe(1);
  const [file, sketch] = fleet.calls.sent[0]!.attachments!;
  expect(file).toEqual({ name: "notes.txt", media_type: "text/plain", data: btoa("hello") });
  expect(sketch).toMatchObject({ name: "pin the clock.png", media_type: "image/png" });
  expect(atob(sketch!.data).startsWith("\x89PNG")).toBe(true);
  // What was drawn stays in Artifacts, since the wire holds only the picture.
  await key("Artifacts").click();
  await expect.element(layer("Artifacts").getByRole("button", { name: "pin the clock" })).toBeVisible();
});

const agent = (id: string, text: string) => ({ kind: "message" as const, id, at: "2026-10-07T13:49:00.000Z", from: { kind: "agent" as const }, text });
const call = (id: string, text: string) => ({ kind: "tool" as const, id, at: "2026-10-07T13:49:00.000Z", text });

test("Sessions wired: a running subagent opens as its own thread, gains rows as Fleet reads it again, and a finished one ends on its report", async () => {
  const fleet = new FakeSessionsFleet([
    terminal(ID, {
      title: "Read the CI history",
      attachments: [
        held("subagent", "live1", { description: "Read the CI history" }),
        held("subagent", "done1", { description: "Find clocks" }, "spent"),
      ],
    }),
  ]);
  fleet.subagents["live1"] = { rows: [agent("a1", "Reading the runs."), call("a2", "gh run list --limit 30")], finished: false };
  fleet.subagents["done1"] = { rows: [call("b1", "Grep Instant::now"), agent("b2", "None reads the clock.")], finished: true, report: "None reads the clock." };
  mount(served(fleet));
  await onScreen();
  await toSession("Read the CI history");
  await key("Subagents").click();
  await userEvent.click(layer("Subagents").getByRole("button", { name: "Read the CI history" }));
  const panel = page.getByRole("dialog", { name: "Subagent Read the CI history" });
  await expect.element(panel.getByText("Reading the runs.")).toBeVisible();
  // The calls are one folded group, as in the Session's own thread.
  await userEvent.click(panel.getByText("gh", { exact: true }));
  await expect.element(panel.getByText("gh run list --limit 30")).toBeVisible();

  // The transcript moves on: the next read the sheet makes draws the new call and the report.
  fleet.subagents["live1"] = {
    rows: [...fleet.subagents["live1"]!.rows, call("a3", "Read crates/store/tests/flaky.rs"), agent("a4", "Four of thirty failed.")],
    finished: true,
    report: "Four of thirty failed.",
  };
  await expect.element(panel.getByText("Four of thirty failed.")).toBeVisible();
  await expect.element(panel.getByText("gh, Read", { exact: true })).toBeVisible();

  await userEvent.click(panel.getByRole("button", { name: /^Close/ }));
  await userEvent.click(layer("Subagents").getByRole("button", { name: "Find clocks" }));
  const done = page.getByRole("dialog", { name: "Subagent Find clocks" });
  await expect.element(done.getByText("Grep", { exact: true })).toBeVisible();
  await expect.element(done.getByText("None reads the clock.")).toBeVisible();
});

test("Sessions wired: a subagent's sheet is wide and states its type, its model and its context in the head", async () => {
  const fleet = new FakeSessionsFleet([
    terminal(ID, {
      title: "Read the CI history",
      attachments: [held("subagent", "live1", { description: "Read the CI history", type: "rust-engineer" })],
    }),
  ]);
  fleet.subagents["live1"] = {
    rows: [agent("a1", "Reading the runs.")],
    finished: false,
    model: "vendor-opus-5-5",
    context_tokens: 142_000,
    context_window: 200_000,
  };
  mount(served(fleet));
  await onScreen();
  await toSession("Read the CI history");
  await key("Subagents").click();
  await userEvent.click(layer("Subagents").getByRole("button", { name: "Read the CI history" }));
  const panel = page.getByRole("dialog", { name: "Subagent Read the CI history" });
  await expect.element(panel.getByText("rust-engineer")).toBeVisible();
  await expect.element(panel.getByText("Opus 5.5")).toBeVisible();
  await expect.element(panel.getByRole("img", { name: "Context 71%" })).toBeVisible();
  expect(panel.element().getAttribute("data-size")).toBe("wide");
});

test("Sessions wired: a page in Artifacts is shown in main's view over the layer and taken away when the layer closes", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(ID, { title: "Write up the clock", attachments: [held("artifact", "https://example.com/artifact/findings", { form: "page", title: "Clock findings" })] }),
  ]);
  mount(served(fleet));
  await onScreen();
  await toSession("Write up the clock");
  await key("Artifacts").click();
  await userEvent.click(layer("Artifacts").getByRole("button", { name: "Clock findings" }));
  await expect.poll(() => fleet.calls.pages).toEqual(["https://example.com/artifact/findings"]);
  // Main's view lies over the body, so nothing is framed here.
  await expect.element(page.getByTitle("Clock findings")).not.toBeInTheDocument();
  // Esc pressed in main's view, which has the keyboard, closes the panel.
  fleet.pageEscape();
  await expect.poll(() => fleet.calls.pagesHidden).toBeGreaterThan(0);
});
