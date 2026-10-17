// Dispatch: the words a request is made of before anything is started — staged pictures, `@` paths
// and the repository the words are about.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

/**
 * Which repository a request's words are about (`read_request_repository`). `manifest_id` is set only
 * where exactly one repository is clearly meant, and `null` is "could not tell": ask the person before
 * anything is built. `likely` is every set-up repository, best first, and empty where nothing could be read.
 */
export type RequestRepositoryRead = { manifest_id: string | null; likely: string[] };

export type DispatchApi = {
  /**
   * Write pasted or picked bytes to a staging file before a Session exists —
   * there is no Session id yet to key storage on. Returns the absolute path
   * written, which the caller carries until the first message sends it.
   */
  stageAttachment: (
    bytes: ArrayBuffer,
    filename: string,
    mimeType: string,
  ) => Promise<{ path: string }>;
  /**
   * Paths under the checkout narrowed against typed text, for the `@` mention
   * popup. **Empty rather than a fault**, on a call that could not be made or
   * on nothing connected — a person is typing, and a toast over a popup they
   * may not even have open would be Bridge announcing a failure nobody asked
   * to hear about.
   */
  /** `manifestId` searches that repository rather than the one the window has picked: a Session's own. */
  searchFiles: (query: string, manifestId?: string) => Promise<string[]>;
  /**
   * Which repository the words typed so far are about, with no model call: a ticket link, a name, a
   * path, or the only repository there is. **Never a fault**, for `searchFiles`' reason — it runs on a
   * pause in typing — so nothing connected, or a Fleet that would not answer, reads as no pick and no
   * ranking. Only the first 16 000 characters are read, since the words ride the query string.
   */
  readRequestRepository: (words: string) => Promise<RequestRepositoryRead>;
};

export const DISPATCH_CHANNELS = {
  stageAttachment: "bridge:stage-attachment",
  searchFiles: "bridge:search-files",
  readRequestRepository: "bridge:read-request-repository",
} as const;
