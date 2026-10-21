// The one ask before a shell tile the repository committed first runs on this Mac: its name, the command it runs, and Run. Run
// approves that command by the hash Fleet showed, so an approval cannot cover a later command, then presses the tile.
// `docs/design/who-sees-each-piece-of-configuration.md`, *A tile a teammate committed asks before it runs*.

import { Dialog } from "@armada/components";
import type { Outcome, TileAsk } from "@armada/protocol";

/** The ask a press was refused with, where Fleet refused it as not approved here: its `fields` carry the command and its hash. */
export function askOfRefusal(outcome: Outcome, id: string): TileAsk | undefined {
  if (outcome.ok || outcome.why !== "refused" || outcome.error.code !== "fleet.tile_not_approved") return undefined;
  const { command, command_hash } = outcome.error.fields;
  return typeof command === "string" && typeof command_hash === "string" ? { id, command, command_hash, approved: false } : undefined;
}

export function TileAskDialog({ name, ask, said, onRun, onCancel }: { name: string; ask: TileAsk; said?: string; onRun: () => void; onCancel: () => void }) {
  return (
    <Dialog open title={name} tone="neutral" confirmLabel="Run" onConfirm={onRun} onCancel={onCancel}>
      <pre className="deck-ask__command" aria-label="Command">
        {ask.command}
      </pre>
      {said === undefined ? null : (
        <p className="deck-ask__said" role="alert">
          {said}
        </p>
      )}
    </Dialog>
  );
}
