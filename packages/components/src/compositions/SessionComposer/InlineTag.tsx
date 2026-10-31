import { Bug, GitBranch, GitPullRequest, SquareTerminal, Ticket } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a message tags: **`@` another Session**, **`#` an issue or a pull request by its
 * number**, whose `id` is the number and whose `title` leads with it, `#2214 The title`. A branch is
 * still a kind a message can carry, though neither trigger offers one. **A reported bug** rides the Job started
 * on it: its `id` is the report's, its `title` the report's.
 */
export type ComposerTag = {
  kind: "session" | "pull_request" | "branch" | "issue" | "report";
  id: string;
  title: string;
  /** The forge's address of an issue or a pull request, sent with it. */
  url?: string;
};

/**
 * The title without the reference it leads with: `#2214 ` on the forge, and a key such as `ABC-12 ` where
 * the issue's tag id is that key.
 */
export function titleWithoutReference(tag: Pick<ComposerTag, "kind" | "id" | "title">): string {
  const forge = tag.title.replace(/^#\d+\s*/, "");
  if (forge !== tag.title || tag.kind !== "issue") return forge;
  return tag.title.toLowerCase().startsWith(tag.id.toLowerCase()) ? tag.title.slice(tag.id.length).trimStart() : tag.title;
}

export const TAG_GLYPH: Record<ComposerTag["kind"], LucideIcon> = {
  session: SquareTerminal,
  pull_request: GitPullRequest,
  branch: GitBranch,
  issue: Ticket,
  report: Bug,
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
      <Tooltip label={`${TAG_KIND_SAID[tag.kind]} · ${titleWithoutReference(tag)}`} asChild>
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

const TAG_KIND_SAID: Record<ComposerTag["kind"], string> = { session: "Job", pull_request: "Pull request", branch: "Branch", issue: "Issue", report: "Reported bug" };
