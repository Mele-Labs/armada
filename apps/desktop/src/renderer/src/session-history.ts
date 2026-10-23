// A Job's attachment history as the Artifacts graph reads it: `get_attachment_history`, drawn the way the ledger draws each attachment.

import { versionsOfHistory } from "@armada/screens/src/sessions-wire";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { BridgeApi } from "../../shared/api";

/** Undefined where Fleet could not answer: the graph then draws what it can with no versions. */
export async function readHistory(api: Pick<BridgeApi, "readAttachmentHistory">, id: string): Promise<NonNullable<Session["versions"]> | undefined> {
  const read = await api.readAttachmentHistory(id);
  return read.ok ? versionsOfHistory(read.value) : undefined;
}
