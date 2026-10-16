// The scripted program behind the fake terminal: `wrangler login` and `wrangler deploy` as wrangler 4
// prints them for a first deploy, with the three prompts waiting on keys. Mock-only. What it prints is the
// byte stream a real program sends (colours and all), and what it reads is the byte a real terminal sends
// for a key, so the screen in front of it is the part the real build replaces with xterm.js.

const ESC = "\x1b";
const sgr = (open: number, close: number) => (text: string) => `${ESC}[${open}m${text}${ESC}[${close}m`;
const bold = sgr(1, 22);
const dim = sgr(2, 22);
const cyan = sgr(36, 39);
const green = sgr(32, 39);
const red = sgr(31, 39);
const underlined = sgr(4, 24);
/** An address a terminal shows as a link: cyan and underlined. */
const link = (address: string) => cyan(underlined(address));

const VERSION = "4.42.0";
const HEADER = ` ⛅️ wrangler ${VERSION}\r\n${dim("─".repeat(19))}\r\n`;
const SIGN_IN =
  "https://dash.cloudflare.com/oauth2/auth?response_type=code&client_id=54d11594-84e4-41aa-b438-e81b8fa78ee7&redirect_uri=http%3A%2F%2Flocalhost%3A8976%2Foauth%2Fcallback&scope=account%3Aread%20user%3Aread%20workers%3Awrite&state=Bx7vNq2eT1";

const REGISTER = "Would you like to register a workers.dev subdomain now?";
const NAME = "What would you like your workers.dev subdomain to be? It will be accessible at https://<subdomain>.workers.dev";
const confirmation = (name: string) => `Creating a workers.dev subdomain for your account at https://${name}.workers.dev. Ok to proceed?`;

/** A prompt as prompts draws it while it waits: the question in bold, then what it expects. */
const asking = (question: string, hint = "") => `${cyan("?")} ${bold(question)} ${dim("›")}${hint === "" ? " " : ` ${dim(hint)} `}`;
/** The same line once answered, written over the prompt. */
const answered = (question: string, answer: string) => `\r${ESC}[K${green("✔")} ${bold(question)} ${dim("…")} ${answer}\r\n`;

/** The keys the program reads: what a terminal sends for Enter, Backspace and Ctrl-C. */
export const KEY = { enter: "\r", backspace: "\x7f", interrupt: "\x03" } as const;

type Waiting = "register" | "name" | "confirm";

export type Deploy = {
  /** Bytes from the keyboard. */
  write: (data: string) => void;
  /** Ends it as a stop: no more output, and `end` hears null. */
  stop: () => void;
};

/**
 * Starts the program. `print` is what it writes to the terminal; `end` is how it exits, with `null` for
 * a program that was stopped. Nothing is printed after it ends.
 */
export function startDeploy(command: string, print: (text: string) => void, end: (code: number | null) => void): Deploy {
  let waiting: Waiting | null = null;
  let typed = "";
  let ended = false;
  let name = "";
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
  /** Each part after the wait before it. A part that is a function runs instead of printing. */
  const play = (parts: readonly (readonly [wait: number, part: string | (() => void)])[]) => {
    let at = 0;
    for (const [wait, part] of parts) {
      at += wait;
      const handle = window.setTimeout(() => {
        timers.delete(handle);
        if (typeof part === "string") out(part);
        else part();
      }, at);
      timers.add(handle);
    }
  };
  const ask = (next: Waiting) => {
    waiting = next;
    typed = "";
    out(next === "register" ? asking(REGISTER, "(Y/n)") : next === "name" ? asking(NAME) : asking(confirmation(name), "(Y/n)"));
  };
  const deploy = () => {
    waiting = null;
    const address = `https://armada-store.${name}.workers.dev`;
    play([
      [300, `${green("Success!")} It may take a few minutes for DNS records to update.\r\n`],
      [500, "Uploaded armada-store (3.12 sec)\r\n"],
      [400, "Deployed armada-store triggers (1.04 sec)\r\n"],
      [100, `  ${link(address)}\r\n`],
      [100, "Current Version ID: 6f2c1e0a-4b7d-4c1e-9a55-0d3f8e2b71c4\r\n"],
      [200, () => finish(0)],
    ]);
  };
  const submit = () => {
    const question = waiting;
    const answer = typed.trim();
    if (question === "name") {
      if (answer === "") return;
      name = answer;
      out(answered(NAME, cyan(answer)));
      ask("confirm");
      return;
    }
    const yes = answer === "" || /^y(es)?$/i.test(answer);
    const asked = question === "register" ? REGISTER : confirmation(name);
    if (!yes) {
      out(`\r${ESC}[K${red("✘")} ${bold(asked)} ${dim("…")} no\r\n`);
      waiting = null;
      play([[100, `${red("Aborted.")} A workers.dev subdomain is needed to deploy.\r\n`], [100, () => finish(1)]]);
      return;
    }
    out(answered(asked, "yes"));
    if (question === "register") ask("name");
    else deploy();
  };

  out(`${green("$")} ${command}\r\n`);
  play([
    [150, HEADER],
    [250, "Attempting to login via OAuth...\r\n"],
    [200, `Opening a link in your default browser: ${link(SIGN_IN)}\r\n`],
    // The browser sign-in.
    [2000, "Successfully logged in.\r\n\r\n"],
    [300, HEADER],
    [150, "Total Upload: 412.38 KiB / gzip: 88.10 KiB\r\n"],
    [150, "You need to register a workers.dev subdomain before publishing to workers.dev\r\n"],
    [200, () => ask("register")],
  ]);

  return {
    write: (data) => {
      if (ended) return;
      if (data === KEY.interrupt) {
        out("^C\r\n");
        finish(null);
      } else if (waiting === null) return;
      else if (data === KEY.enter) submit();
      else if (data === KEY.backspace) {
        if (typed === "") return;
        typed = typed.slice(0, -1);
        out("\b \b");
      } else if (data.length === 1 && data >= " ") {
        typed += data;
        out(data);
      }
    },
    stop: () => finish(null),
  };
}
