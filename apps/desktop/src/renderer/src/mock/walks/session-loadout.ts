// A Session's Loadout: the mark in its head, its card counting what the agent knows and was told and listing what the
// latest turn used, and the panel a press opens over the work area, where a Check opens, the latest turn open in it.
// The turn that sent `armada:mechanic` on sonnet is opened in its place. A row pressed opens what that item says to
// the Loadout's left: a repository skill, then the session rules in its place, then an MCP server's tools, and Esc
// closes it with the Loadout still open. Then typing /mcp opens the Loadout on its MCP servers, read live from the
// Job's process: chrome-devtools timed out and is Reconnected, claude.ai Gmail signs in and then offers Reconnect, and
// forge is turned off and on. Over the `session-loadout` scenario.

import { timeSaid } from "@armada/components";

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Job s-a3f7c2d9");
const sessions = region("Jobs", { exact: true });
const mark = inside(head, button("Loadout", { exact: true }));
const sheet = dialog("Loadout");
const knows = inside(sheet, region("Knows"));
const told = inside(sheet, region("Told"));
const servers = inside(sheet, region("MCP servers"));
const act = (name: string) => inside(servers, button(name, { exact: true }));
const server = (name: RegExp) => inside(servers, role("button", name));
/** The scenario's third turn, fourteen minutes after it began at 13:40Z. */
const mechanicAt = timeSaid("2026-10-17T13:54:00.000Z");

const sessionLoadout = walk(
  "session-loadout",
  [
    toSessions,
    { press: inside(sessions, role("button", /Why did the mechanic skip/)), say: "The Job" },
    { look: mark, say: "The Loadout's mark, after the context" },
    { hover: mark, say: "What it knows and was told, and what the latest turn used" },
    { press: mark, say: "The whole Loadout, the latest turn open" },
    { look: servers, say: "Each server as the Job's process reports it, with the one act its state takes" },
    { press: inside(sheet, button(`Turn at ${mechanicAt}`)), say: "The turn that sent armada:mechanic on sonnet" },
    { look: inside(sheet, role("list", `Used in the turn at ${mechanicAt}`)), say: "The subagent, the forge tool it called and the repository skill" },
    { press: inside(inside(knows, region("Skills")), role("button", /^work-issue/)), say: "The repository skill work-issue, to the left" },
    { look: inside(dialog("work-issue"), text(".harness/skills/work-issue/SKILL.md")), say: "Its SKILL.md, read from the repository" },
    { press: inside(told, role("button", /^Armada job rules/)), say: "The job rules, in its place" },
    { look: dialog("Armada job rules"), say: "The rules Armada gives every Job, as text" },
    { press: inside(inside(knows, region("MCP servers")), role("button", /^forge/)), say: "The forge MCP server" },
    { look: inside(dialog("forge"), region("Tools")), say: "Its tools from the Loadout, each with its state" },
    { key: "Escape", on: dialog("forge"), say: "Esc closes the server's panel" },
    { look: sheet, say: "The Loadout stays open" },
    { key: "Escape", on: sheet, say: "Esc closes the Loadout" },
    { type: "/mcp\n", into: role("textbox", "Message"), say: "/mcp in a hosted Job, picked from what / offers" },
    { key: "Enter", on: role("textbox", "Message"), say: "and sent" },
    { look: servers, say: "opens the Loadout on its MCP servers" },
    { hover: inside(server(/^chrome-devtools/), role("img", "Failed")), say: "chrome-devtools failed: timed out after 30s" },
    { press: act("Reconnect chrome-devtools"), say: "Reconnect it" },
    { look: server(/^chrome-devtools 26 tools$/), say: "Connected, with its 26 tools and no mark" },
    { press: act("Sign in claude.ai Gmail"), say: "claude.ai Gmail needs sign-in: its page opens in the browser" },
    { look: act("Reconnect claude.ai Gmail"), say: "A claude.ai page sends nothing back, so the row offers Reconnect" },
    { press: act("Turn off forge"), say: "Turn forge off" },
    { look: inside(server(/^forge/), role("img", "Off")), say: "Off, and Turn on beside it" },
    { press: act("Turn on forge"), say: "Turn it on again" },
    { look: server(/^forge 2 tools$/), say: "Connected, with its 2 tools" },
  ],
  { width: 1512, height: 949 },
);

export { sessionLoadout as "session-loadout" };
