// The Sessions box under Helm's thread: what `/`, `@` and `#` offer, the model and effort it asks on,
// and the pad a sketch is drawn on. Built from the Sessions the window already holds, so Helm's box
// offers exactly what a Session's does, with the repository Helm answers for as the one `#` reads.
//
// **Helm is a Session, so its model and effort are the Session's.** A live Helm Session is tuned as any
// Session is (`tune_session`). Before the repository's first message there is none to tune, so the pick
// is held here and goes with that message (`AskHelm.model` and `effort`). What a message carried is a
// Session's own: a file is sent as an upload, a sketch as the picture it renders to.

import { useCallback, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import type { HelmBox } from "@armada/components";
import type { ThoughtCarry } from "@armada/helm";
import type { HelmContext, Outcome, SessionUpload } from "@armada/protocol";
import type { HelmCarry } from "../../shared/api/helm";
import type { DrawnSketch, SessionTag } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { useHelmSession } from "./helm-session";
import { searchFiles } from "./commands";
import { useNumbered } from "./numbered";
import { uploadOfFile, uploadOfSketch, wireTagOf } from "./session-uploads";
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
  /** What a thought went with, as the wire takes it: files read, sketches drawn to pictures, tags as sent. */
  carryOf: (carry: ThoughtCarry) => Promise<HelmCarry>;
  /** Words for Helm with what went with them. Uploads are read here; the answer is main's. */
  ask: (text: string, context: HelmContext, carry?: ThoughtCarry) => Promise<Outcome>;
};

export function useHelmBox(helm: BridgeState["helm"]): HelmBoxHost {
  const manifestId = helm.state === "none" ? undefined : helm.manifestId;
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const session = useHelmSession(manifestId);
  const numbered = useNumbered(manifestId);
  // The pick made before the repository has a Helm Session, kept until its first message takes it.
  const [chosen, setChosen] = useState<{ manifestId: string | undefined; tuning: Tuning } | undefined>();
  const [drawn, setDrawn] = useState<DrawnSketch[]>([]);
  const [padOpen, setPadOpen] = useState(false);
  const known = useRef(new Map<string, DrawnSketch>());
  const before = chosen !== undefined && chosen.manifestId === manifestId ? chosen.tuning : undefined;
  const tuning: Tuning = session !== undefined ? { model: session.model ?? null, effort: session.effort ?? null } : (before ?? AUTO);

  const tune = useCallback(
    (next: Tuning): void => {
      if (session === undefined) return setChosen({ manifestId, tuning: next });
      draft?.tune(session.id, { ...next, mode: session.mode ?? "auto" });
    },
    [draft, session, manifestId],
  );

  const box = useMemo((): Partial<HelmBox> | undefined => {
    if (draft === undefined) return undefined;
    return {
      model: tuning.model,
      effort: tuning.effort,
      models: draft.models,
      efforts: draft.efforts,
      onTune: tune,
      commands: draft.commands,
      taggable: [
        ...sessions.filter((one) => one.dead === undefined && one.helm !== true && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! })),
        ...draft.taggable(),
      ],
      searchFiles,
      ...(numbered === undefined ? {} : { numbered }),
      drawn: drawn.map(({ id, title }) => ({ id, title })),
      onDraw: () => setPadOpen(true),
      onRemoveDrawn: (id) => setDrawn((was) => was.filter((one) => one.id !== id)),
    };
  }, [draft, sessions, numbered, tuning.model, tuning.effort, tune, drawn]);

  const sketchesOf = useCallback((ids: readonly string[]) => ids.flatMap((id) => known.current.get(id) ?? []), []);

  const carryOf = useCallback(
    async (carry: ThoughtCarry): Promise<HelmCarry> => {
      const files = (await Promise.all(carry.files.map(uploadOfFile))).filter((one): one is SessionUpload => one !== undefined);
      const sketches = sketchesOf(carry.sketches.map((one) => one.id));
      const pictured = await Promise.all(sketches.map(uploadOfSketch));
      const attachments = [...files, ...pictured];
      const mentions = carry.tags.map(wireTagOf);
      return {
        ...(attachments.length === 0 ? {} : { attachments }),
        ...(mentions.length === 0 ? {} : { mentions }),
      };
    },
    [sketchesOf],
  );

  const ask = useCallback(
    async (text: string, context: HelmContext, carry?: ThoughtCarry): Promise<Outcome> => {
      const wire = carry === undefined ? {} : await carryOf(carry);
      // Only the message that starts the Session reads these; a live one ignores them.
      const start: HelmCarry = {
        ...(before?.model == null ? {} : { model: before.model }),
        ...(before?.effort == null ? {} : { effort: before.effort }),
      };
      const sent: HelmCarry = { ...wire, ...(session === undefined ? start : {}) };
      return Object.keys(sent).length === 0 ? await window.armada.askHelm(text, context) : await window.armada.askHelm(text, context, sent);
    },
    [carryOf, before, session],
  );

  return {
    box,
    carryOf,
    ask,
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
    sketchesOf,
  };
}
