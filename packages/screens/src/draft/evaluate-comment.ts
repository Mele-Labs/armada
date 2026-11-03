// The message Evaluate sends a Job about one comment on its pull request. The Job answers in its
// own waiting list and never on the forge. The decision record `2026-11-03-pull-request-comments-reach-the-job.md`.

import type { EvaluatedComment } from "@armada/components/src/compositions/PullRequestPanel/evaluated-comment";

/** A comment Evaluate hands to the Job that holds its pull request. */
export type CommentToEvaluate = EvaluatedComment & {
  /** The pull request's number. */
  pullRequest: number;
};

/** The body as a Markdown quote, a blank line kept as a bare `>`. */
const quoted = (body: string): string =>
  body
    .trim()
    .split("\n")
    .map((line) => (line.trim() === "" ? ">" : `> ${line}`))
    .join("\n");

/** What the Job reads. `where` with a line number is a line, without one a file. */
export function evaluateMessage(comment: CommentToEvaluate): string {
  const where =
    comment.where === undefined
      ? []
      : [
          `${/:\d+$/.test(comment.where) ? "Line" : "File"}: ${comment.where}`,
        ];
  return [
    `Evaluate a comment on #${comment.pullRequest}.`,
    `Comment: ${comment.id}`,
    `Author: ${comment.author}`,
    ...where,
    "",
    quoted(comment.body),
    "",
    "Decide whether the comment is right, and put what you propose in waiting_for as one item for me.",
    "Never reply on the pull request, and never post a comment, review or reaction anywhere.",
  ].join("\n");
}
