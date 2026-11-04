// Where the Usage page gets what the tokens were spent on. **Fleet serves none yet**, so the window holds nothing and the page
// offers no Spent on view; a mock scenario may hand a stand-in.

import { createContext, useContext, type ReactNode } from "react";
import type { UsageSpent } from "@armada/components";

const Source = createContext<UsageSpent | undefined>(undefined);

export function SpentFrom({
  spent,
  children,
}: {
  spent: UsageSpent | undefined;
  children: ReactNode;
}) {
  return <Source.Provider value={spent}>{children}</Source.Provider>;
}

export const useSpent = (): UsageSpent | undefined => useContext(Source);
