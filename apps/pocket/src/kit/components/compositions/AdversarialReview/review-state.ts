// What a Judge's notes are drawn from, apart from their components so a module built without JSX can read it.

/**
 * A Judge reads a thing and says what it missed. **It knows nothing of what it read**: a plan, a diff or a document. The host
 * hands the notes and the callbacks, and they are pinned to the parts of the thing they are about (`AnchoredFindings.tsx`).
 *
 * **A finding is a claim**: the consequence heads it, then Expected and Found, each clamped to two lines. Its acts sit in
 * one column, so a list of them reads down. A finding sent or dismissed folds to one muted line. No count of findings is
 * drawn anywhere.
 */
export type Finding = {
  id: string;
  /** What breaks, in a line. Heads the finding. */
  consequence: string;
  expected?: string;
  found?: string;
  /** What a person using the thing sees or cannot do because of it. */
  impact?: string;
  /** What it cites, as `path:line`. */
  cites?: readonly string[];
  /** What it is about: a task id, a group id, or nothing for the plan as a whole. A host that draws the plan pins the finding there (`AnchoredFindings`). */
  about?: string;
  /** Acted on: the finding folds to a line. */
  settled?: "sent" | "dismissed";
};
