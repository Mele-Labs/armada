// Where the flight deck's map gets the directories a repository is drawn in. **Fleet reports none yet** (F1 of the port, #2301), so
// the window's source is `noAreas` and a territory draws its pads and its main flag with no plots; a scenario may hand a stand-in.

import { createContext, useContext, type ReactNode } from "react";

import { noAreas, type AreaSource } from "./areas";

const Source = createContext<AreaSource>(noAreas);

/** Hands the window a source of areas. Nothing handed, nothing is drawn. */
export function AreasFrom({ source, children }: { source: AreaSource | undefined; children: ReactNode }) {
  return source === undefined ? <>{children}</> : <Source.Provider value={source}>{children}</Source.Provider>;
}

export const useAreas = (): AreaSource => useContext(Source);
