import { diff, type StateChange } from "../shared/state-change";

/**
 * What each window was last sent, so the next send is what differs from it. A window is any object the caller keys
 * on, a `WebContents` in main; one that goes is forgotten with it.
 */
export class StateSends<S> {
  private readonly held = new WeakMap<object, { sent: S; seq: number }>();

  /** The change that takes `window` from what it holds to `state`, or `null` where it already holds it. */
  next(window: object, state: S): StateChange<S> | null {
    const held = this.held.get(window);
    if (held === undefined) return this.full(window, state);
    const patch = diff(held.sent, state);
    if (patch === undefined) return null;
    const from = held.seq;
    held.sent = state;
    held.seq += 1;
    return { patch, from, to: held.seq };
  }

  /** The whole of `state`, and what `window` is held to hold from here. */
  full(window: object, state: S): StateChange<S> {
    const seq = (this.held.get(window)?.seq ?? 0) + 1;
    this.held.set(window, { sent: state, seq });
    return { full: state, seq };
  }
}
