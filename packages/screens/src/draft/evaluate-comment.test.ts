import { expect, test } from "vitest";

import { evaluateMessage } from "./evaluate-comment";

test("a line comment names the pull request, the comment, its author and its line, and quotes the body", () => {
  expect(
    evaluateMessage({
      pullRequest: 20481,
      id: "PRRC_20481_1",
      author: "Maya",
      where: "src/cart/totals.ts:42",
      body: "This rounds before the discount.\n\nA total can be a cent off.",
    }),
  ).toBe(
    [
      "Evaluate a comment on #20481.",
      "Comment: PRRC_20481_1",
      "Author: Maya",
      "Line: src/cart/totals.ts:42",
      "",
      "> This rounds before the discount.",
      ">",
      "> A total can be a cent off.",
      "",
      "Decide whether the comment is right, and put what you propose in waiting_for as one item for me.",
      "Never reply on the pull request, and never post a comment, review or reaction anywhere.",
    ].join("\n"),
  );
});

test("a file comment names the file, and a conversation comment names no place", () => {
  const onFile = evaluateMessage({
    pullRequest: 20481,
    id: "PRRC_20481_3",
    author: "Jun",
    where: "README.md",
    body: "Say what the balance endpoint returns.",
  });
  expect(onFile).toContain("\nFile: README.md\n");
  const onConversation = evaluateMessage({
    pullRequest: 20481,
    id: "IC_20481_1",
    author: "Jun",
    body: "Looks good.",
  });
  expect(onConversation).not.toMatch(/^(Line|File):/m);
  expect(onConversation).toContain("Author: Jun\n\n> Looks good.");
});

test("the message carries no dash", () => {
  expect(
    evaluateMessage({
      pullRequest: 1,
      id: "c",
      author: "a",
      body: "b",
    }),
  ).not.toMatch(/[—–]| - /);
});
