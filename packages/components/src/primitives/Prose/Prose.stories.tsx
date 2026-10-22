import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor, within } from "storybook/test";
import { Clamped } from "../../compositions/Clamped/Clamped";
import { Card } from "../Card/Card";
import { CopyAct, Prose, ProseCommand, ProseLinks, ProseRefs, ProseRunner, isSlashCommand } from "./Prose";
import { readsAsShell } from "./RunnableBlock";
import type { CodeRunner, ProseReference } from "./Prose";

/** What a click hands over. Bridge's is `window.armada.openLink`. */
const opened = fn();

const meta: Meta<typeof Prose> = {
  title: "Primitives/Prose",
  component: Prose,
  decorators: [
    (Story) => (
      <ProseLinks.Provider value={opened}>
        <Story />
      </ProseLinks.Provider>
    ),
  ],
  beforeEach: () => {
    opened.mockClear();
  },
};
export default meta;

type Story = StoryObj<typeof Prose>;

/**
 * A Judge's `consequence`: several paragraphs with a path and an expression
 * in them. Rendered as one string, this was the wall the override dialog was
 * reported for.
 */
export const AJudgesConsequence: Story = {
  args: {
    text:
      "The route table is walked once per operation and the loop skips `forget_job`, so the " +
      "assertion that every operation is served passes without ever reading the one operation " +
      "the step was about.\n\n" +
      "The skip is in `crates/api/src/tests/served.rs` and reads:\n\n" +
      "```rust\nif route.operation == \"forget_job\" { continue; }\n```\n\n" +
      "Nothing else in the file narrows the set, so the count the assertion compares against " +
      "was lowered by the same edit that made it pass.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("paragraph")).toHaveLength(3);
    await expect(canvas.getByText("crates/api/src/tests/served.rs")).toBeVisible();
    await expect(
      canvas.getByText('if route.operation == "forget_job" { continue; }'),
    ).toBeVisible();
    await expect(canvas.queryByText(/`/)).toBeNull();
  },
};

/**
 * A citation on a gaming flag. Inline code stays inline, and the fenced block
 * wraps rather than clipping, because the failing render broke an expression
 * mid-token.
 */
export const AFlagsCitation: Story = {
  args: {
    text:
      "`crates/api/src/tests/served.rs:214` — the `served_every_operation` assertion counts " +
      "`ROUTES.len()` after the filter rather than before it:\n\n" +
      "```\nlet routes: Vec<&Route> = ROUTES.iter().filter(|r| r.operation != \"forget_job\").collect();\n" +
      "assert_eq!(routes.len(), served.len());\n```",
  },
};

/**
 * The structure a model writes when it lists findings. The `#` line renders at
 * `--text-lg` over a rule, at `--weight-heading` and full contrast. It takes no
 * place in the page's outline, since a model's `#` inside a card is not the
 * card's title.
 */
export const AListAndAHeading: Story = {
  args: {
    text:
      "# What the check read\n\n" +
      "Three things, and the third is the one that matters:\n\n" +
      "- the diff touches `armada.yml`\n" +
      "- the touched key is `checks.tests.command`\n" +
      "- the command it was changed **to** exits 0 on an empty test set\n\n" +
      "The first two alone are ordinary. Together with the third they are the pattern.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("What the check read")).toBeVisible();
    await expect(canvas.queryByText(/#/)).toBeNull();
    await expect(canvas.queryByRole("heading")).toBeNull();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(3);
  },
};

/** Strong is contrast, emphasis is slant, and a strikethrough is a line through. */
export const Emphasis: Story = {
  args: {
    text: "The command was changed **to** exit 0, *after* the review, and ~~before~~ the Judge read it.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("strong")).toHaveTextContent("to");
    await expect(canvas.getByRole("emphasis")).toHaveTextContent("after");
    await expect(canvas.getByRole("deletion")).toHaveTextContent("before");
    await expect(canvas.queryByText(/[*~]/)).toBeNull();
  },
};

/** An ordered list keeps its numbers, because they carry the order. */
export const AnOrderedList: Story = {
  args: {
    text:
      "1. Read the diff\n" +
      "2. Run the touched Check\n" +
      "3. Compare the count against `main`\n" +
      "   - before the filter\n" +
      "   - after it",
  },
  play: async ({ canvas }) => {
    const [steps, nested] = canvas.getAllByRole("list");
    await expect(steps?.tagName).toBe("OL");
    await expect(nested?.tagName).toBe("UL");
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
  },
};

/** A task list's state is a disabled checkbox, so it is read as well as seen. */
export const ATaskList: Story = {
  args: {
    text: "- [x] Reproduce the skip\n- [ ] Restore `forget_job` to the walk",
  },
  play: async ({ canvas }) => {
    const [done, open] = canvas.getAllByRole("checkbox");
    await expect(done).toBeChecked();
    await expect(open).not.toBeChecked();
    await expect(done).toBeDisabled();
  },
};

/**
 * A table drawn with the Table primitive's rules and header contrast, at the
 * surrounding size. A cell wraps rather than scrolling.
 */
export const ATable: Story = {
  args: {
    text:
      "| Check | Before | After |\n" +
      "|---|---:|---:|\n" +
      "| `served_every_operation` | 41 | 40 |\n" +
      "| `routes_are_documented` | 41 | 41 |",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("table")).toBeVisible();
    await expect(canvas.getAllByRole("columnheader").map((cell) => cell.textContent)).toEqual([
      "Check",
      "Before",
      "After",
    ]);
    await expect(canvas.getAllByRole("row")).toHaveLength(3);
  },
};

/** A quote and a rule. Neither keeps its characters. */
export const AQuoteAndARule: Story = {
  args: {
    text:
      "The brief said:\n\n" +
      "> Every operation in `operations.toml` is served by exactly one route.\n\n" +
      "---\n\n" +
      "The assertion checked one fewer.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("blockquote")).toHaveTextContent(
      "Every operation in operations.toml is served by exactly one route.",
    );
    await expect(canvas.queryByText(/^>|---/)).toBeNull();
  },
};

/**
 * Links, in `--accent`. A click hands the address to `ProseLinks` and the
 * window goes nowhere. Only `http(s):` is a link. A `javascript:` address and
 * a relative path draw as their text, and an image is a link to it rather than
 * a fetch.
 */
export const Links: Story = {
  args: {
    text:
      "The upstream fix is [tokio#7012](https://forge.invalid/tokio-rs/tokio/pull/7012), and " +
      "https://docs.rs/tokio says the same.\n\n" +
      "[This one](javascript:alert(1)) and [this one](../served.rs) are not links.\n\n" +
      "![the failing render](https://example.com/render.png)",
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("link", { name: "tokio#7012" }));
    await expect(opened).toHaveBeenCalledWith("https://forge.invalid/tokio-rs/tokio/pull/7012");
    await expect(canvas.getByRole("link", { name: "https://docs.rs/tokio" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "the failing render" })).toBeVisible();
    await expect(canvas.queryByRole("img")).toBeNull();
    await expect(canvas.queryByRole("link", { name: "This one" })).toBeNull();
    await expect(canvas.queryByRole("link", { name: "this one" })).toBeNull();
    await expect(canvas.getByText("This one and this one are not links.")).toBeVisible();
    await expect(opened).toHaveBeenCalledTimes(1);
  },
};

/** With nothing to open it, a link is its text: no control where there is nowhere to go. */
export const LinksWithNowhereToOpen: Story = {
  args: {
    text: "The upstream fix is [tokio#7012](https://forge.invalid/tokio-rs/tokio/pull/7012).",
  },
  decorators: [
    (Story) => (
      <ProseLinks.Provider value={null}>
        <Story />
      </ProseLinks.Provider>
    ),
  ],
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("link")).toBeNull();
    await expect(canvas.getByText(/tokio#7012/)).toBeVisible();
  },
};

/** What a host's `scan` hands back: every `#N` and every `s-` id, wherever it stands, code included. */
const pressed = fn();
const scan = (text: string) =>
  [...text.matchAll(/#\d+|s-[0-9a-f]{8}/g)].map((one) => ({
    from: one.index,
    to: one.index + one[0].length,
    ref: { kind: one[0].startsWith("#") ? "issue" : "session", id: one[0], title: one[0] } satisfies ProseReference,
  }));

/**
 * A number in code is not a reference; a Job id is, and a model writes it in backticks. Code that is
 * more than the id stays code.
 */
export const AReferenceInCode: Story = {
  args: {
    text: "The change is #2209 and `#2209` is code. Job `s-9e2e41a1` found it; `s-9e2e41a1 --resume` is code.",
  },
  decorators: [
    (Story) => (
      <ProseRefs.Provider value={{ scan, open: pressed }}>
        <Story />
      </ProseRefs.Provider>
    ),
  ],
  beforeEach: () => {
    pressed.mockClear();
  },
  play: async ({ canvas, userEvent }) => {
    // The sentence's `#2209` links; the one in backticks stays plain.
    await expect(canvas.getAllByRole("link", { name: "#2209" })).toHaveLength(1);
    await expect(canvas.getByText("#2209", { selector: "code" })).toBeVisible();
    // A span that is exactly a Job id links, and is still code.
    await userEvent.click(canvas.getByRole("link", { name: "s-9e2e41a1" }));
    await expect(pressed).toHaveBeenCalledWith({ kind: "session", id: "s-9e2e41a1", title: "s-9e2e41a1" });
    await expect(canvas.queryByRole("link", { name: "s-9e2e41a1 --resume" })).toBeNull();
    await expect(canvas.getByText("s-9e2e41a1 --resume", { selector: "code" })).toBeVisible();
    await expect(canvas.getAllByRole("link")).toHaveLength(2);
  },
};

/** Raw HTML is the characters it was written as. This text arrives from a model. */
/** What a host that knows `docs/plan.md` is a file in the Job's checkout hands back: it finds it anywhere, plain words included. */
const scanFile = (text: string) =>
  [...text.matchAll(/docs\/plan\.md/g)].map((one) => ({
    from: one.index,
    to: one.index + one[0].length,
    ref: { kind: "file", id: "docs/plan.md", title: "docs/plan.md" } satisfies ProseReference,
  }));

/**
 * A path a model writes in backticks is a link where the host found it to be a file; it stays code. The same words in a
 * sentence, in a span with more than the path, or in a fenced block are not the link.
 */
export const AFileInCode: Story = {
  args: {
    text: "Read `docs/plan.md`, then `docs/plan.md --draft`. Plain docs/plan.md stays words.\n\n```\ndocs/plan.md\n```",
  },
  decorators: [
    (Story) => (
      <ProseRefs.Provider value={{ scan: scanFile, open: pressed }}>
        <Story />
      </ProseRefs.Provider>
    ),
  ],
  beforeEach: () => {
    pressed.mockClear();
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("link", { name: "docs/plan.md" }));
    await expect(pressed).toHaveBeenCalledWith({ kind: "file", id: "docs/plan.md", title: "docs/plan.md" });
    await expect(canvas.getByText("docs/plan.md --draft", { selector: "code" })).toBeVisible();
    await expect(canvas.getAllByRole("link")).toHaveLength(1);
  },
};

