// `armada://pull/<owner>/<name>/<n>`, the link that opens Bridge on a pull request's review page. **Behind `features.deepLinks`**: while it
// is off Armada does not claim the scheme from the OS and ignores a link that arrives anyway. The setting is read off the list Fleet publishes
// (`settings.ts`), so a link that arrives first, being what launched the app, is held until the person's switch is known.

import { DEEP_LINK_SCHEME, parsePullLink, type PullLink } from "../shared/pull-link";

export type DeepLinkHost = {
  /** Registers Armada with the OS as the handler of the scheme. */
  claim: () => void;
  /** Gives the scheme back. */
  release: () => void;
  /** Takes a link to the window. */
  deliver: (link: PullLink) => void;
};

export class DeepLinks {
  private readonly host: DeepLinkHost;
  /** `undefined` until the settings are read. */
  private on: boolean | undefined;
  private waiting: PullLink | undefined;

  constructor(host: DeepLinkHost) {
    this.host = host;
  }

  /** The setting as Fleet last said it. Claims or releases the scheme when it changed, and settles a link that was waiting on it. */
  follow(on: boolean): void {
    const changed = this.on !== on;
    this.on = on;
    if (changed) {
      if (on) this.host.claim();
      else this.host.release();
    }
    const waiting = this.waiting;
    this.waiting = undefined;
    if (on && waiting !== undefined) this.host.deliver(waiting);
  }

  /** What the OS handed over. Answers whether Armada took it: it is Armada's link to a pull request, and the setting is not off. */
  arrive(address: string): boolean {
    if (this.on === false || !address.startsWith(`${DEEP_LINK_SCHEME}:`)) return false;
    const link = parsePullLink(address);
    if (link === undefined) return false;
    if (this.on === undefined) this.waiting = link;
    else this.host.deliver(link);
    return true;
  }
}
