// The data-flow sketch of `sketch-labels-and-box-fit`, shared: the web page, the image and the Mermaid text, each
// shown as the person it is sent to would open it. Nothing is written to disk and the machine's clipboard is not
// touched. The walk `sketch-share` plays it.

import type { Scenario } from "../moment";
import { showSaved, useWalkClipboard } from "../sketch-share-view";
import { s247SketchLabelsAndBoxFit } from "./sketch-labels-and-box-fit";

export const s248SketchShare: Scenario = {
  ...s247SketchLabelsAndBoxFit,
  name: "sketch-share",
  says: "A Session whose agent drew the data flow, and what its Share menu hands to somebody without Armada",
  behaves: (fleet) => {
    useWalkClipboard();
    return {
      ...s247SketchLabelsAndBoxFit.behaves?.(fleet),
      // The person keeps the first format offered, which is the PNG; the SVG is the same drawing as vectors.
      saveSketchFile: async (files) => {
        const first = files[0]!;
        showSaved(first);
        return { saved: true, name: first.name };
      },
    };
  },
};
