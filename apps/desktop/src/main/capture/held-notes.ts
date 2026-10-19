// The notes taken on a Job's walk, held in main until the owner sends them as one turn — the owner, 19 Oct 2026:
// "leave batch annotations on a walk and then submit them back to the agent all at once".
// `docs/practices/capture-window.md`, *Held notes*.
//
// One small JSON file in `userData`, as `window-state.ts` keeps the window's place: read once, written whole on
// every change. A note belongs to the Job, not to the walk it was taken on, so it outlives the walk and the window.

import { randomUUID } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

import type { SendSessionNote, SessionNote } from "@armada/protocol";

import type { LandsOn } from "./layout";

/** A note held on a Job: `send_session_notes`' entry for it, and when it was taken. */
export type HeldNote = { id: string; at: string; note: SessionNote };

type Kept = { jobs: Record<string, HeldNote[]> };

const isHeld = (value: unknown): value is HeldNote => {
  const one = value as Partial<HeldNote> | null;
  return typeof one?.id === "string" && typeof one.at === "string" && typeof one.note?.said === "string" && typeof one.note.place === "object";
};

function read(file: string | null): Map<string, HeldNote[]> {
  const jobs = new Map<string, HeldNote[]>();
  if (file === null) return jobs;
  try {
    const kept = JSON.parse(readFileSync(file, "utf8")) as Partial<Kept>;
    for (const [sessionId, notes] of Object.entries(kept.jobs ?? {})) {
      const held = Array.isArray(notes) ? notes.filter(isHeld) : [];
      if (held.length > 0) jobs.set(sessionId, held);
    }
  } catch {
    // A file that is missing or not this one's: nothing is held.
  }
  return jobs;
}

export class HeldNotes {
  private readonly jobs: Map<string, HeldNote[]>;

  /** `file` null keeps them in memory alone. `changed` is told the Job whose notes moved. */
  constructor(
    private readonly file: string | null,
    private readonly changed: (sessionId: string) => void = () => {},
    private readonly mint: () => string = randomUUID,
    private readonly now: () => string = () => new Date().toISOString(),
  ) {
    this.jobs = read(file);
  }

  of(sessionId: string): readonly HeldNote[] {
    return this.jobs.get(sessionId) ?? [];
  }

  /** The note a save built, held on its Job instead of sent. */
  hold(send: SendSessionNote): HeldNote {
    const { session_id: sessionId, ...note } = send;
    const held: HeldNote = { id: this.mint(), at: this.now(), note };
    this.jobs.set(sessionId, [...this.of(sessionId), held]);
    this.moved(sessionId);
    return held;
  }

  /** New words for one note. **Refused for nothing typed**, which would send an empty note, and for a note not on that Job. */
  edit(sessionId: string, id: string, said: string): boolean {
    const words = said.trim();
    const notes = this.of(sessionId);
    if (words === "" || !notes.some((one) => one.id === id)) return false;
    this.jobs.set(sessionId, notes.map((one) => (one.id === id ? { ...one, note: { ...one.note, said: words } } : one)));
    this.moved(sessionId);
    return true;
  }

  remove(sessionId: string, id: string): boolean {
    const notes = this.of(sessionId);
    if (!notes.some((one) => one.id === id)) return false;
    this.put(sessionId, notes.filter((one) => one.id !== id));
    this.moved(sessionId);
    return true;
  }

  /** The notes Fleet took. **Only those**: one held while the send was out stays for the next. */
  drop(sessionId: string, ids: readonly string[]): void {
    this.put(sessionId, this.of(sessionId).filter((one) => !ids.includes(one.id)));
    this.moved(sessionId);
  }

  private put(sessionId: string, notes: HeldNote[]): void {
    if (notes.length === 0) this.jobs.delete(sessionId);
    else this.jobs.set(sessionId, notes);
  }

  private moved(sessionId: string): void {
    if (this.file !== null) {
      try {
        writeFileSync(this.file, JSON.stringify({ jobs: Object.fromEntries(this.jobs) } satisfies Kept));
      } catch {
        // Held in memory still; a restart loses what could not be written.
      }
    }
    this.changed(sessionId);
  }
}

/** The Job a note saved on this window is held on, and the address it names. `null`: a Studio's capture, or a review's walk no Job holds. */
export function heldOn(landsOn: LandsOn, url: string): { sessionId: string; address: string } | null {
  if ("session" in landsOn) return { sessionId: landsOn.session.id, address: url };
  // A review's `address` is where the change runs, not Bridge's runner the page may be on, so Fleet finds its checkout.
  if ("review" in landsOn && landsOn.review.holder !== undefined) return { sessionId: landsOn.review.holder.id, address: landsOn.review.address };
  return null;
}
