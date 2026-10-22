import { createContext, useContext } from "react";
import { Save } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

/** **Proposed, not on the wire**: drawn from a mock's rows until Fleet writes one. */
export type SavedThreadRow = Extract<SessionThreadRow, { kind: "saved" }>;

/** Opens a saved request in Requests, in its collection. A context as `RequestOpen` is; absent, the row has no press for it. */
export const SavedOpen = createContext<((row: SavedThreadRow) => void) | null>(null);

/** The agent saved a request into a collection: one quiet line naming both, and the press that opens it in Requests. */
export function SavedRow({ row }: { row: SavedThreadRow }) {
  const onOpen = useContext(SavedOpen);
  const said = `Saved ${row.name} to ${row.collection}`;
  return (
    <li className="armada-session-saved" aria-label={said}>
      <Tooltip label="Saved request">
        <span className="armada-session-saved__mark" role="img" aria-label="Saved request">
          <Save size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-session-saved__said">{said}</span>
      <Stamp still at={row.at} className="armada-session-saved__at" />
      {onOpen === null ? null : (
        <Button variant="ghost" size="sm" onClick={() => onOpen(row)}>
          Open in Requests
        </Button>
      )}
    </li>
  );
}
