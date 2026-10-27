// The open pull requests and issues the palette lists, read from Fleet **when the palette opens** and not only when the
// Reviews or Issues tab mounts. Pull requests are the merge lines' hubs, which the window already holds, joined with the
// Reviews lists (requested of him, his own) that Fleet reads on request; issues are each repository's `list_issues`. A
// list is asked for again only once it is a half minute old, as `#` in a message box asks (`numbered-fleet.tsx`).
//
// **The repository is named as the title row's picker names it**, so a word typed for a repository finds its rows, and a
// row opens where its address opens today (`link-routes.ts`). An issue a tracker names by a key rather than a number
// (`ABC-123`) has no number to find it by and is left out.

import { jobIdOf, repositoryLabel } from "@armada/shell";

import type { BridgeApi } from "../../shared/api";
import type { PaletteLinked } from "./palette-fixtures";
import type { StateSource } from "./state-feed";

/** What a list is worth before it is asked for again. */
export const STALE_MS = 30_000;

export type Tickets = { pulls: readonly PaletteLinked[]; issues: readonly PaletteLinked[] };

type Reads = StateSource & Pick<BridgeApi, "listIssues" | "getReviewLists">;

const NONE: Tickets = { pulls: [], issues: [] };
const signature = (rows: readonly PaletteLinked[]): string => rows.map((one) => `${one.repo}\n${one.number}\n${one.title}\n${one.url}`).join("\n\n");

/** The store behind one window's palette rows. Exported for a test that holds it without a window. */
export class PaletteTickets {
  private readonly api: Reads;
  private readonly now: () => number;
  /** The repositories that carry a Manifest id, with the name the picker gives each. */
  private repositories: readonly { id: string; root: string; repo: string }[] = [];
  private hubs: Readonly<Record<string, readonly PaletteLinked[]>> = {};
  private reviews: readonly (PaletteLinked & { manifestId: string })[] = [];
  private issuesOf: Readonly<Record<string, readonly PaletteLinked[]>> = {};
  private askedAt = Number.NEGATIVE_INFINITY;
  private snap: Tickets = NONE;
  private readonly listeners = new Set<() => void>();

  constructor(api: Reads, now: () => number = Date.now) {
    this.api = api;
    this.now = now;
  }

  /** Read the state and follow it. Returns the way to stop. */
  attach(): () => void {
    let live = true;
    void this.api.state().then((state) => live && this.take(state));
    const off = this.api.subscribe((state) => this.take(state));
    return () => {
      live = false;
      off();
    };
  }

  readonly get = (): Tickets => this.snap;
  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  private take(state: Parameters<Parameters<StateSource["subscribe"]>[0]>[0]): void {
    const served = state.holds.repositories ?? [];
    this.repositories = served.flatMap((one) => {
      const id = jobIdOf(one);
      return id === undefined ? [] : [{ id, root: one.root, repo: repositoryLabel(one, served) }];
    });
    const hubs: Record<string, readonly PaletteLinked[]> = {};
    for (const line of state.mergeLines?.lines ?? []) {
      const repository = this.repositories.find((one) => one.root === line.root);
      if (repository === undefined) continue;
      hubs[repository.id] = (line.hub?.pull_requests ?? []).map((one) => ({ repo: repository.repo, number: one.number, title: one.title, url: one.url }));
    }
    this.hubs = hubs;
    this.rebuild();
  }

  /** The palette opened: read what is stale. A refusal keeps what was held. */
  load(): void {
    if (this.now() - this.askedAt <= STALE_MS) return;
    this.askedAt = this.now();
    void this.api
      .getReviewLists()
      .then((read) => {
        if (!read.ok) return;
        const seen = new Set<string>();
        this.reviews = [...read.value.requested, ...read.value.mine, ...read.value.assigned, ...read.value.team].flatMap((row) => {
          const repository = this.repositories.find((one) => one.id === row.manifest_id);
          if (repository === undefined || seen.has(`${row.manifest_id}:${row.number}`)) return [];
          seen.add(`${row.manifest_id}:${row.number}`);
          return [{ manifestId: row.manifest_id, repo: repository.repo, number: row.number, title: row.title, url: row.address }];
        });
        this.rebuild();
      })
      .catch(() => undefined);
    for (const repository of this.repositories) {
      void this.api
        .listIssues(repository.id, false)
        .then((read) => {
          if (!read.ok) return;
          this.issuesOf = {
            ...this.issuesOf,
            [repository.id]: read.value.issues.flatMap((one) => {
              const number = /^#(\d+)$/.exec(one.reference)?.[1];
              return number === undefined ? [] : [{ repo: repository.repo, number: Number(number), title: one.title, url: one.url }];
            }),
          };
          this.rebuild();
        })
        .catch(() => undefined);
    }
  }

  private rebuild(): void {
    const pulls: PaletteLinked[] = [];
    for (const repository of this.repositories) {
      const hub = this.hubs[repository.id] ?? [];
      pulls.push(...hub, ...this.reviews.filter((one) => one.manifestId === repository.id && !hub.some((held) => held.number === one.number)).map((one) => ({ repo: repository.repo, number: one.number, title: one.title, url: one.url })));
    }
    const issues = this.repositories.flatMap((repository) => this.issuesOf[repository.id] ?? []);
    if (signature(pulls) === signature(this.snap.pulls) && signature(issues) === signature(this.snap.issues)) return;
    this.snap = { pulls, issues };
    this.listeners.forEach((one) => one());
  }
}
