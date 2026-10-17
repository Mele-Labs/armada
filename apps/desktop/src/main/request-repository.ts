// Which repository the words of a request are about, asked of Fleet. Beside `proposing.ts` for its
// reason: `command.ts` is at the length the gate refuses, and this is one read.

import type { RequestRepository } from "@armada/protocol";

import type { RequestRepositoryRead } from "../shared/api/dispatch";
import type { Board } from "./command";
import { ask } from "./request";

/** How much of a request's words ride the query string of `read_request_repository`. */
const MOST_WORDS_READ = 16_000;

/**
 * `read_request_repository`. Fleet reads it without a model, so this runs on a pause in typing.
 *
 * **Never a fault, `searchFiles`' reason**: nothing connected, or a Fleet that refuses, answers no pick
 * and no ranking, which the panel meets by asking the person. Fleet leaves `manifest_id` out where it
 * could not tell; this is where that becomes the `null` the renderer compares against.
 */
export async function readRequestRepository(board: Pick<Board, "port">, words: string): Promise<RequestRepositoryRead> {
  const none: RequestRepositoryRead = { manifest_id: null, likely: [] };
  const port = board.port();
  if (port === null) return none;
  const answer = await ask(port, "GET", `/repositories/of_request?q=${encodeURIComponent(words.slice(0, MOST_WORDS_READ))}`);
  if (answer.ok !== true) return none;
  const read = answer.body as RequestRepository;
  return { manifest_id: read.manifest_id ?? null, likely: read.likely };
}
