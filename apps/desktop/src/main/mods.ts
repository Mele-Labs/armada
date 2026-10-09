// The mods on this machine: read when Fleet connects, replaced whole by `mods.changed`, and the
// three acts Bridge makes on one. `docs/concepts/mods.md`.
//
// **Bridge keeps no copy of a mod's files.** The list says which mods exist and whether each may
// be drawn; the stylesheet itself is asked for at the moment it is applied, so what is injected is
// what Fleet just checked.

import type { ModChecked, ModList, ModPromoted, ModSummary, Outcome } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class Modding {
  private readonly publish: (change: Partial<BridgeState>) => void;
  private readonly port: () => number | null;
  private held: ModList | null = null;

  constructor(publish: (change: Partial<BridgeState>) => void, port: () => number | null) {
    this.publish = publish;
    this.port = port;
  }

  /** The list, once per connection. A failed read publishes nothing, so a list already held stays. */
  async read(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/mods");
    if (answer.ok === true) this.listed(answer.body as ModList);
  }

  /** The list as `mods.changed` carries it, which replaces what is held. */
  listed(list: ModList): void {
    this.held = { mods: list.mods };
    this.publish({ mods: this.held });
  }

  /** Fleet's check of one mod. `null` where Fleet cannot be asked or knows no such mod. */
  async validate(name: string): Promise<ModChecked | null> {
    const port = this.port();
    if (port === null) return null;
    const answer = await ask(port, "GET", `/mods/validate?name=${encodeURIComponent(name)}`);
    return answer.ok === true ? (answer.body as ModChecked) : null;
  }

  /** This machine's switch. The answered row is folded in at once, and `mods.changed` follows. */
  async setEnabled(name: string, enabled: boolean): Promise<Outcome> {
    const port = this.port();
    if (port === null) return NOT_CONNECTED;
    const answer = await ask(port, "POST", "/mods/enable", { name, enabled });
    if (answer.ok !== true) return answer.outcome;
    this.replaced(answer.body as ModSummary);
    return { ok: true };
  }

  /** Put a valid mod on a new branch of the first repository Fleet serves. */
  async promote(name: string): Promise<Outcome> {
    const port = this.port();
    if (port === null) return NOT_CONNECTED;
    const answer = await ask(port, "POST", "/mods/promote", { name });
    if (answer.ok !== true) return answer.outcome;
    return { ok: true, modPromoted: answer.body as ModPromoted };
  }

  private replaced(row: ModSummary): void {
    if (this.held === null) return;
    this.listed({ mods: this.held.mods.map((one) => (one.name === row.name ? row : one)) });
  }
}
