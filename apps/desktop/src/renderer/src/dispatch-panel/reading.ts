// Which repository a request is for, as Fleet reads it from the words (`read_request_repository`).
// The read waits for the words to be still, and an answer to words no longer there is let go, so the
// chip never names a repository for a sentence that has since changed. Fleet ranks every set-up
// repository, best first; the panel's pick list follows that ranking and still lists all of them.

import { useEffect, useState } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf, repositoryLabel } from "@armada/shell";

import type { RepositoryChip, RepositoryOption } from "./DispatchPanel";

/** How long the words stay still before they are read. */
export const READ_MS = 250;

/** What Fleet answers when it cannot tell, and what a window with no Fleet to ask answers. */
const NOTHING_READ = { manifest_id: null, likely: [] };

type Answer = { manifest_id: string | null; likely: readonly string[] };

/** Every repository, set up or not, in Fleet's order, then any it did not rank by name. Only the head of a ranking is marked likely. */
export function optionsFor(repositories: readonly RepositorySummary[], likely: readonly string[]): RepositoryOption[] {
  const place = (one: RepositorySummary) => {
    const at = likely.indexOf(jobIdOf(one) ?? "");
    return at === -1 ? likely.length : at;
  };
  return repositories
    .map((one) => ({ root: one.root, name: repositoryLabel(one, repositories), at: place(one) }))
    .sort((a, b) => a.at - b.at || a.name.localeCompare(b.name))
    .map(({ at, ...one }) => ({ ...one, likely: at === 0 && likely.length > 0 }));
}

/**
 * The row of chips, in order: the repository the rail is on, the one the words name, the ones picked last,
 * then the rest as `options` ranks them. The order does not follow a pick, so a key keeps meaning the chip it was read from.
 */
export function chipsFor(
  options: readonly RepositoryOption[],
  { scoped, read, chosen, recent }: { scoped?: string | undefined; read?: string | undefined; chosen?: string | undefined; recent: readonly string[] },
): RepositoryChip[] {
  const lead = [scoped, read, ...recent].filter((root): root is string => root !== undefined);
  const ahead = [...new Set(lead)].flatMap((root) => options.filter((one) => one.root === root));
  const rest = options.filter((one) => !ahead.includes(one));
  return [...ahead, ...rest].map(({ root, name }) => ({ root, name, ...(root === chosen ? { mark: "chosen" as const } : root === read ? { mark: "read" as const } : {}) }));
}

export type Reading = { state: "idle" } | { state: "detecting" } | { state: "unresolved" } | { state: "detected"; manifestId: string };

/** The reading of `words` once they have been still for `READ_MS`, and Fleet's ranking of the repositories. Empty words read nothing. */
export function useReading(words: string): { reading: Reading; likely: readonly string[] } {
  const said = words.trim();
  const [answer, setAnswer] = useState<(Answer & { said: string }) | null>(null);
  useEffect(() => {
    if (said === "") return setAnswer(null);
    let stale = false;
    const timer = window.setTimeout(() => {
      // No connection answers `undefined` rather than a reading.
      void window.armada.readRequestRepository(said).then((read) => {
        if (!stale) setAnswer({ said, ...(read ?? NOTHING_READ) });
      });
    }, READ_MS);
    return () => {
      stale = true;
      window.clearTimeout(timer);
    };
  }, [said]);
  const mine = answer?.said === said ? answer : null;
  const likely = mine?.likely ?? answer?.likely ?? [];
  if (said === "") return { reading: { state: "idle" }, likely };
  if (mine === null) return { reading: { state: "detecting" }, likely };
  return { reading: mine.manifest_id === null ? { state: "unresolved" } : { state: "detected", manifestId: mine.manifest_id }, likely };
}
