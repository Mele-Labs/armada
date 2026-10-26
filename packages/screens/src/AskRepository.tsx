// What a surface that needs one repository draws on All repositories: the question, and the
// rail's own options. Picking answers it by picking in the rail, so one pick is all there is.
//
// **Two callers, one gate between them.** The Manifest surface's own ask picks any repository,
// set up or not — `RepositoryOptions` lists both. New job cannot start where Fleet gave a
// folder no id (no Manifest and no `implicit_id`), and #959 decided that picking one here must
// not route anywhere or narrow the Board, so `onlySetUp` shows it greyed out and unpickable instead.
//
// **`action` is the composer's alone, and a rail surface passes none.** The composer is drawn
// over whatever was showing — the rail stays marked on it — so its Close has a place to return
// to and #1366 put it on the ask's own title. Studios and Manifest are rail destinations: `goTo`
// clears every other surface before it sets one, nothing opens either over anything, and the rail
// marks the surface itself. A Close there would have no state to close to. The design system says
// the same for the trail — *a surface reached from Navigation carries no trail*, because
// Navigation is already the answer to where you are — and `actions.deck_tabs` (`⌘1`–`⌘7`)
// is the registered way off one. `actions.close` is for an overlay or a detail route, and the ask
// here is neither: it is the surface's first step, answered in place by picking.

import type { ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import { Alert, DropdownMenu } from "@armada/components";
import { FIND_A_REPOSITORY, repositoryEntries, rootOf, useRecentRepositories } from "@armada/shell";

export function AskRepository({
  repositories,
  title,
  next,
  onPick,
  onlySetUp = false,
  action,
}: {
  repositories: readonly RepositorySummary[];
  /** What the surface needs one repository for. Sentence case, no Wh- opener. */
  title: string;
  /** What picking does. */
  next: string;
  /** A root Fleet lists. */
  onPick: (root: string) => void;
  /**
   * New job's own use — #959: a repository with neither a Manifest nor an
   * `implicit_id` has nothing for the Job to belong to, so it is shown rather
   * than hidden, and cannot be picked. `false`, the default, is the Manifest
   * surface's own ask, which picks any repository.
   */
  onlySetUp?: boolean;
  /**
   * The one control the ask carries — the composer's own way out. It sits at
   * the trailing edge of the title's line, which is where the two card states
   * of the same composer draw theirs. Absent draws none, which is what the two
   * rail surfaces pass — the head says why.
   */
  action?: ReactNode;
}) {
  const [recent, remember] = useRecentRepositories();
  // The same menu the title row's picker is, filter field and all, so one habit finds a repository
  // in either place and a list of twenty is no longer a scroll. The trigger keeps the old prompt.
  return (
    <Alert tone="neutral" title={title} action={action} actionOn="title">
      <p>{next}</p>
      <DropdownMenu
        align="start"
        triggerLabel="Choose a repository"
        searchLabel={FIND_A_REPOSITORY}
        entries={repositoryEntries(repositories, { onlySetUp, recent })}
        onSelect={(picked) => {
          const root = rootOf(picked);
          remember(root);
          onPick(root);
        }}
      />
    </Alert>
  );
}
