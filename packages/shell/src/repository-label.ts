// What a repository is called wherever Bridge names one: the title row's picker
// and, on All repositories, a Board row. No React, so a node test reaches it.
//
// A set-up repository read as `manifest.id` alone until 28 Sep 2026, on the
// reasoning that Fleet refuses to serve one id twice. `ManifestId` is a newtype
// over a `Ulid`, so the picker drew `01M1CNPKTV0018H2M1CXDNBK06` —
// `docs/concepts/manifest.md`, Deletion: a surface renders the name, never a
// bare id. One rule now, climbing the same ladder for both kinds.

import type { RepositorySummary } from "@armada/protocol";

/**
 * What to call one repository among the set Fleet serves: a word, then
 * `parent/folder`, then the whole root — each rung taken only because the one
 * above it names something else too. The root is always the hover.
 *
 * **Every other repository is a rival, set up or not.** The labels share one
 * namespace, so a comparison that skipped the set-up ones would let two rows
 * read alike, which is what the ladder exists to prevent.
 */
export function repositoryLabel(
  repository: RepositorySummary,
  repositories: readonly RepositorySummary[],
): string {
  const others = repositories.filter((other) => other.root !== repository.root);
  const word = wordOf(repository);
  if (!others.some((other) => wordOf(other) === word)) return word;
  const placed = placedOf(repository.root);
  return others.some((other) => placedOf(other.root) === placed) ? repository.root : placed;
}

/** A Job's repository, by the Manifest id it carries. One no longer served reads as that id. */
export function manifestLabel(manifestId: string, repositories: readonly RepositorySummary[]): string {
  const served = repositories.find((one) => one.manifest?.id === manifestId);
  return served === undefined ? manifestId : repositoryLabel(served, repositories);
}

/**
 * The readable word for a repository, before any widening.
 *
 * `manifest.repository` is Fleet's own name for what it read — the workspace's
 * folder rather than the checkout's inside a monorepo, which is more precise
 * than the root's last segment. That segment is all a repository nobody wrote
 * an `armada.yml` for has to offer.
 */
function wordOf(repository: RepositorySummary): string {
  return repository.manifest?.repository ?? folderOf(repository.root);
}

function placedOf(root: string): string {
  const parts = root.replace(/\/+$/, "").split("/");
  return `${parts.at(-2) ?? ""}/${parts.at(-1) ?? root}`;
}

function folderOf(root: string): string {
  return root.replace(/\/+$/, "").split("/").pop() || root;
}
