// Starting out, as main asks Fleet: the folders a person already works in, how a repository runs and where that is kept,
// and the mod's install. One request each and nothing kept here; Fleet holds every answer. `docs/practices/protocol.md`.

import type {
  DropRunFact,
  ImportFolders,
  ImportRuns,
  KeptOnThisMac,
  ModInstalled,
  Outcome,
  RunMemory,
  RunMemoryPlace,
  SetRunMemoryPlace,
} from "@armada/protocol";
import type { OnboardingAnswer } from "../shared/api/onboarding";
import { ask, COMMAND_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/**
 * How long the mod's install is waited for. **Fleet stops each of its steps at two minutes**, and the install has up to three; this
 * is the six plus one, so Fleet's own `failed` arrives rather than Bridge's abort. Coupled to the Rust bound by this comment.
 */
export const INSTALL_MOD_MS = 7 * 60_000;

/**
 * How long a read of past sessions is waited for. **It reads up to 30 sessions' records, 8 MiB of each**, which an ordinary read's
 * wait is not sized for. Fleet states no bound of its own on it, so this is a guess, made generous; a timeout says the request went out.
 */
export const IMPORT_READ_MS = 120_000;

export class OnboardingHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  importFolders(): Promise<OnboardingAnswer<ImportFolders>> {
    return this.send("GET", "/import/folders");
  }

  importRuns(path: string): Promise<OnboardingAnswer<ImportRuns>> {
    return this.send("GET", `/import/runs?path=${encodeURIComponent(path)}`, undefined, IMPORT_READ_MS);
  }

  installMod(): Promise<OnboardingAnswer<ModInstalled>> {
    return this.send("POST", "/import/install_mod", { requested: true }, INSTALL_MOD_MS);
  }

  /** Reading merges in the repository's own folder's imported sessions, so it waits as an import read does. */
  runMemory(repository: string): Promise<OnboardingAnswer<RunMemory>> {
    return this.send("GET", `/repositories/runs?repository=${encodeURIComponent(repository)}`, undefined, IMPORT_READ_MS);
  }

  runMemoryPlace(repository: string): Promise<OnboardingAnswer<RunMemoryPlace>> {
    return this.send("GET", `/repositories/runs/place?repository=${encodeURIComponent(repository)}`);
  }

  setRunMemoryPlace(body: SetRunMemoryPlace): Promise<OnboardingAnswer<RunMemoryPlace>> {
    return this.send("POST", "/repositories/runs/place", body);
  }

  keptOnThisMac(repository: string): Promise<OnboardingAnswer<KeptOnThisMac>> {
    return this.send("GET", `/repositories/kept_on_this_mac?repository=${encodeURIComponent(repository)}`);
  }

  setKeptOnThisMac(body: KeptOnThisMac): Promise<OnboardingAnswer<KeptOnThisMac>> {
    return this.send("POST", "/repositories/kept_on_this_mac", body);
  }

  dropRunFact(body: DropRunFact): Promise<OnboardingAnswer<RunMemory>> {
    return this.send("POST", "/repositories/runs/drop", body);
  }

  private async send<T>(method: "GET" | "POST", path: string, body?: unknown, waitMs: number = COMMAND_MS): Promise<OnboardingAnswer<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body, waitMs);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}
