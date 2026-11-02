// Copy debug report's text as a window reads it: Fleet over the connection with the selected repository's root, and where that
// fails or Fleet is not connected, `armada report` run by main. Either text gets Bridge's own protocol on its last line.

import { PROTOCOL_ID, spoken } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import type { DebugRead } from "./debug-report/debug-report";
import type { StateSource } from "./state-feed";

/** The text with `bridge protocol <id>  ` ahead of its last line, which is `fleet protocol <id>  taken <time>` or only `taken <time>`. */
export function withBridgeProtocol(text: string, bridgeProtocol: string): string {
  const lines = text.trimEnd().split("\n");
  lines[lines.length - 1] = `bridge protocol ${bridgeProtocol}  ${lines[lines.length - 1]}`;
  return lines.join("\n");
}

export function createFleetDebugReport(api: Pick<BridgeApi, "getDebugReport" | "runArmadaReport">, state: StateSource, bridgeProtocol: string = spoken(PROTOCOL_ID)): DebugRead {
  return async () => {
    // The pick is main's, as the other reads take it, and `null` is All repositories.
    const { repository } = await state.state();
    const fleet = await api.getDebugReport(repository);
    if (fleet.ok) return withBridgeProtocol(fleet.value.text, bridgeProtocol);
    const ran = await api.runArmadaReport(repository);
    if (!ran.ok) throw new Error(ran.why);
    return withBridgeProtocol(ran.text, bridgeProtocol);
  };
}
