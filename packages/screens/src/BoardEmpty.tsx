// What the Cockpit says when Fleet cannot be reached, is still starting, or serves no repository yet.

import { BoardEmptyState, fleetSaid, FleetStarting, STARTING_PHRASE } from "@armada/components";

import { NOTHING_SERVED } from "./locate-reads";

export function BoardEmpty({
  disconnected,
  starting = false,
  nothingServed,
}: {
  disconnected: string | null;
  /** Fleet's process is up and has not answered yet: the glyph alone, and no command to run. */
  starting?: boolean;
  /** Fleet has listed no repository, so there is nothing to dispatch against. */
  nothingServed: boolean;
}) {
  // A Fleet that is not running is a fault Bridge cannot fix; one that is up and serves nothing is a null result.
  if (disconnected !== null && starting) {
    // A Fleet that is coming up has nothing to be started: no command, the mark the Fleet
    // panel's head draws and a phrase under it, named on hover.
    return (
      <BoardEmptyState>
        <span role="img" aria-label="Starting" title={fleetSaid("Starting")}>
          <FleetStarting className="armada-fleet-starting" size={32} phrase={STARTING_PHRASE} />
        </span>
      </BoardEmptyState>
    );
  }
  if (disconnected !== null) {
    return (
      <BoardEmptyState command="armada fleet start" note={disconnected}>
        Fleet is not connected, so there is nothing to show.
      </BoardEmptyState>
    );
  }
  // A fresh install: a dispatch would send a person to a composer with nothing to name.
  return nothingServed ? <BoardEmptyState quiet>{`${NOTHING_SERVED.title}. ${NOTHING_SERVED.next}`}</BoardEmptyState> : null;
}
