// The scripted program behind the fake terminal for `claude auth login`: it opens a browser sign-in, waits
// for it, and prints the success line. Mock-only, and it reads no keys beyond Ctrl-C.

const ESC = "\x1b";
const sgr = (open: number, close: number) => (text: string) => `${ESC}[${open}m${text}${ESC}[${close}m`;
const green = sgr(32, 39);
const cyan = sgr(36, 39);
const underlined = sgr(4, 24);
const link = (address: string) => cyan(underlined(address));

const SIGN_IN = "https://claude.ai/oauth/authorize?code=true&client_id=9d1c250a&response_type=code&scope=user%3Ainference&state=Qn4tZ8";

export type Login = {
  write: (data: string) => void;
  stop: () => void;
};

/** Starts the program. `print` is what it writes; `end` is its exit, `null` for a stop. */
export function startLogin(command: string, print: (text: string) => void, end: (code: number | null) => void): Login {
  let ended = false;
  const timers = new Set<number>();
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
  let at = 0;
  const parts: readonly (readonly [wait: number, part: string | (() => void)])[] = [
    [150, `${green("$")} ${command}\r\n`],
    [250, "Opening browser to sign in\r\n"],
    [150, `${link(SIGN_IN)}\r\n`],
    // The browser sign-in.
    [2000, `${green("✔")} Login successful\r\n`],
    [200, () => finish(0)],
  ];
  for (const [wait, part] of parts) {
    at += wait;
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      if (typeof part === "string") out(part);
      else part();
    }, at);
    timers.add(handle);
  }
  return {
    write: (data) => {
      if (data !== "\x03") return;
      out("^C\r\n");
      finish(null);
    },
    stop: () => finish(null),
  };
}