export const RawHtml: Story = {
  args: {
    text: 'A tag is written <b>like this</b> and stays that way.\n\n<img src="x" onerror="alert(1)">',
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText("A tag is written <b>like this</b> and stays that way."),
    ).toBeVisible();
    await expect(canvas.getByText('<img src="x" onerror="alert(1)">')).toBeVisible();
    await expect(canvas.queryByRole("strong")).toBeNull();
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};

/** Empty text draws nothing rather than an empty block. */
export const Empty: Story = {
  args: { text: "  \n\n" },
  play: async ({ canvasElement }) => {
    await expect(canvasElement).toBeEmptyDOMElement();
  },
};

/** A single newline is a line break, as a forge draws a review comment. */
export const SoftLineBreaks: Story = {
  args: {
    text: "Restore the walk.\nThen rerun the check.\n\nA second paragraph.",
  },
  play: async ({ canvas }) => {
    const [first] = canvas.getAllByRole("paragraph");
    await expect(canvas.getAllByRole("paragraph")).toHaveLength(2);
    await expect(first?.innerText).toBe("Restore the walk.\nThen rerun the check.");
  },
};

/**
 * Prose inside `Clamped`, with a list in it. `-webkit-line-clamp` counts no
 * lines inside a flex box, so Prose is block flow: as a flex column this drew
 * whole and the control never appeared.
 */
