// Helm routes a thought into running work. Fleet reads where the words belong (`read_thought`);
// delivering them is the paths that already exist. A Session gets a message, and a new one opens
// the Dispatch panel. Once the words have arrived Fleet is told, and
// keeps a row for them in Helm's thread, which is where the dock draws it from.

import type { RepositorySummary } from "@armada/protocol";
import { routeTargetOf, wireTargetOf } from "@armada/helm";
import type { RouteTarget, ThoughtCarry, ThoughtReading, ThoughtRouting } from "@armada/helm";
import type { DrawnSketch, SessionTag } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { HelmCarry } from "../../shared/api/helm";
import type { BridgeState } from "../../shared/bridge";
import type { Dispatching } from "./dispatch-panel/dispatching";
import { useSessionsDraft } from "./sessions-draft";
import { askToTell } from "./tell";

type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  dispatching: Dispatching;
  openSession: (id: string) => void;
  /** Sketches drawn in Helm's box, by id: a thought sent to a Session carries them. */
  sketchesOf: (ids: readonly string[]) => DrawnSketch[];
  /** What a thought went with, as the wire takes it, for the row Fleet keeps. */
  carryOf: (carry: ThoughtCarry) => Promise<HelmCarry>;
};

/** The dock's routing: Fleet's reading, and the acts the words may go through. */
export function useThoughtRouting({ state, repositories, dispatching, openSession, sketchesOf, carryOf }: Host): ThoughtRouting {
  const draft = useSessionsDraft();
  // The conversation the dock shows, whose thread keeps the row for words sent on.
  const manifestId = state.helm.state === "none" ? undefined : state.helm.manifestId;

  /** The words reached the Session: Fleet keeps a row for them. A refusal is said, the words having arrived. */
  async function recorded(target: RouteTarget, text: string, carry?: ThoughtCarry): Promise<void> {
    if (manifestId === undefined) return;
    try {
      // What the thought went with is kept on the row, so a reload draws it as it was.
      const sent = carry === undefined || carry.files.length + carry.tags.length + carry.sketches.length === 0 ? undefined : await carryOf(carry);
      const outcome = sent === undefined
        ? await window.armada.recordThoughtSent(manifestId, text, wireTargetOf(target))
        : await window.armada.recordThoughtSent(manifestId, text, wireTargetOf(target), sent);
      if (!outcome.ok) askToTell(refusalWords(outcome));
    } catch {
      askToTell(`Sent to "${target.title}", but the thread could not record it`);
    }
  }

  return {
    read: async (text): Promise<ThoughtReading> => {
      const read = await window.armada.readThought(text);
      // A read that failed is a question: the words go to Helm and are never lost.
      if (!read.ok || read.reading.kind === "question") return { kind: "question" };
      const { target } = read.reading;
      return target === undefined ? { kind: "thought" } : { kind: "thought", match: routeTargetOf(target, repositories) };
    },
    send: async (target, text, carry?: ThoughtCarry) => {
      if (draft === undefined) {
        askToTell(`Not sent to "${target.title}": Jobs are not served`);
        return false;
      }
      // The composer's own send, with what went with the words: a refusal is already a toast, in Fleet's words.
      const arrived = await draft.send(target.id, {
        text,
        files: carry?.files ?? [],
        sketches: sketchesOf(carry?.sketches.map((one) => one.id) ?? []),
        tags: (carry?.tags ?? []) as readonly SessionTag[],
      });
      if (arrived === false) return false;
      await recorded(target, text, carry);
      return true;
    },
    open: (target) => openSession(target.id),
    startNew: (text) => {
      dispatching.choose("session");
      dispatching.openSheet(text);
    },
  };
}
