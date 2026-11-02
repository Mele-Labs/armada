// A root of its own, beside the app's rather than inside it: the app's error
// boundary, its keyboard handlers and its fibers never see the layer. It holds
// its own Sessions, for the Job card a sent note shows.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { WiredSessions } from "../sessions-wired";
import { Layer } from "./Layer";
import { devServerSink, mainSink, type Sink } from "./sink";

/** `story` loads a story file for the card's Variant tiles: the mock's, which has the files; Electron passes none. */
export function mount(story?: Sink["story"]): void {
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const sink = window.armadaDev !== undefined ? mainSink(window.armadaDev) : { ...devServerSink(), story };
  createRoot(host).render(
    <StrictMode>
      <WiredSessions>
        <Layer sink={sink} />
      </WiredSessions>
    </StrictMode>,
  );
}
