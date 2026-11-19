// A Job's artifacts as the phone reads them: Fleet's `JobArtifact` list (`/api/sessions/artifacts`), read once per change
// of the Job and only while one of its screens is open, and the acts that answer a part, press a button and mark a
// revision opened. `crates/ipc/src/artifact_parts.rs`, `docs/concepts/artifact.md`.

import { useEffect, useState } from "react";
import type {
  AnswerDocument,
  AnswerPart,
  ArtifactPart,
  CastPageVote,
  HostToPage,
  JobArtifact,
  MarkArtifactSeen,
  PageMessage,
  PagePress,
  PageToHost,
  PartAction,
  PressPageAction,
  VoteCast,
} from "./wire";
import type { Scene } from "./kit/components/compositions/SketchScene/scene";
import { sceneOfWire } from "./kit/screens/sessions-wire";

import { call, getJson, refusal } from "./client";
import { loadLists, onChange } from "./fleet";
import { bodyOf } from "./jobState";
import type { Machine } from "./machines";

export type ArtifactsRead = {
  artifacts?: readonly JobArtifact[];
  error?: string;
};

/** The Job's artifacts, read again on every change Fleet tells of it, and on `reread`. */
export function useArtifacts(
  machine: Machine | undefined,
  id: string,
): [
  ArtifactsRead,
  () => void,
] {
  const [read, setRead] = useState<ArtifactsRead>({});
  const [epoch, setEpoch] = useState(0);
  useEffect(() => {
    if (machine === undefined) return;
    return onChange((origin, sessionId) => {
      if (origin === machine.origin && (sessionId === undefined || sessionId === id))
        setEpoch((was) => was + 1);
    });
  }, [
    machine?.origin,
    id,
  ]);
  useEffect(() => {
    if (machine === undefined) return;
    let gone = false;
    getJson<JobArtifact[]>(
      machine,
      `/api/sessions/artifacts?session_id=${encodeURIComponent(id)}`,
    ).then(
      (artifacts) =>
        !gone &&
        setRead({
          artifacts,
        }),
      (why) =>
        !gone &&
        setRead((was) => ({
          ...was,
          error: refusal(why, machine),
        })),
    );
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    machine?.deviceId,
    id,
    epoch,
  ]);
  return [
    read,
    () => setEpoch((was) => was + 1),
  ];
}

/** Its parts, in the order the Job added them. A page that holds none of the Job's parts is one page part of itself. */
export function partsOf(artifact: JobArtifact): readonly ArtifactPart[] {
  if (artifact.parts !== undefined && artifact.parts.length > 0) return artifact.parts;
  if (artifact.form !== "page") return [];
  const newest = newestOf(artifact);
  return [
    {
      kind: "page",
      path: artifact.id,
      id: artifact.id,
      asks: artifact.state === "asking",
      state: "open",
      revision: newest,
      drawn: String(newest),
      added: artifact.revisions[0]?.at ?? "",
    },
  ];
}

export const newestOf = (artifact: JobArtifact): number =>
  artifact.revisions[artifact.revisions.length - 1]?.number ?? 1;

/** When its newest revision was kept, which is where it stands in the thread. */
export const keptAt = (artifact: JobArtifact): string =>
  artifact.revisions[artifact.revisions.length - 1]?.at ?? "";

export const opensAsk = (part: ArtifactPart): boolean => part.asks && part.state === "open";

/** Still put to him: any part asks and is not answered. */
export const asks = (artifact: JobArtifact): boolean =>
  artifact.archived === undefined &&
  (artifact.state === "asking" || partsOf(artifact).some(opensAsk));

/**
 * An artifact the thread's older cards do not draw: one of parts the Job added with `add_part`, or several things joined.
 * A lone sketch, page or draft keeps its own card.
 */
export const ofParts = (artifact: JobArtifact): boolean => {
  const parts = artifact.parts ?? [];
  return (
    parts.length > 1 ||
    parts.some((one) => one.kind === "words" || one.kind === "takes" || one.kind === "buttons")
  );
};

/** The scene a drawing or a take carries, read. `undefined` for one that does not draw. */
export const sceneOfPart = (text: string | undefined): Scene | undefined =>
  text === undefined ? undefined : sceneOfWire(text);

/** The take a pick answered, by its title. */
export function pickedTitle(part: ArtifactPart): string | undefined {
  if (part.kind !== "takes") return undefined;
  const take = part.answer?.take;
  return take === undefined
    ? undefined
    : (part.takes.find((one) => one.id === take)?.title ?? take);
}

/** The one line an answered artifact folds to: how it settled, and his words where he gave any. */
export function foldedSaid(artifact: JobArtifact): string {
  const parts = partsOf(artifact);
  const picks = parts.flatMap((part) => {
    const title = pickedTitle(part);
    return title === undefined
      ? []
      : [
          `Picked ${title}`,
        ];
  });
  const head =
    artifact.state === "sent_back"
      ? "Sent back"
      : artifact.state === "withdrawn"
        ? "Withdrawn"
        : artifact.state === "dismissed"
          ? "Dismissed"
          : artifact.state === "reading"
            ? "Shown"
            : "Approved";
  const words =
    artifact.words ??
    parts.find((part) => part.state === "sent_back")?.answer?.words ??
    parts.find((part) => part.answer?.words !== undefined)?.answer?.words;
  return [
    picks.length > 0 && artifact.state !== "sent_back" ? picks.join(", ") : head,
    ...(words === undefined || words.trim() === ""
      ? []
      : [
          words.trim(),
        ]),
  ].join(": ");
}

