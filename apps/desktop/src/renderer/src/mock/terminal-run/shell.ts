// The Job's shell behind the mock's Terminal: zsh in the Job's worktree, as far as a walk needs it. It
// echoes what is typed, answers `ls` and `pwd`, serves `pnpm dev` until Ctrl-C, and says it has no other
// command. Mock-only. It reads the bytes a terminal sends and writes the bytes zsh would, so the screen in
// front of it is the real one. The agent types into it the same way: its words, then Enter.

import { KEY } from "./wrangler";

const ESC = "\x1b";
const sgr = (open: string, close: number) => (text: string) => `${ESC}[${open}m${text}${ESC}[${close}m`;
const dim = sgr("2", 22);
const cyan = sgr("36", 39);
const green = sgr("32", 39);
const folder = sgr("1;34", 0);

const LISTING = ["ARCHITECTURE.md", "CONTRIBUTING.md", "Cargo.toml", "README.md", folder("apps"), folder("crates"), folder("docs"), "package.json", folder("packages"), folder("xtask")];

/** How often the dev server prints while it serves, and the most it prints before it goes quiet. */
const SERVED_EVERY = 1200;
const SERVED_MOST = 30;
const RELOADED = ["src/App.tsx", "src/sessions.tsx", "src/styles/index.css", "src/sessions-wired.tsx"];

export type FakeShell = {
  /** Bytes from the keyboard, the person's or the agent's. */
  write: (data: string) => void;
  /** Ends it as a stop: no more output, and `end` hears null. */
  stop: () => void;
};

const clock = () => new Date().toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", second: "2-digit" });

/** Starts the shell in `dir`, which the prompt names by its last part. `print` writes to the terminal; `end` is its exit. */
export function startShell(dir: string, print: (text: string) => void, end: (code: number | null) => void): FakeShell {
  const prompt = `${folder(dir.split("/").pop() ?? dir)} % `;
  let typed = "";
  let ended = false;
  /** Whether the dev server holds the foreground, which Ctrl-C ends, and what keeps it printing. */
  let serving = false;
  let ticking: number | null = null;

  const out = (text: string) => {
    if (!ended) print(text);
  };
  const quiet = () => {
    if (ticking !== null) window.clearInterval(ticking);
    ticking = null;
  };
  const finish = (code: number | null) => {
    if (ended) return;
    quiet();
    ended = true;
    end(code);
  };

  const serve = () => {
    serving = true;
    out(`\r\n> armada@0.0.0 dev ${dir}\r\n> pnpm -C apps/desktop dev\r\n\r\n`);
    out(`  ${green("VITE v6.3.5")}  ${dim("ready in")} 412 ms\r\n\r\n`);
    out(`  ${green("➜")}  Local:   ${cyan("http://localhost:5173/")}\r\n  ${green("➜")}  ${dim("Network: use --host to expose")}\r\n`);
    let printed = 0;
    ticking = window.setInterval(() => {
      out(`${dim(clock())} ${cyan("[vite]")} ${green("hmr update")} ${dim(`/${RELOADED[printed % RELOADED.length]}`)}\r\n`);
      printed += 1;
      // Quiet after a while, and still in the foreground: Ctrl-C is what gives the prompt back.
      if (printed >= SERVED_MOST) quiet();
    }, SERVED_EVERY);
  };

  const run = (line: string) => {
    const command = line.trim();
    if (command === "") return out(prompt);
    if (command === "exit") return finish(0);
    if (command === "pwd") return out(`${dir}\r\n${prompt}`);
    if (command === "ls") return out(`${LISTING.join("  ")}\r\n${prompt}`);
    if (command === "pnpm dev") return serve();
    out(`zsh: command not found: ${command.split(/\s+/)[0]}\r\n${prompt}`);
  };

  const key = (one: string) => {
    if (one === KEY.interrupt) {
      quiet();
      serving = false;
      typed = "";
      return out(`^C\r\n${prompt}`);
    }
    // The server holds the foreground: nothing else typed reaches the shell until it is stopped.
    if (serving) return;
    if (one === KEY.enter || one === "\n") {
      const line = typed;
      typed = "";
      out("\r\n");
      return run(line);
    }
    if (one === KEY.backspace) {
      if (typed === "") return;
      typed = typed.slice(0, -1);
      return out("\b \b");
    }
    if (one >= " ") {
      typed += one;
      out(one);
    }
  };

  // A shell takes a moment to start, and says nothing until its prompt.
  window.setTimeout(() => out(prompt), 30);
  return {
    // An escape sequence is a key this shell does not answer, an arrow among them.
    write: (data) => {
      if (ended) return;
      for (const one of data.replace(/\x1b\[[0-9;]*[A-Za-z~]/g, "")) key(one);
    },
    stop: () => finish(null),
  };
}
