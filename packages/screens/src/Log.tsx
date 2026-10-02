// The activity log — chapter two of a step's story, streaming, with nothing in
// front of it.
//
// **It was behind a tab.** The stream only drew after pressing a control called
// *The drone's turns*, inside a four-tab region the drawing has none of. So the
// one chapter that says what is happening right now was the one thing a person
// had to go and find. It is on the page now, at every state, and it fills as
// rows arrive.
//
// **Every line opens in place.** A row and its payload are one thing in the
// order it happened, so a payload opens beneath its row rather than replacing
// the list or opening a pane beside it. Which row is open is held here, because
// it is a property of reading this log and not of the Job. The keyboard used
// to open rows too, with `h`/`l`; that went when the activity log sheet did.

import { LogEntry, PayloadLine, Prose, ToolName } from "@armada/components";
import { useState } from "react";
import type { ReactNode } from "react";

import type { LogRow } from "./story";

/**
 * The attribute on the well that holds a log's rows, carrying which log it is.
 *
 * **A story draws more than one at once** — chapter one's turns and chapter
 * two's preview are two logs over one stream, so the same row is on the screen
 * twice under two ids that are equal. Which one a row belongs to is therefore
 * part of naming it, and this is where that name is written.
 */
const LOG_REGION = "data-armada-log";

/** What a row's payload is called, so the row's control can point at it. */
export function payloadId(rowId: string): string {
  return `${PAYLOAD}${rowId}`;
}

const PAYLOAD = "log-payload-";

export function Log({
  rows,
  emptyNote,
  region,
}: {
  rows: LogRow[];
  /** What an empty log says. Never a blank: a blank reads as a failed render. */
  emptyNote: string;
  /**
   * Which log this is, where the story draws more than one. Two logs over one
   * stream hold the same rows, and a reader who opened a row in chapter one has
   * not opened it in chapter two.
   */
  region: string;
}) {
  const [open, setOpen] = useState<string | null>(null);

  if (rows.length === 0) {
    return (
      <p className="text-2xs text-fg-muted" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <div {...{ [LOG_REGION]: region }}>
      {rows.map((row) => (
        <LogEntry
          key={row.id}
          at={row.at}
          actor={row.actor}
          message={messageOf(row)}
          mono={row.mono}
          {...(row.called === undefined
            ? {}
            : {
                sized: { ...(row.called.added === undefined ? {} : { added: row.called.added }),
                  ...(row.called.deleted === undefined ? {} : { deleted: row.called.deleted }) },
                ...(row.called.took === undefined ? {} : { took: row.called.took }),
              })}
          working={row.working}
          open={open === row.id}
          payloadId={payloadId(row.id)}
          onToggle={() => setOpen(open === row.id ? null : row.id)}
          payload={payloadOf(row)}
          payloadAbsent="This line is the whole of what was recorded."
        />
      ))}
    </div>
  );
}

/**
 * The line a row shows closed.
 *
 * **A call composes the tool's name**, because its family's hue is
 * `ToolName`'s own. The sizes and the figure go to `LogEntry` as values: their
 * hues are the row's stylesheet's, and a screen spelling those classes would
 * own a name it cannot see change. #1196.
 *
 * **The Drone's own sentence is drawn through `Prose`**, because it is
 * markdown a model wrote. Every other row draws its own string as it is — they
 * are Armada's and Fleet's words, and a tool name's underscores are not
 * emphasis.
 */
function messageOf(row: LogRow): ReactNode {
  const call = row.called;
  if (call === undefined) {
    return row.kind === "said" && row.actor === "drone" ? <Prose text={row.message} /> : row.message;
  }
  return (
    <>
      <ToolName tool={call.tool} />
      {`  ${call.detail}`}
    </>
  );
}

/**
 * What an open row shows: the lines it arrived with, or `undefined` where the
 * row carried nothing, which is what draws `LogEntry`'s own absent line.
 */
function payloadOf(row: LogRow): ReactNode {
  return row.payload.length === 0 ? undefined : <>{written(row)}</>;
}

/** The lines the row arrived with. */
function written(row: LogRow): ReactNode {
  return row.payload.map((line, at) => (
    <PayloadLine key={at} named={line.named}>
      {line.text}
    </PayloadLine>
  ));
}
