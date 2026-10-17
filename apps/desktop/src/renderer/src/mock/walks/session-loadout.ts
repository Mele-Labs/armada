// A Session's Loadout: the mark in its head, its card counting what the agent knows and was told and listing what the
// latest turn used, and the panel a press opens over the work area, where a Check opens, the latest turn open in it.
// The turn that sent `armada:mechanic` on sonnet is opened in its place. Over the `session-loadout` scenario.

import { turnSaid } from "@armada/components";

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Session s-a3f7c2d9");
const sessions = region("Work", { exact: true });
const mark = inside(head, button("Loadout", { exact: true }));
const sheet = dialog("Loadout");
/** The scenario's third turn, fourteen minutes after it began at 13:40Z. */
const mechanicAt = turnSaid("2026-10-17T13:54:00.000Z");

const sessionLoadout = walk(
  "session-loadout",
  [
    toSessions,
    { press: inside(sessions, role("button", /Why did the mechanic skip/)), say: "The Session" },
    { look: mark, say: "The Loadout's mark, after the context" },
    { hover: mark, say: "What it knows and was told, and what the latest turn used" },
    { press: mark, say: "The whole Loadout, the latest turn open" },
    { look: inside(sheet, region("MCP servers")), say: "gitnexus failed to connect" },
    { press: inside(sheet, button(`Turn at ${mechanicAt}`)), say: "The turn that sent armada:mechanic on sonnet" },
    { look: inside(sheet, role("list", `Used in the turn at ${mechanicAt}`)), say: "The subagent, the GitHub tool it called and the repository skill" },
  ],
  { width: 1512, height: 949 },
);

export { sessionLoadout as "session-loadout" };
