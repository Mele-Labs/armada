import { Box, GitBranch, GitPullRequest, SquareTerminal, Ticket } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a message tags: **`@` another Session or a Job**, **`#` an issue or a pull request by its
 * number**, whose `id` is the number and whose `title` leads with it, `#2214 The title`. A branch is
 * still a kind a message can carry, though neither trigger offers one.
 */
export type ComposerTag = {
  kind: "session" | "job" | "pull_request" | "branch" | "issue";
  id: string;
  title: string;
  /** The forge's address of an issue or a pull request, sent with it. */
  url?: string;
};

export const TAG_GLYPH: Record<ComposerTag["kind"], LucideIcon> = {
  session: SquareTerminal,
  job: Box,
  pull_request: GitPullRequest,
  branch: GitBranch,
  issue: Ticket,
};

/**
 * A tag drawn in the line it was written in: its kind's glyph and its title, in one unit. In the
 * message box it is atomic, so Backspace takes it whole; in the thread it is the same unit, read-only.
 * **With `onOpen` it is a link** to where Bridge shows what it names, with its kind and title on hover.
 */
export function InlineTag({ tag, atomic = false, onOpen }: { tag: ComposerTag; atomic?: boolean; onOpen?: () => void }) {
  const Glyph = TAG_GLYPH[tag.kind];
  if (onOpen !== undefined) {
    return (
      <Tooltip label={`${TAG_KIND_SAID[tag.kind]} · ${tag.title.replace(/^#\d+\s*/, "")}`} asChild>
        <a
          className="armada-inline-tag armada-inline-tag--link"
          href={`#${tag.kind}`}
          onClick={(event) => {
            event.preventDefault();
            onOpen();
          }}
        >
          <Glyph size={12} strokeWidth={2} aria-hidden />
          {tag.title}
        </a>
      </Tooltip>
    );
  }
  return (
    <span
      className="armada-inline-tag"
      {...(atomic ? { contentEditable: false, "data-tag-kind": tag.kind, "data-tag-id": tag.id, "data-tag-title": tag.title, ...(tag.url === undefined ? {} : { "data-tag-url": tag.url }) } : {})}
    >
      <Glyph size={12} strokeWidth={2} aria-hidden />
      {tag.title}
    </span>
  );
}

const TAG_KIND_SAID: Record<ComposerTag["kind"], string> = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch", issue: "Issue" };
