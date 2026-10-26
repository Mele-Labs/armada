import { expect, test } from "vitest";

import { commandIn } from "./fix-command";

test("the first backticked span is the command", () => {
  expect(commandIn("Sign in again with `gh auth refresh -s repo`, then open it again.")).toBe("gh auth refresh -s repo");
  expect(commandIn("Run `gh auth login` or `gh auth refresh`.")).toBe("gh auth login");
});

test("a sentence with no span, an empty one or one across lines names no command", () => {
  expect(commandIn("GitHub refused access. Open it again later.")).toBeUndefined();
  expect(commandIn("Nothing between `` the ticks.")).toBeUndefined();
  expect(commandIn("An open `tick and no close")).toBeUndefined();
  expect(commandIn("Across `two\nlines` is prose.")).toBeUndefined();
});