export const ClampedThroughAList: Story = {
  args: {
    text:
      "The approach, in three moves:\n\n" +
      "- restore `forget_job` to the walk\n" +
      "- serve `DELETE /jobs/{id}` from `routes.rs`\n" +
      "- rerun the check against `main`\n\n" +
      "Each move is its own commit, so a reviewer can read them apart.",
  },
  render: (args) => (
    <Clamped lines={2}>
      <Prose {...args} />
    </Clamped>
  ),
  play: async ({ canvas }) => {
    await expect(await canvas.findByRole("button", { name: "View more" })).toBeVisible();
  },
};

/** One Judge verdict carrying every construct. */
const VERDICT =
  "## Verdict: the assertion was weakened\n\n" +
  "The Drone made `served_every_operation` pass by **narrowing the set it counts**, not by serving " +
  "`forget_job`. The route still answers *404* on a live Fleet.\n" +
  "Checked against `main` at `4f2c1ab`.\n\n" +
  "### What changed\n\n" +
  "1. The walk now skips `forget_job`\n" +
  "   - `crates/api/src/tests/served.rs:214`\n" +
  "   - the count dropped from 41 to 40\n" +
  "2. No route was added to `crates/api/src/routes.rs`\n" +
  "3. ~~The handler exists~~ The handler was never written\n\n" +
  "### Before and after\n\n" +
  "| Check | main | This step |\n" +
  "|---|---|---|\n" +
  "| `served_every_operation` | fails, 41 of 41 | passes, 40 of 40 |\n" +
  "| `routes_are_documented` | passes | passes |\n\n" +
  "> Every operation in `operations.toml` is served by exactly one route.\n\n" +
  "```rust\n" +
  "let routes: Vec<&Route> = ROUTES.iter()\n" +
  '    .filter(|r| r.operation != "forget_job")\n' +
  "    .collect();\n" +
  "assert_eq!(routes.len(), served.len());\n" +
  "```\n\n" +
  "### What the Drone should do\n\n" +
  "- [x] Restore `forget_job` to the walk\n" +
  "- [ ] Serve `DELETE /jobs/{id}` from `routes.rs`\n" +
  "- [ ] Rerun `armada check api_test`\n\n" +
  "The claim is in [the M4 acceptance test](https://forge.invalid/armada/blob/main/docs/m4.md).";

