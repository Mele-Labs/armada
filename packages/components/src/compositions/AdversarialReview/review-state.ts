// The state an adversarial review is drawn from, apart from its components so a module built without JSX can read it.

/**
 * An adversary reads a thing and says how it breaks. **It knows nothing of what it read**: a plan today, a diff or a
 * design tomorrow. The host hands it a state and the callbacks: the press (`AdversarialReviewPress`) stands in a row of
 * acts, and the findings are pinned to the parts of the thing they are about (`AnchoredFindings.tsx`).
 *
 * **A finding is a claim**, drawn as a judge's concern is: the consequence heads it, then Expected and Found, each
 * clamped to two lines. Its acts sit in one column, so a list of them reads down. A finding sent or dismissed folds
 * to one muted line. No count of findings is drawn anywhere.
 */
export type Finding = {
  id: string;
  /** What breaks, in a line. Heads the finding. */
  consequence: string;
  expected: string;
  found: string;
  /** What it cites, as `path:line`. */
  cites?: readonly string[];
  /** What it is about: a task id, a group id, or nothing for the plan as a whole. A host that draws the plan pins the finding there (`AnchoredFindings`). */
  about?: string;
  /** Acted on: the finding folds to a line. */
  settled?: "sent" | "dismissed";
};

export type AdversarialReviewState =
  | { state: "idle" }
  | { state: "running" }
  /** No findings is an empty list: one quiet mark and no sentence. */
  | { state: "done"; findings: readonly Finding[] }
  /** Fleet's words, as it said them. */
  | { state: "failed"; said: string }
  /** The review read an older version of the thing. `read` is that version; its findings stay where they are. */
  | { state: "stale"; read: number; findings?: readonly Finding[] };
