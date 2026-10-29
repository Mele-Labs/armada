import type { ReactNode } from "react";
import { ProseAttachments } from "@armada/components";

// Main fetches; the renderer's CSP reaches nowhere. One function, so every Prose under it can draw an attachment.
const fetchAttachment = (address: string) => window.armada.fetchAttachment(address);

/** Provided at `App`'s root, and again at the side panel's host, which sits beside `App` and not inside it. */
export function AttachmentsProvided({ children }: { children: ReactNode }) {
  return <ProseAttachments.Provider value={fetchAttachment}>{children}</ProseAttachments.Provider>;
}
