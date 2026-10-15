// The owner chip drawn from what Fleet serves: the `owner` on the merge line's open pull requests and on
// the panel's page, read out of the central store by Fleet. Where a window holds a registry already (the
// mock's `draft.prOwners`), a pull request the wire names no owner for is asked of that one.
//
// **Only the merge line and the panel are inside the provider.** The Manifest form draws its switch
// wherever a registry is held, and Fleet serves no such key to it yet.

import { useMemo } from "react";
import type { ReactNode } from "react";
import { PrOwnerRegistryProvider, repositoryOfAddress, usePrOwnerRegistry } from "@armada/components";
import type { PrOwnerRegistry, RegisteredOwnerRow } from "@armada/components";
import type { MergeLines, PullRequestOwner } from "@armada/protocol";

const keyOf = (repository: string, number: number) => `${repository}#${number}`;

/** The wire's owner as the chip draws it. */
export const rowOf = (owner: PullRequestOwner): RegisteredOwnerRow => ({
  kind: owner.kind,
  id: owner.id,
  machine: { name: owner.machine, here: owner.this_machine },
});

/** Every owner the lines carry, by `owner/name#number`. */
export function ownersOnLines(lines: MergeLines | null | undefined): ReadonlyMap<string, RegisteredOwnerRow> {
  const found = new Map<string, RegisteredOwnerRow>();
  for (const line of lines?.lines ?? []) {
    for (const pull of line.hub?.pull_requests ?? []) {
      const repository = repositoryOfAddress(pull.url);
      if (repository !== undefined && pull.owner !== undefined) found.set(keyOf(repository, pull.number), rowOf(pull.owner));
    }
  }
  return found;
}

/** The pull request a panel is open on, with the owner its page carried. */
export type OwnedPanel = { repository: string | undefined; number: number; owner: PullRequestOwner | undefined };

/**
 * Hands the chips under it the owners the wire names: `lines` for the merge line, and `panel` for a pull
 * request panel, whose own page is read when it opens and wins over the lines.
 */
export function PrOwnersOnTheWire({ lines, panel, children }: { lines?: MergeLines | null; panel?: OwnedPanel; children: ReactNode }) {
  const outer = usePrOwnerRegistry();
  const owners = useMemo(() => {
    const found = new Map(ownersOnLines(lines));
    if (panel?.repository !== undefined && panel.owner !== undefined) found.set(keyOf(panel.repository, panel.number), rowOf(panel.owner));
    return found;
  }, [lines, panel?.repository, panel?.number, panel?.owner]);
  const value = useMemo<PrOwnerRegistry>(
    () => ({
      ownerOf: (repository, number) => owners.get(keyOf(repository, number)) ?? outer?.ownerOf(repository, number),
      registering: outer?.registering ?? false,
      setRegistering: outer?.setRegistering ?? (() => undefined),
    }),
    [owners, outer],
  );
  return <PrOwnerRegistryProvider value={value}>{children}</PrOwnerRegistryProvider>;
}
