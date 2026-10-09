// Which repository a request is for, read from its words. MOCK ONLY: Fleet answers nothing like this
// yet, so the panel is drawn against a stand-in. A word naming the repository, or one of the words
// it is known by here, picks it; a request that names none, or names two, is unresolved, which is
// the state the panel's inline ask exists for. The wait is what a real reading would cost, so the
// panel's detecting state is drawn long enough to be seen.

import { useEffect, useState } from "react";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import type { RepositoryOption } from "./DispatchPanel";

/** How long the stand-in takes to read the words. */
export const READ_MS = 600;

/** What each repository is also called in a request, by its label. A repository named nothing here is known by its own name. */
const KNOWN_AS: Record<string, RegExp> = {
  armada: /\b(armada|bridge|cockpit|fleet|drones?)\b/i,
  storefront: /\b(storefront|cart|checkout|catalogue)\b/i,
  billing: /\b(billing|invoices?|ledger)\b/i,
};

const escaped = (word: string) => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The set-up repositories, those the words name first and then those with the most Jobs on the Board. */
export function optionsFor(words: string, repositories: readonly RepositorySummary[], jobs: readonly JobSummary[]): (RepositoryOption & { named: boolean })[] {
  return repositories
    .filter((one) => one.manifest !== undefined)
    .map((one) => {
      const name = repositoryLabel(one, repositories);
      const known = KNOWN_AS[name.toLowerCase()] ?? new RegExp(`\\b${escaped(name)}\\b`, "i");
      const busy = jobs.filter((job) => job.owner_manifest_id === one.manifest?.id).length;
      return { root: one.root, name, named: known.test(words), busy };
    })
    .sort((a, b) => Number(b.named) - Number(a.named) || b.busy - a.busy || a.name.localeCompare(b.name))
    .map(({ busy, ...one }) => ({ ...one, likely: one.named || busy > 0 }));
}

export type Reading = { state: "idle" } | { state: "detecting" } | { state: "unresolved" } | { state: "detected"; root: string };

/** The reading of `words` once they have been still for `READ_MS`. Empty words read nothing. */
export function useReading(words: string, named: readonly string[]): Reading {
  const said = words.trim();
  const key = named.join("\n");
  const [read, setRead] = useState<{ said: string; key: string } | null>(null);
  useEffect(() => {
    if (said === "") return setRead(null);
    const timer = window.setTimeout(() => setRead({ said, key }), READ_MS);
    return () => window.clearTimeout(timer);
  }, [said, key]);
  if (said === "") return { state: "idle" };
  if (read === null || read.said !== said || read.key !== key) return { state: "detecting" };
  return named.length === 1 ? { state: "detected", root: named[0]! } : { state: "unresolved" };
}