/** Fleet's refusals to say the part changed after he read it, read by code. */
const CHANGED: Record<string, string> = {
  "fleet.part_redrawn": "The Job put this part again. The new one is shown.",
};

/** Answers null when it went, else the sentence to show. A part put again is read again, and its sentence says so. */
async function attempted(
  machine: Machine,
  act: () => Promise<unknown>,
  reread: () => void,
): Promise<string | null> {
  try {
    await act();
    reread();
    void loadLists(machine.origin);
    return null;
  } catch (why) {
    const { code, message } = bodyOf(why);
    if (code !== undefined && code in CHANGED) {
      reread();
      return CHANGED[code]!;
    }
    return message ?? refusal(why, machine);
  }
}

export function partActs(machine: Machine, id: string, artifact: JobArtifact, reread: () => void) {
  const post = (path: string, body: unknown) =>
    call(machine, path, {
      method: "POST",
      body,
    });
  return {
    answer: (
      part: ArtifactPart,
      how: AnswerPart["how"],
      given: Pick<AnswerPart, "take" | "words" | "notes"> = {},
    ) => {
      // A page that holds no parts is answered as a document, at the revision he read.
      if ((artifact.parts ?? []).length === 0 && part.kind === "page") {
        const page: AnswerDocument = {
          session_id: id,
          document: `document:${artifact.id}`,
          revision: part.drawn,
          how: how === "sent_back" ? "sent_back" : "approved",
          ...(given.words === undefined
            ? {}
            : {
                words: given.words,
              }),
        };
        return attempted(machine, () => post("/api/sessions/document/answer", page), reread);
      }
      const body: AnswerPart = {
        session_id: id,
        artifact: artifact.id,
        part: part.id,
        drawn: part.drawn,
        how,
        ...given,
      };
      return attempted(machine, () => post("/api/sessions/artifacts/part/answer", body), reread);
    },
    press: (part: ArtifactPart, action: PartAction, words?: string) => {
      const body: PressPageAction = {
        session_id: id,
        artifact: artifact.id,
        revision: part.revision,
        action: action.id,
        ...(words === undefined
          ? {}
          : {
              words,
            }),
      };
      return attempted(machine, () => post("/api/sessions/artifacts/press", body), reread);
    },
  };
}

/** A press or a vote a sealed page posted, made as Fleet's call, and what goes back to the page. */
export async function pageAct(
  machine: Machine,
  id: string,
  said: PageMessage<PageToHost>,
  reread: () => void,
): Promise<PageMessage<HostToPage>> {
  try {
    if (said.kind === "press") {
      const body: PressPageAction = {
        session_id: id,
        artifact: said.artifact,
        revision: said.revision,
        action: said.action,
        ...(said.words === undefined
          ? {}
          : {
              words: said.words,
            }),
      };
      const press = (await (
        await call(machine, "/api/sessions/artifacts/press", {
          method: "POST",
          body,
        })
      ).json()) as PagePress;
      reread();
      return {
        armada: 1,
        kind: "pressed",
        press,
      };
    }
    const body: CastPageVote = {
      session_id: id,
      artifact: said.artifact,
      revision: said.revision,
      vote: said.vote,
      result: said.result,
    };
    const cast = (await (
      await call(machine, "/api/sessions/artifacts/vote", {
        method: "POST",
        body,
      })
    ).json()) as VoteCast;
    reread();
    return {
      armada: 1,
      kind: "voted",
      cast,
    };
  } catch (why) {
    const { code, message } = bodyOf(why);
    return {
      armada: 1,
      kind: "refused",
      ...(said.kind === "press"
        ? {
            action: said.action,
          }
        : {
            vote: said.vote,
          }),
      code: code ?? "pocket.unreachable",
      message: message ?? refusal(why, machine),
    };
  }
}

/** He opened this revision. Fleet only moves `seen` forward, so a mark that fails is left. */
export function markSeen(machine: Machine, id: string, artifact: JobArtifact, revision: number) {
  if ((artifact.seen ?? 0) >= revision) return;
  const body: MarkArtifactSeen = {
    session_id: id,
    artifact: artifact.id,
    revision,
  };
  void call(machine, "/api/sessions/artifacts/seen", {
    method: "POST",
    body,
  }).catch(() => undefined);
}

/** One kept revision of a page, sealed as Fleet serves it: HTML whose one script is the kit's. */
export async function readPage(
  machine: Machine,
  id: string,
  artifact: string,
  revision: number,
): Promise<string> {
  const response = await call(
    machine,
    `/api/sessions/artifacts/page?session_id=${encodeURIComponent(id)}&artifact=${encodeURIComponent(artifact)}&revision=${String(revision)}`,
  );
  return response.text();
}

/** A node with no size of its own is drawn about this big. */
const NODE_W = 160;
const NODE_H = 60;

/**
 * The shape a picture's box takes, width over height, from the scene's own bounds: a wide flow is a short box and a tall
 * rail a tall one, kept between a tall phone screen and a strip, so the fitted drawing fills it.
 */
export function aspectOf(scene: Scene): number {
  if (scene.nodes.length === 0) return 16 / 9;
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const node of scene.nodes) {
    const w = node.w ?? NODE_W;
    const h = node.h ?? (node.kind === "mock" ? w * 0.4 : NODE_H);
    left = Math.min(left, node.x);
    top = Math.min(top, node.y);
    right = Math.max(right, node.x + w);
    bottom = Math.max(bottom, node.y + h);
  }
  const pad = 48;
  return Math.min(3, Math.max(0.6, (right - left + pad) / (bottom - top + pad)));
}
