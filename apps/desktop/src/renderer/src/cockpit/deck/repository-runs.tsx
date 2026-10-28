// Where a repository's Armada configuration is kept, for the title row's repository list: the glyph of the place on a
// row, and all of how the repository runs as a layer that → on the row (or a press on the glyph) opens. The Cockpit's
// combobox (`RepositoryCombobox`) draws the same glyph and the same layer for the same rows; this is the list's copy, held
// by the app because the title row's package cannot reach the run book.

import { useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { PLACE_GLYPH, RunsAll, RunsPlaces, placePhrase, useAccount } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import { chooseFolder } from "../../commands";
import { NO_SYNCED, profilesOf, useRunBook } from "../../run-book";
import { boundProfile } from "../../run-places";

export type RepositoryRuns = {
  /** The row's glyph and the act that opens its layer, or `undefined` where Fleet has not said where the runs are kept. */
  trailOf: (root: string, label: string) => { node: ReactNode; open: () => void } | undefined;
  /** The layer, drawn while one is open. Rendered once, anywhere. */
  layer: ReactNode;
};

export function useRepositoryRuns(repositories: readonly RepositorySummary[]): RepositoryRuns {
  const [open, setOpen] = useState<string | null>(null);
  const [changing, setChanging] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  const book = useRunBook();
  const account = useAccount();

  const openRuns = (root: string) => {
    setChanging(false);
    setRefused(undefined);
    setOpen(root);
    void book.read(root);
  };
  const closeRuns = () => {
    setOpen(null);
    setChanging(false);
    setRefused(undefined);
  };

  const trailOf: RepositoryRuns["trailOf"] = (root, label) => {
    // Asking for the place is what has Fleet read it, as the combobox's list asks as it opens.
    const kept = book.placeOf(root, boundProfile(account, root));
    if (kept === undefined) return undefined;
    const Glyph = PLACE_GLYPH[kept.id];
    return {
      open: () => openRuns(root),
      node: (
        <span
          role="button"
          tabIndex={-1}
          className="deck-combo__mark"
          aria-label={`Armada configuration for ${label}`}
          title={`Armada configuration kept ${placePhrase(kept)} (→)`}
          onMouseDown={(event) => event.preventDefault()}
          onClick={(event) => {
            event.stopPropagation();
            openRuns(root);
          }}
        >
          <Glyph size={12} aria-hidden="true" />
        </span>
      ),
    };
  };

  const repository = open === null ? undefined : repositories.find((one) => one.root === open);
  const shown = open === null || repository === undefined ? undefined : book.runsOf(open, repositoryLabel(repository, repositories), boundProfile(account, open));
  const layer =
    open === null || shown === undefined
      ? null
      : createPortal(
          <div className="deck-runs-layer" onPointerDown={(event) => event.target === event.currentTarget && closeRuns()}>
            <RunsAll
              name={shown.repository}
              source={shown.source}
              rows={shown.rows}
              place={shown.place}
              said={shown.said}
              onDrop={(id) => void book.drop(open, id)}
              changing={changing}
              onChange={() => setChanging(!changing)}
              chooser={
                !changing ? null : (
                  <RunsPlaces
                    synced={NO_SYNCED}
                    profiles={profilesOf(boundProfile(account, open))}
                    current={shown.place}
                    refused={refused}
                    onFolder={chooseFolder}
                    onPick={(place) =>
                      void book.place(open, place).then((said) => {
                        setRefused(said);
                        if (said === undefined) setChanging(false);
                      })
                    }
                    onClose={() => setChanging(false)}
                  />
                )
              }
              onClose={closeRuns}
            />
          </div>,
          document.body,
        );
  return { trailOf, layer };
}
