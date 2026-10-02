import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Prose } from "../../primitives/Prose/Prose";
import { ToolName } from "../ToolName/ToolName";
import { LogEntry, PayloadLine } from "./LogEntry";

const meta: Meta<typeof LogEntry> = {
  title: "Compositions/Log entry",
  component: LogEntry,
  decorators: [
    // The chapter the log streams inside. Every surface value on the row is
    // picked against --bg-sunken, including the payload's step below it.
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 12)",
          padding: "var(--space-2)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof LogEntry>;

/**
 * Armada's own turn. **The stream carries all three actors**, and this is the
 * one the build did not have: a transcript of the Drone alone cannot show what
 * it was told, or when.
 *
 * Sans, because an injected turn is a sentence somebody wrote.
 */
export const Armada: Story = {
  args: {
    at: "14:22:07",
    actor: "armada",
    message: "Go on to Implement.",
    onToggle: () => {},
    payloadId: "entry-armada",
  },
};

/**
 * The Drone's turn — a tool call. Mono, because it is a command: sans names
 * work, mono names machinery, and the wire already knows which this is.
 */
export const Drone: Story = {
  args: {
    at: "14:26:31",
    actor: "drone",
    message: "Edit  packages/settings/src/selectors.ts",
    mono: true,
    onToggle: () => {},
    payloadId: "entry-drone",
  },
};

/**
 * The Drone saying something, in markdown. **Its blocks run on along the one
 * line**: emphasis and code draw, the list's items follow the paragraph rather
 * than stacking under it, and the row clips as a plain sentence would.
 */
export const DroneMarkdown: Story = {
  name: "The Drone writes markdown",
  args: {
    at: "14:26:35",
    actor: "drone",
    message: (
      <Prose text={"## The split\nSplitting the reducer.\n**The public signature** stays put:\n\n- `selectSettings` keeps its name\n- the tests move with it"} />
    ),
    onToggle: () => {},
    payloadId: "entry-drone-markdown",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The public signature").tagName).toBe("STRONG");
    await expect(canvas.getByText("selectSettings").tagName).toBe("CODE");
    await expect(canvas.getByText("the tests move with it").tagName).toBe("LI");
  },
};

/**
 * A plain row above a markdown one holding a heading, a list and a
 * single-newline break. **The two are one line each**: a `<br>` the row did not
 * hide once made it 48px against its 20px line, and nothing measured it.
 */
export const MarkdownHoldsTheLine: Story = {
  name: "Markdown holds the one line",
  render: () => (
    <div style={{ display: "flex", flexDirection: "column" }}>
      <LogEntry at="14:26:31" actor="drone" message="Splitting the reducer." onToggle={() => {}} />
      <LogEntry
        at="14:26:35"
        actor="drone"
        message={<Prose text={"## The split\nSplitting the reducer.\nThe *public signature* stays:\n\n- one\n- two"} />}
        onToggle={() => {}}
      />
    </div>
  ),
  play: async ({ canvas }) => {
    const [plain, marked] = canvas.getAllByRole("button");
    await expect(marked!.getBoundingClientRect().height).toBe(plain!.getBoundingClientRect().height);
  },
};

/**
 * The same call, with what it changed and how long it took. **Three hues on
 * one mono line**: the tool's family, the lines added, the lines removed. The
 * path between them stays neutral, because a line where everything is coloured
 * says nothing. A zero is not drawn — `−0` reads as a value that failed to
 * arrive. #1196.
 */
export const AnEditWithItsSizes: Story = {
  name: "An edit with its sizes",
  args: {
    at: "14:26:31",
    actor: "drone",
    message: (
      <>
        <ToolName tool="Edit" />
        {"  packages/settings/src/selectors.ts"}
      </>
    ),
    mono: true,
    sized: { added: 8, deleted: 5 },
    took: "71ms",
    onToggle: () => {},
    payloadId: "entry-sized",
  },
};

/**
 * Fleet reporting on itself. A heartbeat and a Check result are Fleet's, not
 * the Drone's, and they belong in the same stream because they happened
 * between the Drone's turns in time — which is the whole reason this is one
 * log rather than three.
 */
export const Fleet: Story = {
  args: {
    at: "14:30:28",
    actor: "fleet",
    message: "Heartbeat — the Drone has been quiet for 48 seconds",
    onToggle: () => {},
    payloadId: "entry-fleet",
  },
};

/**
 * Closed, which is every row until it is pressed. The chevron says the row
 * opens.
 */
export const Closed: Story = {
  args: {
    at: "14:23:11",
    actor: "drone",
    message: "Read  packages/settings/src/reducer.ts",
    mono: true,
    open: false,
    onToggle: () => {},
    payloadId: "entry-closed",
  },
};

/**
 * Open, with its payload — the way a bash line opens in a terminal.
 *
 * The payload keeps its own newlines and scrolls sideways rather than
 * wrapping: a column of compiler output reflowed turns one long line into
 * three and loses which was which.
 */
export const OpenWithItsPayload: Story = {
  args: {
    at: "14:29:40",
    actor: "drone",
    message: "Bash  cargo build --workspace --locked",
    mono: true,
    open: true,
    onToggle: () => {},
    payloadId: "entry-open",
    payload: (
      <>
        <PayloadLine named="echo">$ cargo build --workspace --locked</PayloadLine>
        <PayloadLine>   Compiling armada-settings v0.1.0 (packages/settings)</PayloadLine>
        <PayloadLine>   Compiling armada-fleet v0.1.0 (crates/fleet)</PayloadLine>
        <PayloadLine named="passed">
          {"    Finished `dev` profile [unoptimized] in 47.61s"}
        </PayloadLine>
        <PayloadLine named="meta">exit 0 · 47.61s · in .armada/worktrees/job_2d90bb</PayloadLine>
      </>
    ),
  },
};

/**
 * A Drone still producing the line. The running dot and nothing else — a
 * spinner in a stream where a row arrives every second is motion nobody can
 * read.
 */
export const StillProducing: Story = {
  args: {
    at: "14:31:58",
    actor: "drone",
    message: "thinking",
    working: true,
    onToggle: () => {},
    payloadId: "entry-working",
  },
};

/**
 * An open line with nothing under it. **One sentence, and no transport in
 * it** — the build rendered "The call's arguments were cut before Bridge saw
 * them" here, which makes the one gesture whose entire purpose is seeing the
 * payload into a lie.
 *
 * Where an argument is genuinely too large to send whole, this shows what was
 * sent with its real size and offers the rest. It never reports that Bridge
 * was given nothing.
 */
export const OpenAndEmpty: Story = {
  args: {
    at: "14:32:40",
    actor: "fleet",
    message: "Heartbeat — the Drone has been quiet for 48 seconds",
    open: true,
    onToggle: () => {},
    payloadId: "entry-empty",
    payloadAbsent: "A heartbeat carries only its time.",
  },
};

/**
 * The stream as the drawing has it: three actors, in the order things
 * happened, with one line open.
 *
 * The time column never moves. That is the point of the fixed first two
 * columns — in a log where a row arrives every second, a column that shifted
 * on a long message is the complaint the v1 failure log recorded nine times.
 */
export const TheStream: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column" }}>
      <LogEntry at="14:22:07" actor="armada" message="Go on to Implement." onToggle={() => {}} />
      <LogEntry
        at="14:26:31"
        actor="drone"
        mono
        message="Edit  packages/settings/src/selectors.ts"
        onToggle={() => {}}
      />
      <LogEntry
        at="14:29:40"
        actor="drone"
        mono
        open
        message="Bash  cargo build --workspace --locked"
        onToggle={() => {}}
        payload={
          <>
            <PayloadLine named="echo">$ cargo build --workspace --locked</PayloadLine>
            <PayloadLine>   Compiling armada-settings v0.1.0 (packages/settings)</PayloadLine>
            <PayloadLine named="passed">
              {"    Finished `dev` profile [unoptimized] in 47.61s"}
            </PayloadLine>
            <PayloadLine named="meta">exit 0 · 47.61s · in .armada/worktrees/job_2d90bb</PayloadLine>
          </>
        }
      />
      <LogEntry
        at="14:30:28"
        actor="fleet"
        message="Heartbeat — the Drone has been quiet for 48 seconds"
        onToggle={() => {}}
      />
      <LogEntry at="14:31:58" actor="drone" working message="thinking" onToggle={() => {}} />
    </div>
  ),
};
