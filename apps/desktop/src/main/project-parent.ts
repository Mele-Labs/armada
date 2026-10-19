// The folder a new project is made in. Bridge's window writes it with `~` for home, which only this process knows, and Fleet wants an absolute folder that is
// already there. `~/code` is where a person with no projects yet is offered to make one, so it is made here if it is missing; any other folder was chosen or read
// from where projects already are.

import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

/** Where a person with no other projects is offered a new one. The window's `FALLBACK_PARENT`. */
export const CODE = "~/code";

/** `parent` with a leading `~` made `home`, and `~/code` made if it is not there. */
export async function projectParent(
  parent: string,
  home: string = homedir(),
  make: (path: string) => Promise<unknown> = (path) => mkdir(path, { recursive: true }),
): Promise<string> {
  const absolute = parent === "~" ? home : parent.startsWith("~/") ? join(home, parent.slice(2)) : parent;
  if (parent === CODE) await make(absolute);
  return absolute;
}
