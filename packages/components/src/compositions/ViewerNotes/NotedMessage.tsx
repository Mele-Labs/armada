import { createContext } from "react";

import { Prose } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { notedSaid, type NotedBlock, type NotedRef } from "./compose";

/** How a thread opens what a sent note named, and the Job's checkout, which a file's path is shown from. */
export type NotedOpen = { open: (ref: NotedRef) => void; root?: string };

/** Given by a thread that can open what its notes name. Absent, a notes message draws as the words it is. */
export const NotedOpens = createContext<NotedOpen | null>(null);

const nameOf = (path: string) => path.split("/").pop() ?? path;

function opensSaid(ref: NotedRef): string {
  if (ref.kind === "sketch") return `Open ${ref.title}, ${ref.parts.length === 1 ? "part" : "parts"} picked`;
  if (ref.kind === "file") return `Open ${nameOf(ref.path)}, ${ref.from === ref.to ? `line ${String(ref.from)}` : `lines ${String(ref.from)}-${String(ref.to)}`}`;
  return `Open ${nameOf(ref.path)}${ref.region === undefined ? "" : ", region marked"}`;
}

/**
 * A sent notes message, as the thread draws it: each note's number, what it is about as a press that opens it again,
 * and the note under it. The message the agent was sent is the same words, whole.
 */
export function NotedMessage({ blocks, opens }: { blocks: readonly NotedBlock[]; opens: NotedOpen }) {
  return (
    <ol className="armada-noted" aria-label="Notes">
      {blocks.map((one) => (
        <li key={one.n} className="armada-noted__note">
          <span className="armada-noted__head">
            <span className="armada-noted__n">{one.n}.</span>
            <Tooltip label={opensSaid(one.ref)}>
              <button type="button" className="armada-prose__link armada-noted__ref" data-kind={one.ref.kind} onClick={() => opens.open(one.ref)}>
                {notedSaid(one.ref, opens.root)}
              </button>
            </Tooltip>
          </span>
          {one.body === "" ? null : (
            <div className="armada-noted__body">
              <Prose text={one.body} />
            </div>
          )}
        </li>
      ))}
    </ol>
  );
}
