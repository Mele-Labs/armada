// The scripted program behind the film's Flight 2: the payments CLI's sign-in. It asks for a one time code, shows the characters as
// they are typed, and exits 0 once a line is entered. Mock-only, and reads the bytes a real terminal sends for a key, as
// `passphrase.ts` does.

import { KEY, type Deploy } from "./wrangler";

const PROMPT = "Sign-in code: ";

/** Starts the program. `print` is what it writes to the terminal; `end` is how it exits, with null for a stop. */
export function startPaymentsLogin(command: string, print: (text: string) => void, end: (code: number | null) => void): Deploy {
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
  later(150, () => out("Relay payments sandbox, api.sandbox.relay.test\r\n"));
  later(300, () => {
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
          out("\r\nSigned in to the payments sandbox as relay-api\r\n");
          later(150, () => finish(0));
          return;
        }
        if (key === KEY.backspace) {
          if (typed !== "") out("\b \b");
          typed = typed.slice(0, -1);
          continue;
        }
        typed += key;
        out(key);
      }
    },
    stop: () => {
      out("^C\r\n");
      finish(null);
    },
  };
}
