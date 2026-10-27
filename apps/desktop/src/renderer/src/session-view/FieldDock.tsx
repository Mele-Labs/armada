// The Field in the Cockpit's lower left, under the Jobs list: the open Job's field map when a Job is open, and the Cockpit's own map when none is.
// It is a panel of the deck's left column, sized and dragged by its gap like the others (`cockpit/deck/splits.tsx`), and a layout can hide it (`field`
// of `dashboard.panels`). The Job's band keeps its Expand control and its key, ⌥F, which opens or folds it.

import type { ReactNode } from "react";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { FieldPanel } from "./FieldPanel";
import { SvPanel } from "./SvPanel";
import { bandKeyFor } from "./story";
import "./flight.css";

export function FieldDock({ session, repositories, map }: { session: Session | undefined; repositories: readonly RepositorySummary[]; /** The Cockpit's own map, drawn when no Job is open. */ map: ReactNode }) {
  if (session !== undefined) return <FieldPanel session={session} repositories={repositories} place="left" />;
  return (
    <SvPanel label="Field" area="field" place="left" keycap={bandKeyFor("field")}>
      {map}
    </SvPanel>
  );
}
