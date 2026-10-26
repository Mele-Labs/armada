// A pull request's or an issue's link, pasted or dropped anywhere in Bridge, goes to the page it names: Reviews for a pull
// request, the side panel for an issue. They claim ahead of `link-drop.tsx`, which keeps every other link. **A paste into a text
// field is not theirs**: the field keeps it, and the link panel's offer stands beside it.

import { useEffect, useMemo, useState } from "react";
import type { RepositorySummary } from "@armada/protocol";

import { classifyLink, type LinkClass } from "../../shared/classify-link";
import type { PullLink } from "../../shared/pull-link";
import { fieldOf } from "./link-drop";
import { manifestOfLink } from "./pr-review/link";
import { useClaim } from "./paste-and-drop";
import { openInPanel } from "./shared-panel/store";

/** The first link a drop carries, as `link-drop.tsx` reads it. */
const droppedLink = (event: DragEvent): string | undefined => event.dataTransfer?.getData("text/uri-list").split("\n").find((one) => !one.startsWith("#")) ?? event.dataTransfer?.getData("text/plain");

/**
 * The hosts of the trackers Fleet serves a Jira-keyed issue from, read off the addresses of the issues it lists: the Manifest's
 * `site` is Fleet's, and a list is how Bridge hears it. A `/browse/KEY` address on any other host is no tracker's.
 */
export function useTrackerHosts(repositories: readonly RepositorySummary[]): readonly string[] {
  const manifests = repositories.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest.id])).join();
  const [hosts, setHosts] = useState<readonly string[]>([]);
  useEffect(() => {
    if (manifests === "" || typeof window.armada?.listIssues !== "function") return;
    let live = true;
    void Promise.all(
      manifests.split(",").map((id) =>
        window.armada
          .listIssues(id, false)
          .then((read) => (read.ok ? read.value.issues.filter((one) => !one.reference.startsWith("#")).flatMap((one) => hostOf(one.url)) : []))
          .catch(() => []),
      ),
    ).then((found) => live && setHosts([...new Set(found.flat())]));
    return () => {
      live = false;
    };
  }, [manifests]);
  return hosts;
}

const hostOf = (address: string): string[] => {
  try {
    return [new URL(address).hostname.toLowerCase()];
  } catch {
    return [];
  }
};

function openIssue(one: Extract<LinkClass, { kind: "issue" }>, repositories: readonly RepositorySummary[]): boolean {
  const manifestId = one.owner === undefined || one.name === undefined ? undefined : manifestOfLink(repositories, [], { owner: one.owner, name: one.name, number: 0 });
  const root = manifestId === undefined ? undefined : repositories.find((repository) => repository.manifest?.id === manifestId)?.root;
  return openInPanel({ kind: "issue", number: one.number, address: one.address, ...(root === undefined ? {} : { root }), ...(manifestId === undefined ? {} : { manifestId }) });
}

/** What the window knows of where links go: the pull request opener, the repositories, and the tracker hosts. */
export type LinkWorld = { openPull: (link: PullLink) => void; repositories: readonly RepositorySummary[]; trackerHosts: readonly string[] };

/**
 * Opens the page a link names, where it names a `kind` of one, and answers whether it did. An issue with no side panel hosted
 * is not opened, and is left for the link panel as any web address is. `openPull` is `usePullLinks`'s, which takes the window to Reviews.
 */
export function routeLink(text: string | undefined, kind: "pull" | "issue" | "any", world: LinkWorld): boolean {
  const one = classifyLink(text ?? "", world.trackerHosts);
  if (one.kind === "other" || (kind !== "any" && one.kind !== kind)) return false;
  if (one.kind === "issue") return openIssue(one, world.repositories);
  world.openPull(one.link);
  return true;
}

/** Mounted once, by `App`. Answers what the palette's row needs: whether a typed link names a pull request or an issue, and opening it. */
export function useLinkRoutes(openPull: (link: PullLink) => void, repositories: readonly RepositorySummary[]): { kindOf: (typed: string) => "pull" | "issue" | null; open: (typed: string) => boolean } {
  const trackerHosts = useTrackerHosts(repositories);
  const world: LinkWorld = { openPull, repositories, trackerHosts };
  const take = (kind: "pull" | "issue") => (event: Event, text: string | undefined): boolean => {
    if (!routeLink(text, kind, world)) return false;
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
  return useMemo(
    () => ({
      kindOf: (typed: string) => {
        const one = classifyLink(typed, trackerHosts);
        return one.kind === "other" ? null : one.kind;
      },
      open: (typed: string) => routeLink(typed, "any", { openPull, repositories, trackerHosts }),
    }),
    [openPull, repositories, trackerHosts],
  );
}
