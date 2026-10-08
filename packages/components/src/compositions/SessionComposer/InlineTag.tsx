import { Box, GitBranch, GitPullRequest, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

/** What `@` tags: another Session, a Job, a pull request or a branch. */
export type ComposerTag = { kind: "session" | "job" | "pull_request" | "branch"; id: string; title: string };

export const TAG_GLYPH: Record<ComposerTag["kind"], LucideIcon> = {
  session: SquareTerminal,
  job: Box,
  pull_request: GitPullRequest,
  branch: GitBranch,
};

/**
 * A tag drawn in the line it was written in: its kind's glyph and its title, in one unit. In the
 * message box it is atomic, so Backspace takes it whole; in the thread it is the same unit, read-only.
 */
export function InlineTag({ tag, atomic = false }: { tag: ComposerTag; atomic?: boolean }) {
  const Glyph = TAG_GLYPH[tag.kind];
  return (
    <span
      className="armada-inline-tag"
      {...(atomic ? { contentEditable: false, "data-tag-kind": tag.kind, "data-tag-id": tag.id, "data-tag-title": tag.title } : {})}
    >
      <Glyph size={12} strokeWidth={2} aria-hidden />
      {tag.title}
    </span>
  );
}
