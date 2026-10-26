// A pull request's or an issue's link, pasted or dropped anywhere in Bridge, goes to the page it names: Reviews for a pull
// request, the side panel for an issue. They claim ahead of `link-drop.tsx`, which keeps every other link. **A paste into a text
// field is not theirs**: the field keeps it, and the link panel's offer stands beside it.

import type { RepositorySummary } from "@armada/protocol";

import { classifyLink, type LinkClass } from "../../shared/classify-link";
import type { PullLink } from "../../shared/pull-link";
import { fieldOf } from "./link-drop";
import { manifestOfLink } from "./pr-review/link";
import { useClaim } from "./paste-and-drop";
import { openInPanel } from "./shared-panel/store";

/** The first link a drop carries, as `link-drop.tsx` reads it. */
const droppedLink = (event: DragEvent): string | undefined => event.dataTransfer?.getData("text/uri-list").split("\n").find((one) => !one.startsWith("#")) ?? event.dataTransfer?.getData("text/plain");

function openIssue(one: Extract<LinkClass, { kind: "issue" }>, repositories: readonly RepositorySummary[]): boolean {
  const manifestId = one.owner === undefined || one.name === undefined ? undefined : manifestOfLink(repositories, [], { owner: one.owner, name: one.name, number: 0 });
  const root = manifestId === undefined ? undefined : repositories.find((repository) => repository.manifest?.id === manifestId)?.root;
  return openInPanel({ kind: "issue", number: one.number, address: one.address, ...(root === undefined ? {} : { root }), ...(manifestId === undefined ? {} : { manifestId }) });
}

/** Mounted once, by `App`. `openPull` takes the window to Reviews on a pull request (`usePullLinks`). */
export function useLinkRoutes(openPull: (link: PullLink) => void, repositories: readonly RepositorySummary[]): void {
  const take = (kind: "pull" | "issue") => (event: Event, text: string | undefined): boolean => {
    const one = classifyLink(text ?? "");
    if (one.kind !== kind) return false;
    // An issue with no side panel hosted is left for the link panel, as any web address is.
    if (one.kind === "issue" && !openIssue(one, repositories)) return false;
    if (one.kind === "pull") openPull(one.link);
    event.preventDefault();
    return true;
  };
  for (const kind of ["pull", "issue"] as const) {
    const claim = take(kind);
    // eslint-disable-next-line react-hooks/rules-of-hooks -- a fixed pair, in a fixed order
    useClaim(kind, {
      paste: (event) => fieldOf(event.target) === null && claim(event, event.clipboardData?.getData("text/plain")),
      drop: (event) => fieldOf(event.target) === null && claim(event, droppedLink(event)),
    });
  }
}
