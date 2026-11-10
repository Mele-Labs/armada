// Where the Operator shows outside its page: on a question in Needs you while it reads it or once it raised it, in a held Job's
// header, and on a Job row a merge or keep in touch ruling named. Each draws nothing in a window with no Operator store.

import type { ReactNode } from "react";
import { Bot, Clock, SquaresIntersect, SquaresUnite } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { tieSaid, waitSaid } from "./model";
import { useOperator } from "./store";
import "./operator.css";

const READING = "The Operator is reading this. Answer it yourself and yours wins";

function Glyph({ said }: { said: string }) {
  return (
    <Tooltip label={said}>
      <span role="img" aria-label={said} className="armada-operator__glyph">
        <Bot size={12} strokeWidth={2} />
      </span>
    </Tooltip>
  );
}

/**
 * What the Operator adds to a Job's question in Needs you: its glyph while it reads it, or its read with its pick selected once
 * raised. `answered` drops the ruling, since the person's answer wins.
 */
export function useOperatorRead(session: string):
  | {
      detail: ReactNode;
      pick?: string;
      answered: () => void;
    }
  | undefined {
  const store = useOperator();
  const read = store?.state.reads.find((one) => one.session === session);
  if (store === null || read === undefined) return undefined;
  const answered = () =>
    store.act({
      kind: "answered",
      session,
    });
  if (read.state === "reading")
    return {
      detail: (
        <p className="armada-operator__read">
          <Glyph said={READING} />
        </p>
      ),
      answered,
    };
  return {
    detail: (
      <p className="armada-operator__read" aria-label="The Operator's read">
        <Glyph said="Operator" />
        {read.read}
      </p>
    ),
    pick: read.pick,
    answered,
  };
}

/** The line under a held Job's title: what it waits for, naming the other Job, and Start now. Nothing once it starts. */
export function HeldNotice({ session }: { session: string }) {
  const store = useOperator();
  const row = store?.state.line.find((one) => one.session === session);
  if (store === null || row === undefined) return null;
  return (
    <div className="armada-operator__held" role="group" aria-label="Held by the Operator">
      <Clock size={12} strokeWidth={2} aria-hidden />
      <span>
        Held until {waitSaid(row.waits)}.{row.why === undefined ? null : ` ${row.why}`}
      </span>
      <Button
        size="sm"
        variant="secondary"
        onClick={() =>
          store.act({
            kind: "start",
            line: row.id,
          })
        }
      >
        Start now
      </Button>
    </div>
  );
}

/** Before a Job row's title, where a ruling on who does the work names another Job: the Job it took in, or the one it is kept in touch with. */
export function TieMark({ session, className }: { session: string; className: string }) {
  const tie = useOperator()?.state.ties.find((one) => one.session === session);
  if (tie === undefined) return null;
  const said = tieSaid(tie);
  const Glyph = tie.kind === "merged" ? SquaresUnite : SquaresIntersect;
  return (
    <Tooltip label={said}>
      <span className={className} role="img" aria-label={said}>
        <Glyph size={12} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}
