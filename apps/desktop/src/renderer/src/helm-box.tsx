// The Sessions box under Helm's thread: what `/`, `@` and `#` offer, the model and effort it asks on,
// and the pad a sketch is drawn on. Built from the Sessions the window already holds, so Helm's box
// offers exactly what a Session's does, with the repository Helm answers for as the one `#` reads.
//
// **The conversation's own facts are Fleet's.** The model and effort and the names `/` lists ride the
// thread as `facts`; picking one is `tune_helm`, and the facts that come back replace what was shown
// meanwhile. What a message carried is the `asked` row's: a file is sent as an upload, a sketch as the
// picture it renders to, and a picture on a sent row is read back once (`get_helm_file`).

import { useCallback, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";

import type { HelmBox } from "@armada/components";
import type { HelmKept, ThoughtCarry } from "@armada/helm";
import type { HelmBoxFacts, HelmContext, Outcome, SessionUpload } from "@armada/protocol";
import type { DrawnSketch, SessionCommand, SessionTag } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../shared/bridge";
import { useNumbered } from "./numbered";
import { uploadOfFile, uploadOfSketch, wireTagOf } from "./session-uploads";
import { SketchSheet } from "./sessions";
import { useSessions, useSessionsDraft } from "./sessions-draft";
import { askToTell } from "./tell";

type Tuning = { model: string | null; effort: string | null };

/** What a conversation's facts say it runs on. Absent is Auto. */
const tuningOf = (facts: HelmBoxFacts | undefined): Tuning => ({ model: facts?.model ?? null, effort: facts?.effort ?? null });

/** The names `/` lists. Fleet has no sentence for each, so none is drawn. */
const NO_COMMANDS: readonly SessionCommand[] = [];

export type HelmBoxHost = {
  /** The box's parts. Absent where Sessions are not served, and the box then offers only what is typed. */
  box: Partial<HelmBox> | undefined;
  /** The pad, to be drawn once beside the dock. */
  pad: ReactNode;
  /** Sketches drawn here, by id, for a thought that carries them to a Session. */
  sketchesOf: (ids: readonly string[]) => DrawnSketch[];
  /** Words for Helm with what went with them. Uploads are read here; the answer is main's. */
  ask: (text: string, context: HelmContext, carry?: ThoughtCarry) => Promise<Outcome>;
  /** How a sent row draws the files it carried. */
  kept: HelmKept;
};

export function useHelmBox(helm: BridgeState["helm"]): HelmBoxHost {
  const manifestId = helm.state === "none" ? undefined : helm.manifestId;
  const facts = helm.state === "open" || helm.state === "failed" ? helm.facts : undefined;
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const numbered = useNumbered(manifestId);
  // A pick is shown at once and kept until Fleet's facts replace it, which is a new object on the thread.
  const [picked, setPicked] = useState<{ manifestId: string | undefined; facts: HelmBoxFacts | undefined; tuning: Tuning } | undefined>();
  const [drawn, setDrawn] = useState<DrawnSketch[]>([]);
  const [padOpen, setPadOpen] = useState(false);
  const known = useRef(new Map<string, DrawnSketch>());
  // The sketches sent in this window, by the name of the picture they went as. Fleet holds only the picture.
  const sent = useRef(new Map<string, { id: string; title: string }>());
  const pictures = useRef(new Map<string, string>());
  const reading = useRef(new Set<string>());
  const [pictured, drawAgain] = useState(0);
  const tuning = picked !== undefined && picked.manifestId === manifestId && picked.facts === facts ? picked.tuning : tuningOf(facts);
  const names = facts?.commands;
  const commands = useMemo((): readonly SessionCommand[] => names?.map((name) => ({ name, says: "" })) ?? NO_COMMANDS, [names]);

  const tune = useCallback(
    (next: Tuning): void => {
      setPicked({ manifestId, facts, tuning: next });
      void window.armada.tuneHelm(manifestId, next.model, next.effort).then((done) => {
        if (done.ok) return setPicked({ manifestId, facts, tuning: tuningOf(done.facts) });
        setPicked(undefined);
        askToTell(refusalWords(done.outcome));
      });
    },
    [manifestId, facts],
  );

  const box = useMemo((): Partial<HelmBox> | undefined => {
    if (draft === undefined) return undefined;
    return {
      model: tuning.model,
      effort: tuning.effort,
      models: draft.models,
      efforts: draft.efforts,
      onTune: tune,
      commands,
      taggable: [
        ...sessions.filter((one) => one.dead === undefined && one.title !== undefined).map((one): SessionTag => ({ kind: "session", id: one.id, title: one.title! })),
        ...draft.taggable(),
      ],
      ...(numbered === undefined ? {} : { numbered }),
      drawn: drawn.map(({ id, title }) => ({ id, title })),
      onDraw: () => setPadOpen(true),
      onRemoveDrawn: (id) => setDrawn((was) => was.filter((one) => one.id !== id)),
    };
  }, [draft, sessions, numbered, tuning, tune, commands, drawn]);

  const sketchesOf = useCallback((ids: readonly string[]) => ids.flatMap((id) => known.current.get(id) ?? []), []);

  const ask = useCallback(
    async (text: string, context: HelmContext, carry?: ThoughtCarry): Promise<Outcome> => {
      if (carry === undefined) return await window.armada.askHelm(text, context);
      const files = (await Promise.all(carry.files.map(uploadOfFile))).filter((one): one is SessionUpload => one !== undefined);
      const sketches = sketchesOf(carry.sketches.map((one) => one.id));
      const pictured = await Promise.all(sketches.map(uploadOfSketch));
      pictured.forEach((upload, at) => sent.current.set(upload.name, { id: sketches[at]!.id, title: sketches[at]!.title }));
      const attachments = [...files, ...pictured];
      const mentions = carry.tags.map(wireTagOf);
      return await window.armada.askHelm(text, context, {
        ...(attachments.length === 0 ? {} : { attachments }),
        ...(mentions.length === 0 ? {} : { mentions }),
      });
    },
    [sketchesOf],
  );

  const kept = useMemo(
    (): HelmKept => ({
      picture: (fileId) => {
        if (manifestId === undefined) return undefined;
        const key = `${manifestId}\n${fileId}`;
        const held = pictures.current.get(key);
        if (held !== undefined || reading.current.has(key)) return held;
        reading.current.add(key);
        void window.armada.readHelmFile(manifestId, fileId).then((got) => {
          if (!got.ok) return;
          pictures.current.set(key, URL.createObjectURL(new Blob([got.bytes as BlobPart], { type: got.type })));
          drawAgain((was) => was + 1);
        });
        return undefined;
      },
      sketch: (fileName) => sent.current.get(fileName),
    }),
    // A picture read is a new object, so the dock draws its rows again.
    [manifestId, pictured],
  );

  return {
    box,
    ask,
    kept,
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
