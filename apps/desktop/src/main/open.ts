// Hand settings.json to the editor a person named, or to the OS.
//
// The one place in Bridge that reaches outside the app with a path, and it does so on a path **it
// was given by Fleet** — the renderer sends none, and nothing it sends is concatenated into what
// `shell.openPath` receives.
//
// **Which editor it lands in is `editor.ts`'s question, not this file's.** That module owns the
// precedence and the argv-only rule that keeps a configured string away from a shell; this one owns
// the path and the answer.

import { shell } from "electron";
import { writeFile } from "node:fs/promises";

import type { BridgeState } from "../shared/bridge";
import type { SettingsFileOpened } from "../shared/api/settings";
import { chooseEditor, startEditor, whereIs } from "./editor";
import type { Editor } from "./editor";
import { textSetting } from "./settings";

/**
 * Why a named editor could not be used, in the voice the row will read it in.
 *
 * **A configured editor that is not there says so rather than falling through
 * to Finder.** Finder would open, the path would appear, and the setting would
 * look ignored — which is the one outcome that teaches a person the wrong thing
 * about their own configuration. Falling through from an *unset* variable is a
 * different case entirely and is silent, because nothing has gone wrong.
 *
 * The sentence names the setting or the variable, because the next step is
 * editing the line that set it.
 */
function noEditor(editor: Editor): string {
  if (!editor.command.includes("/")) {
    return `${editor.from} names ${editor.command}, and nothing on PATH is called that.`;
  }
  return editor.command.startsWith("/")
    ? `${editor.from} names ${editor.command}, and nothing executable is there.`
    : `${editor.from} names ${editor.command}, which is relative to nothing — an editor is named by an absolute path or by a name on PATH.`;
}

/**
 * Open a path in the editor `editor.command` names, then `$VISUAL` or `$EDITOR`, then whatever the
 * OS opens it with. `null` is success; a string is why not, in the row's voice.
 */
export async function inEditor(state: BridgeState, path: string): Promise<string | null> {
  const editor = chooseEditor(process.env, textSetting(state.settings, "editor.command"));
  if (editor !== null) {
    const program = await whereIs(editor.command, process.env);
    if (program === null) return noEditor(editor);
    startEditor(program, editor.args, path);
    return null;
  }

  // Empty is success. Anything else is the OS explaining itself, and it is
  // carried through rather than replaced with a sentence Bridge made up.
  const refused = await shell.openPath(path);
  return refused === "" ? null : refused;
}

/**
 * Open settings.json in the editor. **The path is the one Fleet named**, never one the renderer
 * sends. Fleet writes the file only once something is saved, so a machine with none gets an empty
 * object first: `{}` is every shipped default, which is what Fleet already has in force.
 */
export async function openSettingsFile(state: BridgeState): Promise<SettingsFileOpened> {
  const path = state.settings?.path;
  if (path === undefined || path === "") return { ok: false, why: "not_read" };
  try {
    await writeFile(path, "{}\n", { flag: "wx" });
  } catch {
    // Already there, which is the usual case. A directory that cannot be written is the editor's to report.
  }
  const opened = await inEditor(state, path);
  return opened === null ? { ok: true } : { ok: false, why: "refused", detail: opened };
}
