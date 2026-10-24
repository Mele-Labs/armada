// The Cockpit tab, as the window mounts it, out of `App.tsx`, which is at the length the gate refuses. The flight deck is its body
// (`cockpit/deck/`), drawn powering on while Fleet comes online (`cockpit/boot.ts`); where Fleet is still down past the grace or
// serves no repository, the empty state draws instead. Work is
// dispatched from the title bar's one bar (`one-bar.tsx`), one panel over one draft (`dispatch-panel/`).
//
// `.armada-screen__overview` is the one child `.armada-screen__mounted` gets — the surface's own
// padding and gap, so neither the strip nor a panel sits against the window's edge. `Boundary`
// renders its children straight through when nothing has thrown.

import type { RepositorySummary } from "@armada/protocol";
import { Boundary } from "@armada/shell";
import { BoardEmpty } from "@armada/screens/src/BoardEmpty";

import type { BridgeState } from "../../shared/bridge";
import type { Hosts } from "./Dashboard";
import { CommandCentral } from "./CommandCentral";
import { PowerContext, powering, type Power } from "./cockpit/boot";
import { FirstRun } from "./FirstRun";

export function Overview({
  state,
  repositories,
  disconnected,
  power = "live",
  onCopied,
  onOpenLink,
  onOpenSession,
  onSendToSession,
  onTell,
  bring,
  onBrought,
  onWantHeld,
  onReleaseSlot,
  onRescueSlot,
  bayActs,
  onPickRepository,
  onAddRepository,
  onGoTo,
  onOpenCheck,
  onOpenLoadout,
}: {
  /** A call an alert asked for, handed to the cockpit once it is mounted. */
  bring?: { key: string; at: number } | null;
  onBrought?: () => void;
  state: BridgeState;
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The connection's own statement, where Fleet cannot be reached. */
  disconnected: string | null;
  /** Whether the Cockpit is powering on (`cockpit/boot.ts`). Live where none is handed. */
  power?: Power;
  onCopied: (value: string) => void;
  /** Hands an address to whatever opens addresses on this machine: the merge line's pull requests. */
  onOpenLink: (address: string) => void;
  /** A Session the list opened. Where no Sessions are served the list draws nothing. */
  onOpenSession: (sessionId: string) => void;
  /** Sends main's red to a Session as a message, from the merge line's band. */
  onSendToSession?: Hosts["onSendToSession"];
  /** Says a sentence as a toast: a refusal the panel has to name. */
  onTell?: (sentence: string) => void;
  /** Asks Fleet for the worktree pool while the Cockpit's bays are drawn. */
  onWantHeld?: (want: boolean) => void;
  onReleaseSlot?: Hosts["onReleaseSlot"];
  onRescueSlot?: Hosts["onRescueSlot"];
  bayActs?: Hosts["bayActs"];
  /** Picks a repository, as the title row's picker does: Autopilot's control asks for one on All. */
  onPickRepository?: Hosts["onPickRepository"];
  onAddRepository?: Hosts["onAddRepository"];
  onGoTo?: Hosts["onGoTo"];
  onOpenCheck?: Hosts["onOpenCheck"];
  onOpenLoadout?: Hosts["onOpenLoadout"];
}) {
  const guarded = { bridge: state.bridge, onCopied };
  const pickedRepository = repositories.find((one) => one.root === state.repository) ?? null;
  // A Fleet coming online draws the deck powering on, whatever the connection says on the way; past the grace it is a fault.
  const coming = powering(power);
  // Fleet unreachable, still starting, or nothing served: the empty state says so.
  const fault = !coming && (disconnected !== null || state.connection.state === "starting");
  // A Fleet that serves nothing is not a fault: the stage is a prompt, and a Session starts on any folder.
  const fresh = !fault && !coming && repositories.length === 0;

  return (
    <Boundary region="the overview" {...guarded}>
      <div className="armada-screen__overview">
        {fault ? (
          <div role="region" aria-label="Cockpit">
            <BoardEmpty disconnected={disconnected} starting={state.connection.state === "starting"} nothingServed={repositories.length === 0} />
          </div>
        ) : fresh ? (
          <div role="region" aria-label="Cockpit" className="armada-first-run__stage">
            <FirstRun {...(onTell === undefined ? {} : { onTell })} />
          </div>
        ) : (
          <PowerContext.Provider value={power}>
            <CommandCentral
              state={state}
              picked={pickedRepository}
              bring={bring}
              onBrought={onBrought}
              onTell={onTell}
              onOpenSession={onOpenSession}
              onOpenLink={onOpenLink}
              onOpenCheck={onOpenCheck}
              onOpenLoadout={onOpenLoadout}
              onSendToSession={onSendToSession}
              onWantHeld={onWantHeld}
              onReleaseSlot={onReleaseSlot}
              onRescueSlot={onRescueSlot}
              bayActs={bayActs}
              onPickRepository={onPickRepository}
              onAddRepository={onAddRepository}
              onGoTo={onGoTo}
            />
          </PowerContext.Provider>
        )}
      </div>
    </Boundary>
  );
}
