// `retro/lessons` with one more Kit item, a command the agent was refused: its button reads Update
// Kit, and the command lands on the Kit page's allowlist as a retro item's.

import type { Lesson } from "@armada/protocol";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const SESSION = { id: "01M2SESSIONRETRO3WRITTEN", title: "Retire guides 8 and 20, add validation that" };

const GREP: Lesson = {
  id: "01M2LESSONGREPALLOW",
  handle: SESSION.id,
  session: SESSION,
  at: "2026-10-02T22:05:00.000Z",
  who: "agent",
  lands_in: "kit",
  title: "grep had to be allowed on a check log",
  what: "The agent asked to run grep on a check log and waited for you to allow it.",
  fix: "Add grep to the allowlist.",
  statement: "The agent asked to run grep on a check log and waited for you to allow it.",
  evidence: [],
  state: "open",
  change: { kind: "allow_command", command: "grep" },
};

function withCommand(): Scenario {
  const { retros, lessons } = retroFixtures();
  return { ...holding("retro/kit-command", "Retros with a Kit item that carries a command to allow"), retros, lessons: [GREP, ...lessons] };
}

export const retroKitCommand: Scenario = withCommand();
