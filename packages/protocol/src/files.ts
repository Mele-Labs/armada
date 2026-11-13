/**
 * `GET /manifest/files` — paths under the checkout narrowed against typed
 * text, for the `@` mention popup a person opens while writing a request or a
 * brief, or with `?folders=true` the folders instead. Mirrors `ipc::FilesFound`
 * in `crates/ipc/src/files.rs`.
 *
 * **Cut by Fleet, never re-cut by a reader.** `crates/fleet/src/files.rs`
 * decides how many paths come back; a surface draws what it is handed, so a
 * second cap here would be a second answer to a question Fleet already
 * answered.
 *
 * **`folders` answers `?folders=true`, and then `paths` is empty.** Each is
 * relative to the root, `/`-separated, with no trailing `/`, the root itself
 * never listed. Absent where the answer is files.
 */
export type FilesFound = {
  paths: string[];
  folders?: string[];
};