/**
 * **The whole treatment, on a card.** The owner's pick on 1 Oct 2026, "B, more
 * life": levels one and two at `--text-lg` over a rule, accent markers, a
 * header band on the table, a tinted callout for the quote, an accent edge on
 * the fenced block.
 */
export const AJudgesVerdict: Story = {
  args: { text: VERDICT },
  render: (args) => (
    <Card>
      <Prose {...args} />
    </Card>
  ),
  play: async ({ canvas, userEvent }) => {
    // Headings: their words, no `#`, and no entry in the page's outline.
    await expect(canvas.getByText("Verdict: the assertion was weakened")).toBeVisible();
    await expect(canvas.getByText("What the Drone should do")).toBeVisible();
    await expect(canvas.queryByRole("heading")).toBeNull();
    // Not `|`: the fenced Rust closes over `|r|`.
    await expect(canvas.queryByText(/[#`*~]/)).toBeNull();
    // Emphasis, inline code, and the soft break inside the first paragraph.
    await expect(canvas.getByRole("strong")).toHaveTextContent("narrowing the set it counts");
    await expect(canvas.getByRole("emphasis")).toHaveTextContent("404");
    await expect(canvas.getByRole("deletion")).toHaveTextContent("The handler exists");
    await expect(canvas.getAllByText("served_every_operation")).toHaveLength(2);
    const [opening] = canvas.getAllByRole("paragraph").filter((one) => one.textContent?.startsWith("The Drone"));
    await expect(opening?.innerText).toContain("live Fleet.\nChecked against");
    // An ordered list with a list inside it, and a task list.
    const lists = canvas.getAllByRole("list");
    await expect(lists.map((one) => one.tagName)).toEqual(["OL", "UL", "UL"]);
    const tasks = canvas.getAllByRole("checkbox");
    await expect(tasks).toHaveLength(3);
    await expect(tasks[0]).toBeChecked();
    // The table, the quote and the fenced block.
    await expect(canvas.getAllByRole("columnheader").map((cell) => cell.textContent)).toEqual([
      "Check",
      "main",
      "This step",
    ]);
    await expect(canvas.getByRole("blockquote")).toHaveTextContent(
      "Every operation in operations.toml is served by exactly one route.",
    );
    await expect(canvas.getByText(/assert_eq!\(routes\.len\(\), served\.len\(\)\);/)).toBeVisible();
    // The link hands its address over.
    await userEvent.click(canvas.getByRole("link", { name: "the M4 acceptance test" }));
    await expect(opened).toHaveBeenCalledWith("https://forge.invalid/armada/blob/main/docs/m4.md");
  },
};

const SHELL_BLOCK = "Check it:\n\n```sh\npnpm -C apps/desktop typecheck\n```\n\n```ts\nconst unrelated = 1;\n```";

/** A runner that prints two lines and exits with `code`, or never exits where `code` is `"hang"`. */
const runnerOf = (code: number | "hang"): CodeRunner => (_command, on) => {
  on({ kind: "out", text: "checking\n" });
  if (code === "hang") return { stop: () => on({ kind: "end", code: null }) };
  on({ kind: "out", text: "done\n" });
  on({ kind: "end", code });
  return { stop: () => {} };
};

/** **Run on a shell block only**, and its output under the block, with the exit on a mark. */
export const ARunnableShellBlock: Story = {
  args: { text: SHELL_BLOCK },
  decorators: [
    (Story) => (
      <ProseRunner.Provider value={runnerOf(0)}>
        <Story />
      </ProseRunner.Provider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    // One block is a shell block; the TypeScript one has no Run.
    await expect(canvas.getAllByRole("button", { name: "Run" })).toHaveLength(1);
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(canvas.getByRole("log", { name: "Output" })).toHaveTextContent("done");
    await expect(canvas.getByRole("img", { name: "Passed" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Run again" })).toBeVisible();
  },
};

/** A run that does not end offers Stop, and a stopped run reads as failed with no exit code. */
export const AStoppedRun: Story = {
  args: { text: SHELL_BLOCK },
  decorators: [
    (Story) => (
      <ProseRunner.Provider value={runnerOf("hang")}>
        <Story />
      </ProseRunner.Provider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await userEvent.click(canvas.getByRole("button", { name: "Stop" }));
    await expect(canvas.getByRole("img", { name: "Failed" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Run again" })).toBeVisible();
  },
};

/** Without a runner a shell block draws as any other block. */
export const NoRunnerNoRun: Story = {
  args: { text: SHELL_BLOCK },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Run" })).toBeNull();
  },
};

const UNTAGGED = [
  "```",
  "cd ~/Development/armada && \\",
  "  rm -r .armada/slots/slot-3/target && \\",
  "  armada worktree release slot-3 && \\",
  "  git branch -D sessions/old-spike",
  "```",
  "",
  "```",
  "src/a.ts",
  "src/b.ts",
  "```",
  "",
  "```",
  "const rows = items.filter((it) => it.ok);",
  "```",
  "",
  "```",
  "┌────────┐     ┌───────┐",
  "│ Bridge │────▶│ Fleet │",
  "└────────┘     └───────┘",
  "```",
  "",
  "```",
  "Run the tests and check the output.",
  "```",
].join("\n");

/** **A block with no language gets Run where it reads as shell**, and a file list, code, a diagram and prose do not. */
export const AnUntaggedBlockThatReadsAsShell: Story = {
  args: { text: UNTAGGED },
  decorators: [
    (Story) => (
      <ProseRunner.Provider value={runnerOf(0)}>
        <Story />
      </ProseRunner.Provider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getAllByRole("button", { name: "Run" })).toHaveLength(1);
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(canvas.getByRole("img", { name: "Passed" })).toBeVisible();
  },
};

/** What `readsAsShell` takes for shell and what it does not. A doubt is no Run. */
export const ReadsAsShell: Story = {
  args: { text: "" },
  play: async () => {
    const shell = [
      "cd ~/Development/armada && \\\n  rm -r x && \\\n  git branch -D y",
      "scripts/quick --changed",
      "./run.sh",
      "scripts/quick",
      "~/bin/tool --x",
      "/usr/bin/env node a",
      "$ pnpm install",
      "git status",
      "pnpm -C apps/desktop typecheck | tail -5",
      "cargo test -p x\ncargo clippy",
      "git add .",
      "cd ..",
      "FOO=1 node x.js",
      "ls | wc -l",
      "# note\nls -la",
    ];
    const not = [
      "const rows = items.filter((it) => it.ok);",
      '{\n  "a": 1\n}',
      "┌──┐\n│ a │\n└──┘",
      "Bridge -> Fleet",
      "A --> B",
      "foo --> bar",
      "Run the tests and check.",
      "run the tests",
      "see the docs.",
      "fn main() {\n}",
      "import os",
      "def f():\n  return 1",
      "x = 1",
      "pwd",
      "cd x &&",
      "src/foo.ts\nsrc/bar.ts",
      "/etc/hosts",
      "~/x/y",
      "",
    ];
    for (const one of shell) await expect(readsAsShell(one), one).toBe(true);
    for (const one of not) await expect(readsAsShell(one), one).toBe(false);
  },
};

/** Which inline code is a command the Session lists: its name, with or without arguments, and nothing that is only shaped like one. */
export const WhichCodeIsASlashCommand: Story = {
  args: { text: "" },
  play: async () => {
    const names = new Set(["update-armada", "reload-plugins", "model", "claude-md-management:revise-claude-md"]);
    for (const one of ["/update-armada", "/reload-plugins", "/model opus", "/claude-md-management:revise-claude-md"]) {
      await expect(isSlashCommand(one, names), one).toBe(true);
    }
    for (const one of ["/tmp", "/usr/bin", "/Update-armada", "/update-armada/x", "update-armada", "/", "/model\nx"]) {
      await expect(isSlashCommand(one, names), one).toBe(false);
    }
  },
};

const SLASH = "Run `/update-armada`, then `/reload-plugins`, and keep scratch in `/tmp`.\n\n```\n/update-armada\n```";

/** **A listed slash command in inline code is pressable**, says Sent, and sends what is written; `/tmp` and a fenced one are not. */
export const ASlashCommandInTheThread: Story = {
  args: { text: SLASH },
  decorators: [
    (Story) => (
      <ProseCommand.Provider value={{ send: opened, names: new Set(["update-armada", "reload-plugins"]) }}>
        <Story />
      </ProseCommand.Provider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    // The two commands in code, and Copy on the fenced block.
    await expect(canvas.getAllByRole("button")).toHaveLength(3);
    await expect(canvas.getAllByRole("button", { name: "Copy" })).toHaveLength(1);
    await userEvent.click(canvas.getByRole("button", { name: "/update-armada" }));
    await expect(opened).toHaveBeenCalledWith("/update-armada");
    await expect(canvas.getByRole("img", { name: "Sent" })).toBeVisible();
  },
};

/** Where the Session is not a terminal one there is no command to send to, and the code is plain. */
export const NoTerminalNoSlashCommand: Story = {
  args: { text: SLASH },
  play: async ({ canvas }) => {
    // Only the fenced block's Copy: nothing to send to.
    await expect(canvas.getAllByRole("button").map((one) => one.getAttribute("aria-label"))).toEqual(["Copy"]);
  },
};

const FENCED = "Run it:\n\n```bash\npnpm -C apps/desktop typecheck\n```\n\nThe readings:\n\n```\nread 1 -> 12:00:00.99\nread 2 -> 12:00:01.01\n```";

/** The page's clipboard, replaced by one that keeps what it was given (or refuses it), and the way back. */
function stubClipboard(refuse = false) {
  const written: string[] = [];
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), refuse ? Promise.reject(new Error("refused")) : Promise.resolve()) },
  });
  return { written, restore: () => void Reflect.deleteProperty(navigator, "clipboard") };
}

/** **Every fenced block has Copy**, beside Run on a command; it writes the source less the fence's closing newline, then says Copied. */
export const CopyOnEveryFencedBlock: Story = {
  args: { text: FENCED },
  decorators: [
    (Story) => (
      <ProseRunner.Provider value={runnerOf(0)}>
        <Story />
      </ProseRunner.Provider>
    ),
  ],
  play: async ({ canvas, userEvent }) => {
    const { written, restore } = stubClipboard();
    try {
      const command = within(canvas.getByRole("group", { name: "pnpm -C apps/desktop typecheck" }));
      await expect(command.getByRole("button", { name: "Run" })).toBeInTheDocument();
      await expect(canvas.getAllByRole("button", { name: "Copy" })).toHaveLength(2);
      await userEvent.click(command.getByRole("button", { name: "Copy" }));
      await expect(written).toEqual(["pnpm -C apps/desktop typecheck"]);
      await expect(command.getByRole("button", { name: "Copied" })).toBeVisible();
      // The plain block has no Run, and Copy is the one button on it.
      await userEvent.click(canvas.getAllByRole("button", { name: "Copy" })[0]!);
      await expect(written[1]).toBe("read 1 -> 12:00:00.99\nread 2 -> 12:00:01.01");
      await waitFor(() => expect(canvas.getAllByRole("button", { name: "Copy" })).toHaveLength(2), { timeout: 3000 });
    } finally {
      restore();
    }
  },
};

/** A write the clipboard refuses is not a copy: the button stays Copy. */
export const CopyRefused: Story = {
  args: { text: FENCED },
  play: async ({ canvas, userEvent }) => {
    const { written, restore } = stubClipboard(true);
    try {
      await userEvent.click(canvas.getAllByRole("button", { name: "Copy" })[0]!);
      await expect(written).toHaveLength(1);
      await expect(canvas.queryByRole("button", { name: "Copied" })).toBeNull();
    } finally {
      restore();
    }
  },
};

/** Another panel names the act: `label` is the accessible name, and Copied is still Copied. */
export const ACopyActWithALabel: Story = {
  render: () => <CopyAct text="pnpm install" label="Copy step 2 command" />,
  play: async ({ canvas, userEvent }) => {
    const { restore } = stubClipboard();
    try {
      await userEvent.click(canvas.getByRole("button", { name: "Copy step 2 command" }));
      await expect(canvas.getByRole("button", { name: "Copied" })).toBeVisible();
    } finally {
      restore();
    }
  },
};
