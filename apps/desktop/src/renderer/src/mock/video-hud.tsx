// The mission-control overlay of the film's boards, on `?hud=1`: a mono mission clock top left and one status stamp top right, read
// from the beat the walk stands on (`video-boards-beats.ts`). Drawn over the app in a root of its own and outside the accessibility
// tree, so no walk step can find it. Mock only: nothing the Electron build bundles imports this file.

import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "./video-hud.css";
import { BEATS } from "./video-boards-beats";

const toneOf = (stamp: string): "go" | "decision" | "landed" =>
  stamp.endsWith("TOUCHDOWN") ? "landed" : stamp.endsWith("SIGN-IN") || stamp.endsWith("REQUESTED") || stamp.endsWith("CHECK") ? "decision" : "go";

function Hud() {
  const [at, setAt] = useState(1);
  // The walk's handle appears once its player mounts, and moves on each step: read it as it stands.
  useEffect(() => {
    const read = window.setInterval(() => {
      const standing = window.__armadaWalk?.state().at;
      if (standing !== undefined) setAt(standing);
    }, 100);
    return () => window.clearInterval(read);
  }, []);
  const beat = BEATS[Math.min(Math.max(at, 1), BEATS.length) - 1]!;
  return (
    <div aria-hidden="true">
      <span className="armada-mock-hud__clock">{beat.clock}</span>
      {beat.stamp === undefined ? null : (
        <span className="armada-mock-hud__stamp" data-tone={toneOf(beat.stamp)}>
          {beat.stamp}
        </span>
      )}
    </div>
  );
}

export function mountHud(): void {
  const host = document.createElement("div");
  document.body.append(host);
  createRoot(host).render(<Hud />);
}
