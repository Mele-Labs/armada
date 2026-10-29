// A forge attachment's bytes, fetched by main because the renderer's CSP reaches nowhere. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

/** The attachment as the forge served it. */
export type AttachmentFetched = { bytes: Uint8Array; mime: string };

export type AttachmentsApi = {
  /** Null where the address is not a forge one main will fetch, is too big, or the forge refused it. */
  fetchAttachment: (url: string) => Promise<AttachmentFetched | null>;
};

export const ATTACHMENTS_CHANNELS = {
  fetchAttachment: "bridge:fetch-attachment",
} as const;
