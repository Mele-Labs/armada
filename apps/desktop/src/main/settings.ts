// settings.json: read when Fleet connects, replaced whole by `settings.changed` and by every save,
// and published in `BridgeState`. `preferences.ts`' shape, and its own class for the same reason:
// what an act needs is `Board`, so nothing here can drift from what the socket believes.
//
// **Main reads three of the settings itself**, and every publish goes through `publishSettings` so
// none of them can be missed: the command wait, the reconnect interval, and the editor and terminal
// `open.ts` and `terminal.ts` read off the published list.

import type { Outcome, SaveSettings, SettingsList, SettingValue } from "@armada/protocol";
import { ask, followCommandSeconds } from "./request";
import { followReconnectMs } from "./socket";
// Type-only, and therefore not a cycle at runtime.
import type { Board } from "./command";

/** One setting's value in force, by key, or `undefined` where the list is not read or has no such key. */
export function settingOf(list: SettingsList | null, key: string): SettingValue | undefined {
  return list?.settings.find((one) => one.key === key)?.value;
}

/** A text setting, or `""` where it is absent or not text. */
export function textSetting(list: SettingsList | null, key: string): string {
  const value = settingOf(list, key);
  return typeof value === "string" ? value : "";
}

/** Publish a list, and follow the settings main itself acts on. */
export function publishSettings(list: SettingsList, publish: (change: { settings: SettingsList }) => void): void {
  const command = settingOf(list, "timeouts.commandSeconds");
  if (typeof command === "number") followCommandSeconds(command);
  const reconnect = settingOf(list, "timeouts.bridgeReconnectMs");
  if (typeof reconnect === "number") followReconnectMs(reconnect);
  publish({ settings: list });
}

/**
 * Every setting, once per connection. **A failed read publishes nothing**, `readPreferences`'
 * reason: a list that drew as the shipped defaults on a glitch would hide a person's own values.
 */
export async function readSettings(port: number, publish: (change: { settings: SettingsList }) => void): Promise<void> {
  const answer = await ask(port, "GET", "/settings");
  if (answer.ok === true) publishSettings(answer.body as SettingsList, publish);
}

/** One save at a time, out, `Preferring`'s reason: there is no Job to key it on. */
export class Settling {
  private readonly board: Board;
  private saving = false;

  constructor(board: Board) {
    this.board = board;
  }

  /** Change settings by key; `null` removes one. Fleet answers every setting, which is published. */
  async save(changes: SaveSettings["changes"]): Promise<Outcome> {
    if (this.saving) return { ok: false, why: "already_setting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    this.saving = true;
    try {
      const body: SaveSettings = { changes };
      const answer = await ask(port, "POST", "/settings/save", body);
      if (answer.ok !== true) return answer.outcome;
      publishSettings(answer.body as SettingsList, this.board.publish);
      return { ok: true };
    } finally {
      this.saving = false;
    }
  }
}
