// The slim bar across the top of the window while the running build holds Jobs the person has not kept: Keep tells
// each Job to land its change, Revert swaps back onto the build before. `trying` is Fleet's list of titles, or the
// branch where no Job holds it.

import { Button } from "@armada/components";
import "./trying-bar.css";

export function TryingBar({
  trying,
  onKeep,
  onRevert,
}: {
  trying: readonly string[];
  onKeep: () => void;
  onRevert: () => void;
}) {
  if (trying.length === 0) return null;
  return (
    <section className="armada-trying-bar" aria-label="Trying">
      <p className="armada-trying-bar__said">{`Trying ${trying.join(", ")}`}</p>
      <div className="armada-trying-bar__acts">
        <Button variant="secondary" size="sm" onClick={onKeep}>
          Keep
        </Button>
        <Button variant="secondary" size="sm" onClick={onRevert}>
          Revert
        </Button>
      </div>
    </section>
  );
}
