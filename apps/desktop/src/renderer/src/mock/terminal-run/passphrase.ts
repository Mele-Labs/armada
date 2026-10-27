// The scripted program behind `needs-you-run-prompts`: a script that unlocks a signing key, asks for its
// passphrase without echoing it, and exits 0 once a line is entered. Mock-only, and reads the bytes a real
// terminal sends for a key, as `wrangler.ts` does.

import { KEY, type Deploy } from "./wrangler";

const PROMPT = "Passphrase: ";

/** Starts the program. `print` is what it writes to the terminal; `end` is how it exits, with null for a stop. */
export function startUnlock(command: string, print: (text: string) => void, end: (code: number | null) => void): Deploy {
  let typed = "";
  let asking = false;
  let ended = false;
  const timers = new Set<number>();
  const later = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const out = (text: string) => {
    if (!ended) print(text);
  };
  const finish = (code: number | null) => {
    if (ended) return;
    ended = true;
    timers.forEach((one) => window.clearTimeout(one));
    timers.clear();
    end(code);
  };

  out(`$ ${command}\r\n`);
  later(200, () => out("Unlocking the release signing key\r\n"));
  later(400, () => {
    out(PROMPT);
    asking = true;
  });

  return {
    write: (data) => {
      if (ended) return;
      if (data === KEY.interrupt) {
        out("^C\r\n");
        finish(null);
        return;
      }
      if (!asking) return;
      for (const key of data) {
        if (key === KEY.enter) {
          if (typed === "") continue;
          asking = false;
          out("\r\nKey unlocked for 8 hours\r\n");
          later(150, () => finish(0));
          return;
        }
        // Nothing is echoed, as a passphrase prompt does.
        typed = key === KEY.backspace ? typed.slice(0, -1) : typed + key;
      }
    },
    stop: () => {
      out("^C\r\n");
      finish(null);
    },
  };
}
