// What this window holds of how repositories run and where that is kept, read from Fleet and changed through it, and which repositories have shown their
// first Job's Armada configuration card. Fleet has no flag for a card shown, so that one is remembered here, on this machine.

import { useSyncExternalStore } from "react";
import type { RunsPlace, SyncedFolder } from "@armada/components";
import type { DropRunFact, RunMemory, RunMemoryPlace, SetRunMemoryPlace } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import type { RepositoryRuns } from "@armada/screens/src/draft/sessions";

import type { BridgeApi } from "../../shared/api";
import { factOf, placeOf, runsOf, wireOfPlace } from "./run-places";

/** What the book asks of Fleet. */
export type RunBookApi = Pick<BridgeApi, "getRunMemory" | "getRunMemoryPlace" | "setRunMemoryPlace" | "dropRunFact">;

/** Fleet lists none: a synced folder is chosen with the folder dialog and named by where it is. */
export const NO_SYNCED: readonly SyncedFolder[] = [];
export const NO_PROFILES: readonly string[] = [];

const KEY = "armada.bridge.first-jobs";

/** The repositories that have had a first Job, each with the Job that carried the card. `null` is a repository that was already in use when this was first read. */
type Firsts = Record<string, string | null>;

/** What was read, so a Job that draws often does not parse it each time. */
let held: Firsts | undefined;

function readFirsts(): Firsts {
  if (held !== undefined) return held;
  try {
    const parsed: unknown = JSON.parse(window.localStorage.getItem(KEY) ?? "{}");
    held = typeof parsed === "object" && parsed !== null ? (parsed as Firsts) : {};
  } catch {
    held = {};
  }
  return held;
}

function writeFirsts(next: Firsts): void {
  held = next;
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // A failed write leaves the card to show again, which is the honest default.
  }
}

/**
 * Whether this Job carries its repository's card. The first Job on a repository that has none yet takes it, and keeps it until it has sent its first message;
 * every Job after, and this one once it has sent, shows the place as a glyph.
 */
export function cardOn(root: string, job: string, sent: boolean): boolean {
  const firsts = readFirsts();
  if (!(root in firsts)) {
    // A Job with something sent in it belongs to a repository already in use: no card to show for it, now or later.
    writeFirsts({ ...firsts, [root]: sent ? null : job });
    return !sent;
  }
  return firsts[root] === job && !sent;
}

export class RunBook {
  private readonly api: RunBookApi;
  private readonly memories = new Map<string, RunMemory>();
  private readonly places = new Map<string, RunMemoryPlace>();
  private readonly failed = new Map<string, string>();
  private readonly asked = new Set<string>();
  private readonly listeners = new Set<() => void>();
  private version = 0;

  constructor(api: RunBookApi) {
    this.api = api;
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  /** Changes whenever anything the book holds does. */
  snapshot = (): number => this.version;

  private changed(): void {
    this.version += 1;
    this.listeners.forEach((one) => one());
  }

  /** Where a repository keeps its runs, once Fleet has said. The first ask for a repository reads it. */
  placeOf(root: string): RunsPlace | undefined {
    const held = this.places.get(root);
    if (held === undefined && !this.asked.has(root)) {
      this.asked.add(root);
      void this.readPlace(root);
    }
    return held === undefined ? undefined : placeOf(held, NO_SYNCED);
  }

  /** All of how a repository runs, as last read, or the words Fleet refused the read in. `name` is the repository as the picker calls it. */
  runsOf(root: string, name: string): RepositoryRuns | undefined {
    const memory = this.memories.get(root);
    if (memory !== undefined) return runsOf(name, memory, NO_SYNCED);
    const said = this.failed.get(root);
    const place = this.places.get(root);
    if (said === undefined) return undefined;
    return { repository: name, source: "", rows: [], place: place === undefined ? { id: "mac" } : placeOf(place, NO_SYNCED), said };
  }

  /** Reads a repository's runs again, for a layer about to show them. */
  async read(root: string): Promise<void> {
    const answer = await this.api.getRunMemory(root);
    if (answer.ok) {
      this.memories.set(root, answer.value);
      this.places.set(root, { repository: root, place: answer.value.place, ...this.folderOf(root) });
      this.failed.delete(root);
    } else {
      this.failed.set(root, refusalWords(answer.outcome));
    }
    this.changed();
  }

  private folderOf(root: string): { synced_folder?: string } {
    const held = this.places.get(root);
    return held?.synced_folder === undefined ? {} : { synced_folder: held.synced_folder };
  }

  private async readPlace(root: string): Promise<void> {
    const answer = await this.api.getRunMemoryPlace(root);
    if (answer.ok) {
      this.places.set(root, answer.value);
      this.changed();
    }
  }

  /**
   * Keeps the runs in `place`. Answers the words of a refusal and leaves the place where it was; answers nothing where Fleet took it.
   */
  async place(root: string, place: RunsPlace): Promise<string | undefined> {
    const body: SetRunMemoryPlace = wireOfPlace(root, place, NO_SYNCED);
    const answer = await this.api.setRunMemoryPlace(body);
    if (!answer.ok) return refusalWords(answer.outcome);
    this.places.set(root, answer.value);
    this.changed();
    // What is kept, and what waits, depend on the place.
    if (this.memories.has(root)) await this.read(root);
    return undefined;
  }

  /** Never keeps this line again. The memory is the one Fleet answered. */
  async drop(root: string, id: string): Promise<string | undefined> {
    const memory = this.memories.get(root);
    const fact = memory === undefined ? undefined : factOf(memory, id);
    if (fact === undefined) return undefined;
    const body: DropRunFact = { repository: root, workspace: fact.workspace, kind: fact.kind, command: fact.command };
    const answer = await this.api.dropRunFact(body);
    if (!answer.ok) return refusalWords(answer.outcome);
    this.memories.set(root, answer.value);
    this.changed();
    return undefined;
  }
}

/** One book for each `window.armada`, so every reader in a window shares what was read. */
const BOOKS = new WeakMap<object, RunBook>();

export function runBook(api: BridgeApi = window.armada): RunBook {
  let book = BOOKS.get(api);
  if (book === undefined) {
    book = new RunBook(api);
    BOOKS.set(api, book);
  }
  return book;
}

/** The window's book, drawn again when anything in it changes. */
export function useRunBook(): RunBook {
  const book = runBook();
  useSyncExternalStore(book.subscribe, book.snapshot);
  return book;
}

/** A fresh window remembers no card shown. The mock calls it when it mounts a scenario. */
export function forgetFirsts(): void {
  held = undefined;
  try {
    window.localStorage.removeItem(KEY);
  } catch {
    // Nothing was kept.
  }
}
