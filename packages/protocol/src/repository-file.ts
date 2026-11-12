// A file of a Job, read by Fleet for a client that cannot read the agent's disk: `get_repository_file`.
// Hand-mirrored from `crates/ipc/src/repository_file.rs`. The header rules in `protocol.ts` hold here.

/**
 * `get_repository_file` (`GET /sessions/repository_file?session_id=&path=`): the file as text, or as a picture. A file Fleet
 * will not read is a refusal, never one of these: 422 `fleet.repository_file_not_addressable`, `fleet.repository_file_too_big`
 * (over 8 MiB) or `fleet.repository_file_binary`, or 500 `fleet.repository_file_unreadable`.
 */
export type RepositoryFile =
  | {
      kind: "text";
      text: string;
      /** `text/markdown` for `.md` and `.markdown`, else `text/plain`. */
      type: string;
    }
  | {
      kind: "picture";
      /** The file's bytes, base64. */
      bytes: string;
      /** `image/png`, `image/jpeg`, `image/gif`, `image/webp` or `image/svg+xml`, by the extension. */
      type: string;
    };
