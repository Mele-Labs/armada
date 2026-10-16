// The window's one held copy of Bridge's state, folded from what main sends: the whole of it once, then what moved.
// Everything that reads the state reads it here, so a change is applied once and what it did not touch is the
// object it was.

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { apply, type StateChange } from "../../shared/state-change";

/** What a surface reads the state through: the whole of it, as `BridgeApi.state` and `subscribe` once were. */
export type StateSource = {
  state: () => Promise<BridgeState>;
  subscribe: (onState: (state: BridgeState) => void) => () => void;
};

type Wire = Pick<BridgeApi, "subscribeChanges" | "syncState">;

class Feed implements StateSource {
  private held: { state: BridgeState; seq: number } | undefined;
  private readonly listeners = new Set<(state: BridgeState) => void>();
  private asking: Promise<void> | undefined;
  private started: Promise<void> | undefined;

  constructor(private readonly wire: Wire) {}

  private start(): Promise<void> {
    this.started ??= (() => {
      this.wire.subscribeChanges((change) => this.take(change));
      return this.sync();
    })();
    return this.started;
  }

  /** The whole state, asked for once however many ask: what a window holds nothing of, or a change that did not follow its own. */
  private sync(): Promise<void> {
    this.asking ??= this.wire
      .syncState()
      .then((change) => this.take(change, true))
      .finally(() => {
        this.asking = undefined;
      });
    return this.asking;
  }

  /** `asked`: the change is the answer to `syncState`, which a state held since it was asked for is newer than. */
  private take(change: StateChange<BridgeState>, asked = false): void {
    if ("full" in change) {
      // A whole state no newer than the one held arrived late: the held one is right.
      if (this.held !== undefined && (asked ? change.seq <= this.held.seq : change.seq < this.held.seq)) return;
      this.held = { state: change.full, seq: change.seq };
    } else if (this.held === undefined || change.from > this.held.seq) {
      void this.sync();
      return;
    } else if (change.from < this.held.seq) {
      return;
    } else {
      try {
        this.held = { state: apply(this.held.state, change.patch) as BridgeState, seq: change.to };
      } catch {
        void this.sync();
        return;
      }
    }
    const state = this.held.state;
    this.listeners.forEach((listener) => listener(state));
  }

  state = async (): Promise<BridgeState> => {
    await this.start();
    return this.held!.state;
  };

  subscribe = (onState: (state: BridgeState) => void): (() => void) => {
    this.listeners.add(onState);
    void this.start();
    return () => this.listeners.delete(onState);
  };
}

const FEEDS = new WeakMap<object, Feed>();

/** The one feed over `api`, however many ask. */
export function stateOf(api: Wire): StateSource {
  let feed = FEEDS.get(api);
  if (feed === undefined) {
    feed = new Feed(api);
    FEEDS.set(api, feed);
  }
  return feed;
}

const WHOLE = new WeakMap<object, unknown>();

/** `api` with `state` and `subscribe` as whole states, for what takes the app's API and reads the state through it. */
export function withState<A extends Wire>(api: A): Omit<A, "subscribeChanges" | "syncState"> & StateSource {
  let whole = WHOLE.get(api);
  if (whole === undefined) {
    const feed = stateOf(api);
    whole = { ...api, state: feed.state, subscribe: feed.subscribe };
    WHOLE.set(api, whole);
  }
  return whole as never;
}
