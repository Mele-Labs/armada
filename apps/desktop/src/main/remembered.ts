// The picture the screens draw from, kept on disk so a restarted Bridge draws it at once instead of filling in over seconds.
//
// One file in `userData`, written `WRITE_AFTER_MS` after the first change since the last write (`sessions.ts` keeps thread
// rows the same way) and once more as Bridge quits. At boot a file under `REMEMBERED_FOR_MS` old is the state until Fleet's
// own reads land over it, as they do over a held state after a reconnect.
//
// **The file was usually written by the previous build**, so every field is checked on the way in and one that does not
// look right is dropped. A file that does not parse at all is ignored.

import { readFileSync, writeFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import type { Holdings, MergeLines, PlanUsage, SessionRecord, TimelineEvent } from "@armada/protocol";

/** How old the file may be and still be drawn. Fixed, not a setting. */
export const REMEMBERED_FOR_MS = 5 * 60_000;
/** The Jobs list alone is megabytes, so a busy Fleet is written this often at most; the quit write is the one a restart reads. */
const WRITE_AFTER_MS = 15_000;

/** What is kept: each part is absent where it was not read or did not look right. */
export type Picture = {
  sessions?: SessionRecord[];
  mergeLines?: MergeLines;
  holds?: Pick<Holdings, "workflows" | "manifests" | "repositories">;
  planUsage?: PlanUsage;
  timeline?: TimelineEvent[];
};

const isObject = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const listOf = (value: unknown, one: (item: Record<string, unknown>) => boolean): boolean =>
  Array.isArray(value) && value.every((item) => isObject(item) && one(item));

/** The picture in `text`, or `null` where it is older than `REMEMBERED_FOR_MS`, dated ahead of `now`, does not parse, or holds nothing usable. */
export function parsePicture(text: string, now: number): Picture | null {
  let file: unknown;
  try {
    file = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(file) || typeof file["at"] !== "number") return null;
  const age = now - file["at"];
  if (!(age >= 0 && age < REMEMBERED_FOR_MS)) return null;

  const picture: Picture = {};
  const sessions = file["sessions"];
  if (listOf(sessions, (one) => typeof one["id"] === "string" && typeof one["state"] === "string")) picture.sessions = sessions as SessionRecord[];
  const mergeLines = file["mergeLines"];
  if (isObject(mergeLines) && Array.isArray(mergeLines["lines"])) picture.mergeLines = mergeLines as MergeLines;
  const holds = file["holds"];
  if (isObject(holds) && Array.isArray(holds["workflows"]) && Array.isArray(holds["manifests"]) && Array.isArray(holds["repositories"])) {
    picture.holds = { workflows: holds["workflows"], manifests: holds["manifests"], repositories: holds["repositories"] } as Picture["holds"] & object;
  }
  const planUsage = file["planUsage"];
  if (isObject(planUsage) && Array.isArray(planUsage["windows"])) picture.planUsage = planUsage as PlanUsage;
  const timeline = file["timeline"];
  if (listOf(timeline, (one) => typeof one["id"] === "number")) picture.timeline = timeline as TimelineEvent[];
  return Object.keys(picture).length === 0 ? null : picture;
}

export class Remembered {
  private picture: Picture = {};
  private writing: ReturnType<typeof setTimeout> | undefined;
  /** Whether Fleet has said anything since boot: a picture only ever loaded is never stamped again, so it ages out. */
  private heard = false;

  constructor(
    private readonly file: string | undefined,
    private readonly now: () => number,
  ) {}

  /** What the file held, read once at boot. A missing, unreadable, stale or unusable file is nothing. */
  load(): Picture | null {
    if (this.file === undefined) return null;
    let text: string;
    try {
      text = readFileSync(this.file, "utf8");
    } catch {
      return null;
    }
    const found = parsePicture(text, this.now());
    this.picture = { ...found };
    return found;
  }

  /** Parts that changed. They are written `WRITE_AFTER_MS` after the first change since the last write. */
  keep(part: Picture): void {
    this.picture = { ...this.picture, ...part };
    this.heard = true;
    if (this.file === undefined) return;
    this.writing ??= setTimeout(() => void this.write(), WRITE_AFTER_MS);
  }

  /** A write that fails is just a picture that is not there at the next boot. */
  private async write(): Promise<void> {
    this.writing = undefined;
    const file = this.file;
    if (file === undefined) return;
    try {
      await mkdir(dirname(file), { recursive: true });
      await writeFile(file, JSON.stringify({ at: this.now(), ...this.picture }));
    } catch {}
  }

  /** Bridge is quitting: the picture goes now, stamped now, since Fleet's stream kept it current until this moment. */
  flush(): void {
    if (this.file === undefined || !this.heard) return;
    clearTimeout(this.writing);
    this.writing = undefined;
    try {
      writeFileSync(this.file, JSON.stringify({ at: this.now(), ...this.picture }));
    } catch {}
  }
}
