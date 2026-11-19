import type { SketchSettled, SketchAnswer } from "./sketch-answer";
import { useState, useEffect, type ReactNode } from "react";
import { readKept, forgetKept } from "../../keep";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { CardFooter, Card } from "../../primitives/Card/Card";
import { Button } from "../../primitives/Button/Button";
import {
  BadgeCheck,
  Reply,
  ChevronRight,
  ChevronDown,
  PencilRuler,
  BellRing,
  Maximize2,
} from "lucide-react";
import type { Scene } from "../SketchScene/scene";
import type { ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import type { SessionThreadRow } from "./SessionThread";
import { SketchScene } from "../SketchScene/SketchScene";
import { RevisionMenu } from "../RevisionMenu/RevisionMenu";
import { aspectOf } from "../SketchScene/aspect";
import { BandCard, BandFold, Well, type BandFacts, type Stand } from "../BandCard/BandCard";

const SETTLED = {
  approved: {
    said: "Approved",
    Glyph: BadgeCheck,
  },
  sent_back: {
    said: "Sent back",
    Glyph: Reply,
  },
} as const;

/**
 * What answers a sketch or a document put to the person, wherever it is read: the note he writes to send it back, how it
 * ended once it has, and Approve with Send back while it waits. One part for the thread card, the panel and the Needs you
 * row, so they answer it the same way and a note written in one is kept under `keep`. `id` is handed back with the answer.
 */
export function ApprovalAnswers({
  id,
  keep,
  settled,
  open,
  onAnswer,
  onWithdraw,
}: {
  id: string;
  keep: string;
  settled?: SketchSettled;
  open: boolean;
  onAnswer?: (
    id: string,
    answer: SketchAnswer,
  ) => void /** Where passed, the footer offers Withdraw: take it back without answering. Documents pass none. */;
  onWithdraw?: (id: string) => void;
}) {
  const noteKey = `${keep}:note`;
  const [noting, setNoting] = useState(() => readKept<string>(noteKey) !== undefined);
  const [note, setNote] = useState("");
  const cancel = () => {
    forgetKept(noteKey);
    setNote("");
    setNoting(false);
  };
  // The next revision of a sketch is the same card, so a note sent or settled is put away here, not left open on it.
  useEffect(() => {
    if (settled !== undefined) cancel();
  }, [
    settled,
    noteKey,
  ]);
  const state = settled === undefined ? undefined : SETTLED[settled.how];
  return (
    <>
      {open && noting ? (
        <Textarea
          aria-label="Note"
          rows={3}
          autoFocus
          keep={noteKey}
          value={note}
          onChange={(event) => setNote(event.currentTarget.value)}
        />
      ) : null}
      {settled?.how === "sent_back" && settled.words !== undefined ? (
        <blockquote className="armada-session-sketch__note">{settled.words}</blockquote>
      ) : null}
      {state === undefined ? null : (
        <p className="armada-session-sketch__settled" data-how={settled!.how}>
          <Tooltip label={state.said}>
            <span className="armada-session-sketch__mark" role="img" aria-label={state.said}>
              <state.Glyph size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span aria-hidden>{state.said}</span>
        </p>
      )}
      {!open || onAnswer === undefined ? null : (
        <CardFooter role="group" aria-label="Answers">
          {noting ? (
            <>
              <Button
                size="sm"
                variant="primary"
                disabled={note.trim() === ""}
                onClick={() => {
                  onAnswer(id, {
                    text: note.trim(),
                  });
                  cancel();
                }}
              >
                Send back
              </Button>
              <Button size="sm" variant="ghost" onClick={cancel}>
                Cancel
              </Button>
            </>
          ) : (
            <>
              <Button
                size="sm"
                variant="primary"
                onClick={() =>
                  onAnswer(id, {
                    choice: 0,
                  })
                }
              >
                Approve
              </Button>
              <Button size="sm" variant="secondary" onClick={() => setNoting(true)}>
                Send back
              </Button>
              {onWithdraw === undefined ? null : (
                <Button size="sm" variant="ghost" onClick={() => onWithdraw(id)}>
                  Withdraw
                </Button>
              )}
            </>
          )}
        </CardFooter>
      )}
    </>
  );
}

type Against = {
  title: string;
  scene: Scene;
};

/**
 * What a sketch with `revisions` reads as at revision `picked`, the newest where none is: **each revision against the one before
 * it**, and the first plain. The newest keeps the sketch the agent named as `against` where it named one (`named`), found or not. `earlier` is a revision
 * before the newest, read only: a note on it, or an answer, would be about a drawing that no longer stands.
 */
export function sketchAtRevision(
  revisions: readonly Scene[] | undefined,
  picked: number | undefined,
  newest: {
    scene: Scene;
    against?: Against;
    named?: boolean;
  },
): {
  revision?: number;
  scene: Scene;
  against?: Against;
  earlier: boolean;
} {
  if (revisions === undefined || revisions.length === 0)
    return {
      ...newest,
      earlier: false,
    };
  const count = revisions.length;
  const revision = Math.min(picked ?? count, count);
  const before = revision > 1 ? revisions[revision - 2] : undefined;
  const previous =
    before === undefined
      ? {}
      : {
          against: {
            title: `Revision ${String(revision - 1)}`,
            scene: before,
          },
        };
  const own =
    newest.against === undefined
      ? {}
      : {
          against: newest.against,
        };
  if (revision === count)
    return {
      revision,
      scene: newest.scene,
      ...(newest.named === true || newest.against !== undefined ? own : previous),
      earlier: false,
    };
  return {
    revision,
    scene: revisions[revision - 1] ?? newest.scene,
    ...previous,
    earlier: true,
  };
}

/** The revision a sketch's menu shows, `undefined` for the newest, and the menu itself wherever the sketch has revisions, one included, as a document's. */
function useSketchRevision(revisions: readonly Scene[] | undefined, align: "start" | "end") {
  const [picked, setPicked] = useState<number | undefined>(undefined);
  const count = revisions?.length ?? 0;
  const menu =
    count === 0 ? null : (
      <RevisionMenu
        count={count}
        revision={Math.min(picked ?? count, count)}
        align={align}
        onPick={(revision) => setPicked(revision === count ? undefined : revision)}
      />
    );
  return {
    picked,
    menu,
  };
}

/**
 * A sketch the agent drew, where in the thread it did, as the Band card native Bridge draws (`BandCard`): a header band filled
 * with where it stands, the drawing itself in a well (`SketchScene`'s `preview`), and its answers in a footer. A press on the
 * band or the drawing opens the sketch. Once it has a verdict it is one line. **A card whose sketch was replaced stands back as
 * a line**, because its press opens the sketch that took its place. `keep` names it for the note he types. **The newest card of a
 * sketch with revisions offers each one**; an earlier one is drawn as a change to the one before it, and Approve and Ask for
 * changes still answer the newest.
 */
export function SketchRow({
  row,
  keep,
  onOpen,
  onAnswer,
  onWithdraw,
}: {
  row: Extract<
    SessionThreadRow,
    {
      kind: "sketch";
    }
  >;
  keep?: string;
  onOpen?: (sketch: string) => void;
  onAnswer?: (sketch: string, answer: SketchAnswer) => void;
  onWithdraw?: (sketch: string) => void;
}) {
  const settled = row.settled;
  const open =
    row.asked === true &&
    row.withdrawn !== true &&
    settled === undefined &&
    row.replaced !== true &&
    onAnswer !== undefined;
  const revisions = row.replaced === true ? undefined : row.revisions;
  const { picked, menu } = useSketchRevision(revisions, "end");
  // The agent's own `against` is a card it named; the revisions' is the drawing before. The first wins on the newest, as it always has.
  const shown = sketchAtRevision(revisions, picked, {
    scene: row.drawing,
    named: row.against !== undefined,
    ...(row.against === undefined || row.againstScene === undefined
      ? {}
      : {
          against: {
            title: row.against,
            scene: row.againstScene,
          },
        }),
  });
  const stand: Stand =
    settled !== undefined
      ? settled.how === "approved"
        ? "approved"
        : "sent_back"
      : row.withdrawn === true
        ? "withdrawn"
        : row.asked === true && row.replaced !== true
          ? "asking"
          : "reading";
  const number = shown.revision ?? revisions?.length;
  const facts: BandFacts = {
    form: "sketch",
    title: row.title,
    stand,
    words: settled?.words ?? row.facts?.words,
    ...(row.facts?.revision === undefined
      ? number === undefined
        ? {}
        : {
            revision: {
              number,
              said: `r${String(number)}`,
            },
          }
      : {
          revision: {
            ...row.facts.revision,
            ...(number === undefined
              ? {}
              : {
                  number,
                }),
          },
        }),
    yours: shown.earlier ? undefined : row.facts?.yours,
    forLine: row.facts?.forLine,
  };
  if (row.replaced === true)
    return (
      <li className="pk-bc-row">
        <BandFold
          form="sketch"
          title={row.title}
          stand="withdrawn"
          word="Replaced"
          said="A later drawing took its place"
          onOpen={() => onOpen?.(row.sketch)}
        />
      </li>
    );
  const aspect = aspectOf(shown.scene);
  const well = (
    <Well cut={false} sketch>
      <div
        className="pk-bc__sketch"
        style={{
          aspectRatio: String(aspect),
        }}
      >
        <SketchScene
          key={shown.revision}
          scene={shown.scene}
          {...(shown.against === undefined
            ? {}
            : {
                against: shown.against.scene,
              })}
          preview
        />
      </div>
    </Well>
  );
  const asking = open && !shown.earlier;
  return (
    <BandCard
      facts={facts}
      keep={keep ?? `sketch:${row.id}`}
      well={well}
      between={menu}
      acts={{
        ...(onOpen === undefined
          ? {}
          : {
              open: () => onOpen(row.sketch),
            }),
        ...(asking
          ? {
              approve: () =>
                onAnswer?.(row.sketch, {
                  choice: 0,
                }),
              ask: (note: string) =>
                onAnswer?.(row.sketch, {
                  text: note,
                }),
              ...(onWithdraw === undefined
                ? {}
                : {
                    dismiss: () => onWithdraw(row.sketch),
                  }),
            }
          : {}),
      }}
      asked={asking}
    />
  );
}

/**
 * A sketch read in the panel, read-only: pan, zoom and the steps, drawn by `SketchScene`. **Given
 * the sketch it was drawn against, it is read as a change to it** (added green, removed red,
 * changed amber), and a band offers the whole drawing instead. `menu`, a revision menu, ends the band.
 */
export function SketchReading({
  scene,
  title,
  against,
  notes,
  marked,
  answer,
  menu,
}: {
  scene: unknown;
  title?: string;
  against?: {
    title: string;
    scene: unknown;
  };
  notes?: ViewerNotesScope;
  marked?: readonly string[] /** Present while the sketch waits on him: the same Approve and Send back the thread card has. */;
  answer?: {
    sketch: string;
    keep: string;
    onAnswer: (sketch: string, answer: SketchAnswer) => void;
    onWithdraw?: (sketch: string) => void;
  };
  menu?: ReactNode;
}) {
  const [changes, setChanges] = useState(true);
  return (
    <div className="armada-session-sketch-read">
      {against === undefined && menu === undefined ? null : (
        <div className="armada-session-sketch-read__band">
          <span className="armada-session-sketch-read__against">
            {against === undefined ? null : `Against ${against.title}`}
          </span>
          {against === undefined ? null : (
            <span className="armada-session-sketch-read__reads" role="group" aria-label="Read as">
              <Button
                size="sm"
                variant={changes ? "secondary" : "ghost"}
                aria-pressed={changes}
                onClick={() => setChanges(true)}
              >
                Changes
              </Button>
              <Button
                size="sm"
                variant={changes ? "ghost" : "secondary"}
                aria-pressed={!changes}
                onClick={() => setChanges(false)}
              >
                Whole
              </Button>
            </span>
          )}
          {menu}
        </div>
      )}
      <SketchScene
        scene={scene}
        autoplay
        {...(title === undefined
          ? {}
          : {
              title,
            })}
        {...(against === undefined || !changes
          ? {}
          : {
              against: against.scene,
            })}
        {...(notes === undefined
          ? {}
          : {
              notes,
            })}
        {...(marked === undefined
          ? {}
          : {
              marked,
            })}
      />
      {answer === undefined ? null : (
        <div className="armada-session-sketch__answers">
          <ApprovalAnswers
            id={answer.sketch}
            keep={answer.keep}
            open
            onAnswer={answer.onAnswer}
            {...(answer.onWithdraw === undefined
              ? {}
              : {
                  onWithdraw: answer.onWithdraw,
                })}
          />
        </div>
      )}
    </div>
  );
}

/** A sketch waiting on the person, pinned as the plan to approve is. */
export type SketchPinProps = {
  sketch: string;
  title: string;
  scene: Scene;
  against?: {
    title: string;
    scene: Scene;
  } /** Where his notes on it are held, and the key the note he types is kept under. */;
  notes?: ViewerNotesScope;
  keep: string;
  onAnswer: (
    sketch: string,
    answer: SketchAnswer,
  ) => void /** Takes it back without answering; Withdraw shows where this is passed. */;
  onWithdraw?: (
    sketch: string,
  ) => void /** Opens it in the side panel, bigger, as the thread card's press does. */;
  onOpen?: (
    sketch: string,
  ) => void /** Each drawing kept under the title, oldest first, the newest `scene`: offered at the header's right. */;
  revisions?: readonly Scene[];
};

/**
 * A sketch put to the person, as the thread's last card where the plan to approve stands: the drawing with the canvas's own pan, zoom
 * and fit and his notes, and under it Approve and Send back, which are `SketchRow`'s. It is the thread's, so it goes when he answers.
 * **A chevron folds it to its header line**, the glyph, the title and the bell with the keys at its end, as the plan folds to its band.
 * Folded, Send back opens the note under that line, and the drawing stays folded.
 */
export function SketchPin({
  sketch,
  title,
  scene,
  against,
  notes,
  keep,
  onAnswer,
  onWithdraw,
  onOpen,
  revisions,
}: SketchPinProps) {
  const [folded, setFolded] = useState(false);
  const { picked, menu } = useSketchRevision(revisions, "end");
  const shown = sketchAtRevision(revisions, picked, {
    scene,
    ...(against === undefined
      ? {}
      : {
          against,
        }),
  });
  const Fold = folded ? ChevronRight : ChevronDown;
  const said = folded ? "Unfold" : "Fold";
  return (
    <Card
      flat
      className="armada-session-thread__ask armada-session-sketch-pin"
      role="article"
      aria-label={`Sketch to approve: ${title}`}
      data-folded={folded || undefined}
    >
      <div className="armada-session-thread__plan-head">
        <span className="armada-session-sketch-pin__band">
          <PencilRuler size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-thread__eyebrow">{title}</span>
          <Tooltip label="Sketch waiting for approval">
            <span
              className="armada-session-thread__settled"
              role="img"
              aria-label="Sketch waiting for approval"
              data-state="waiting"
            >
              <BellRing size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span className="armada-session-sketch-pin__tools">
            {menu}
            {onOpen === undefined ? null : (
              <Tooltip label="Open in the panel">
                <Button
                  className="armada-session-sketch-pin__open"
                  variant="ghost"
                  size="sm"
                  iconOnly
                  aria-label="Open in the panel"
                  onClick={() => onOpen(sketch)}
                >
                  <Maximize2 size={12} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            )}
            <Tooltip label={said}>
              <Button
                className="armada-session-sketch-pin__fold"
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={said}
                aria-expanded={!folded}
                onClick={() => setFolded(!folded)}
              >
                <Fold size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        </span>
      </div>
      {folded ? null : (
        <div className="armada-session-sketch-pin__read">
          <SketchReading
            key={shown.revision}
            scene={shown.scene}
            title={title}
            {...(shown.against === undefined
              ? {}
              : {
                  against: shown.against,
                })}
            {...(notes === undefined || shown.earlier
              ? {}
              : {
                  notes,
                })}
          />
        </div>
      )}
      <div className="armada-session-sketch__answers">
        <ApprovalAnswers
          id={sketch}
          keep={keep}
          open
          onAnswer={onAnswer}
          {...(onWithdraw === undefined
            ? {}
            : {
                onWithdraw,
              })}
        />
      </div>
    </Card>
  );
}
