// The Sessions box under Helm's thread: what `/`, `@` and `#` offer, the model and effort it asks on,
// and the pad a sketch is drawn on. Built from the Sessions the window already holds, so Helm's box
// offers exactly what a Session's does, with the repository Helm answers for as the one `#` reads.
//
// **Nothing here is Fleet's yet.** Helm's ask is words and context; the model and effort are kept for
// this window, and a picture, a sketch or a tag reaches only the Session a thought is sent to.

import { useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import type { HelmBox } from "@armada/components";
import type { DrawnSketch, SessionTag } from "@armada/screens/src/draft/sessions";

import { useNumbered } from "./numbered";
import { SketchSheet } from "./sessions";
import { useSessions, useSessionsDraft } from "./sessions-draft";

type Tuning = { model: string | null; effort: string | null };
const AUTO: Tuning = { model: null, effort: null };

export type HelmBoxHost = {
  /** The box's parts. Absent where Sessions are not served, and the box then offers only what is typed. */
  box: Partial<HelmBox> | undefined;
  /** The pad, to be drawn once beside the dock. */
  pad: ReactNode;
  /** Sketches drawn here, by id, for a thought that carries them to a Session. */
  sketchesOf: (ids: readonly string[]) => DrawnSketch[];
};

export function useHelmBox(manifestId: string | undefined): HelmBoxHost {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const numbered = useNumbered(manifestId);
  const [tuned, setTuned] = useState<Record<string, Tuning>>({});
  const [drawn, setDrawn] = useState<DrawnSketch[]>([]);
  const [padOpen, setPadOpen] = useState(false);
  const known = useRef(new Map<string, DrawnSketch>());
  const tuning = tuned[manifestId ?? ""] ?? AUTO;

  const box = useMemo((): Partial<HelmBox> | undefined => {
    if (draft === undefined) return undefined;
    return {
      model: tuning.model,
      effort: tuning.effort,
      models: draft.models,
      efforts: draft.efforts,
      onTune: (next) => setTuned((was) => ({ ...was, [manifestId ?? ""]: next })),
      commands: draft.commands,
      taggable: [
        ...sessions.filter((one) => one.dead === undefined && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! })),
        ...draft.taggable(),
      ],
      ...(numbered === undefined ? {} : { numbered }),
      drawn: drawn.map(({ id, title }) => ({ id, title })),
      onDraw: () => setPadOpen(true),
      onRemoveDrawn: (id) => setDrawn((was) => was.filter((one) => one.id !== id)),
    };
  }, [draft, sessions, numbered, tuning, drawn, manifestId]);

  return {
    box,
    pad: (
      <SketchSheet
        open={padOpen}
        onClose={() => setPadOpen(false)}
        onAttach={(sketch) => {
          known.current.set(sketch.id, sketch);
          setDrawn((was) => [...was, sketch]);
          setPadOpen(false);
        }}
      />
    ),
    sketchesOf: (ids) => ids.flatMap((id) => known.current.get(id) ?? []),
  };
}
