import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { PullRequestPage } from "@armada/protocol";

import type { PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestPanel, PullRequestPanelWaiting } from "./PullRequestPanel";

const ADDRESS = "https://github.com/mele-labs/armada/pull/";

/** A pull request as its panel draws it, with its status, queue and failing checks as plates off the leading edge. */
const meta = {
  title: "Compositions/Pull request panel",
  component: PullRequestPanel,
  parameters: { layout: "fullscreen" },
  args: {
    number: 2209,
    title: "Open pull requests in Armada",
    branch: "claude/pr-panel-redesign",
    address: `${ADDRESS}2209`,
    repository: "mele-labs/armada",
    state: "open",
    checks: { state: "passed" },
    auto: false,
    onAct: fn(),
    onOpenLink: fn(),
    onClose: fn(),
  },
} satisfies Meta<typeof PullRequestPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

const row = (place: number, number: number, title: string, state: "queued" | "awaiting_checks" | "mergeable"): PlaceRow => ({
  place,
  number,
  title,
  url: `${ADDRESS}${number}`,
  mark: { line: "queue", state },
});

const QUEUE: PlaceRow[] = [
  row(1, 2201, "Commander repair onto the Job's branch", "mergeable"),
  row(2, 2204, "Keep the dispatch bar under Helm", "awaiting_checks"),
  { ...row(3, 2209, "Open pull requests in Armada", "queued"), current: true },
  row(4, 2211, "Settings: per-repo merge method", "queued"),
  row(5, 2212, "Tidy the verdict sheet", "queued"),
  row(6, 2215, "Wire Cockpit holds", "queued"),
  row(7, 2216, "Spike 031 notes", "queued"),
];

/** In the merge queue, third: the place figure on the status plate, the queue plate with this pull request current. */
export const InTheMergeQueue: Story = {
  args: { queued: true, place: QUEUE },
};

/** Every check passed: the Merge act and the status plate alone. */
export const ReadyToMerge: Story = {};

/** The description its author wrote, drawn as Markdown at the end of the body. */
export const WithADescription: Story = {
  args: {
    body: "Pull request links open the same way everywhere, behind a setting: in Armada or in the browser.\n\n- `links.openPullRequestsIn` picks which\n- Open in browser stays on the panel",
  },
};

/** The forge's checks failed: the failing plate names them, and Notify Job tells the Session. */
export const ChecksFailed: Story = {
  args: { checks: { state: "failed", failing: ["test · bridge", "verify-foundations"] }, onSendBack: fn() },
};

const ago = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

/** What Fleet serves for the tabs and the facts the panel drops without it. */
const PAGE: PullRequestPage = {
  manifest_id: "armada",
  number: 2209,
  base: "main",
  author: "nick",
  created_at: ago(125),
  additions: 412,
  deletions: 88,
  mergeable: "mergeable",
  review: "approved",
  approvals: 1,
  conversation: [
    { kind: "comment", author: "claude", at: ago(95), body: "Open pull requests go through `openPullRequest`:\n\n- the setting is read once\n- the browser stays one press away" },
    { kind: "review", author: "reviewer", at: ago(60), verdict: "changes_requested", body: "The setting needs a default in `settings.toml`." },
    { kind: "review", author: "reviewer", at: ago(40), verdict: "approved", body: "" },
    { kind: "review", author: "nick", at: ago(12), verdict: "commented", body: "Queue ETA waits on the forge." },
  ],
  checks: [
    { name: "build", state: "passed", seconds: 131, address: `${ADDRESS}2209/checks/1` },
    { name: "test \u00b7 rust", state: "passed", seconds: 242, address: `${ADDRESS}2209/checks/2` },
    { name: "test \u00b7 bridge", state: "failed", seconds: 192, address: `${ADDRESS}2209/checks/3` },
    { name: "verify-foundations", state: "running" },
    { name: "lint", state: "passed", seconds: 31 },
    { name: "Claude Approvals", state: "skipped" },
  ],
  files: [
    { path: "apps/desktop/src/renderer/src/session-pull-request.tsx", status: "modified", additions: 64, deletions: 12 },
    { path: "packages/components/src/compositions/PullRequestPanel/PullRequestTabs.tsx", status: "added", additions: 188, deletions: 0 },
    { path: "packages/components/src/compositions/PullRequestCard/PullRequestCard.tsx", status: "removed", additions: 0, deletions: 74 },
    { path: "crates/config/settings.toml", status: "renamed", additions: 3, deletions: 3 },
  ],
  commits: [
    { sha: "3fc86fa", headline: "Draw the pull request tabs", author: "nick", at: ago(130) },
    { sha: "ed96c0f", headline: "Keep the comment box under the tabs", author: "claude", at: ago(100) },
    { sha: "3d35bd9", headline: "Let the protocol gate read the route table" },
  ],
};

const withPage: Story["args"] = { page: PAGE, body: "Pull request links open the same way everywhere, behind a setting: in Armada or in the browser.", onSendBack: fn() };

/** The page read: author, how long ago, base branch and diff size on the meta line, the reviews and conflicts on the status plate, and the description first in the Conversation tab. */
export const Conversation: Story = {
  args: withPage,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("tab", { name: "Conversation", selected: true })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Approved" })).toBeInTheDocument();
    await expect(canvas.getByRole("img", { name: "Changes requested" })).toBeInTheDocument();
    await expect(canvas.getByText("Approved", { selector: ".armada-pr-status__fact" })).toBeInTheDocument();
  },
};

/** The Checks tab: state, name, how long it ran, and the log where the forge keeps one. */
export const ChecksTab: Story = {
  args: withPage,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Checks" }));
    await expect(canvas.getByRole("img", { name: "Failed" })).toBeInTheDocument();
    await expect(canvas.getByText("2m 11s")).toBeInTheDocument();
    await expect(canvas.getAllByRole("button", { name: /^Open the log of/ })).toHaveLength(3);
  },
};

/** The Files tab: path, status and the lines added and removed. */
export const FilesTab: Story = {
  args: withPage,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Files" }));
    await expect(canvas.getByRole("img", { name: "Removed" })).toBeInTheDocument();
  },
};

/** A file pressed in the Files tab: its diff in a panel to the left, the row marked, and the plates in the body. */
export const AFileOpen: Story = {
  args: {
    ...withPage,
    onFile: fn(),
    file: {
      path: PAGE.files[0]?.path ?? "",
      diff: {
        files: [
          {
            path: PAGE.files[0]?.path ?? "",
            lines: [
              { kind: "hunk", text: "@@ -1,2 +1,2 @@" },
              { kind: "removed", text: "-const before = 1;" },
              { kind: "added", text: "+const after = 2;" },
            ],
          },
        ],
      },
    },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Files" }));
    const path = PAGE.files[0]?.path ?? "";
    const row = () => canvas.getByRole("button", { name: (name) => name.includes(path) });
    await expect(within(document.body).getByRole("dialog", { name: path })).toBeInTheDocument();
    await expect(row()).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(row());
    await expect(args.onFile).toHaveBeenCalledWith(null);
  },
};

/** The Commits tab: the short sha in mono, the headline, the author where the forge gave one. */
export const CommitsTab: Story = {
  args: withPage,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Commits" }));
    await expect(canvas.getByText("3fc86fa")).toBeInTheDocument();
  },
};

/** Conflicts and a review still required, with the comment box pinned under the panel. */
export const ConflictsAndReviewRequired: Story = {
  args: {
    ...withPage,
    page: { ...PAGE, mergeable: "conflicting", review: "required", approvals: 0 },
    composer: { kind: "comment", onKind: fn(), value: "", onChange: fn(), keep: "story:pull-request", onSend: fn() },
  },
};

/** An approval with no words sends; a comment with none does not. */
export const ApproveWithNoWords: Story = {
  args: { ...withPage, composer: { kind: "approve", onKind: fn(), value: "", onChange: fn(), keep: "story:approve", onSend: fn() } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Approve" })).toBeEnabled();
  },
};

export const CommentWithNoWords: Story = {
  args: { ...withPage, composer: { kind: "comment", onKind: fn(), value: " ", onChange: fn(), keep: "story:comment", onSend: fn() } },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Comment" })).toBeDisabled();
  },
};

/** The write is out: the send is the pending control. */
export const WriteOut: Story = {
  args: { ...withPage, composer: { kind: "request_changes", onKind: fn(), value: "The setting needs a default.", onChange: fn(), keep: "story:out", pending: true, onSend: fn() } },
};

/** Fleet refused the write: its words sit above the box, and the draft stays. */
export const WriteRefused: Story = {
  args: {
    ...withPage,
    composer: { kind: "comment", onKind: fn(), value: "Looks right to me.", onChange: fn(), keep: "story:refused", said: "Pull request is locked.", onSend: fn() },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Pull request is locked.");
    await expect(canvas.getByRole("textbox", { name: "Comment" })).toHaveValue("Looks right to me.");
  },
};

/** The page could not be read: the panel stays as it was, with the refusal's words and no tabs. */
export const PageRefused: Story = {
  args: { pageSaid: "The forge did not answer." },
};

/** Open in browser leaves Armada: it goes to the browser's opener, never to the link handler that would open this same panel again. */
export const OpenInBrowser: Story = {
  args: { onOpenBrowser: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open in browser" }));
    await expect(args.onOpenBrowser).toHaveBeenCalledWith(`${ADDRESS}2209`);
    await expect(args.onOpenLink).not.toHaveBeenCalled();
  },
};

/** The browser refused: its words sit under the acts. */
export const BrowserRefused: Story = {
  args: { onOpenBrowser: fn(), browserSaid: "This machine did not open the address: no browser is set." },
};

const opener = fn();
const waiting = { number: 418, address: "https://forge.example/harbor-labs/tidewater/pull/418", repository: "harbor-labs/tidewater", onOpenBrowser: opener, onClose: fn() };

/** Pressed, and the forge has not answered: the head, the title and branch the press carried, stand-ins for the rest. */
export const Reading: Story = {
  render: () => <PullRequestPanelWaiting {...waiting} title="Keep the tide tables a day" branch="ines/tide-cache" />,
  play: async ({ canvas }) => {
    await expect(within(document.body).getByRole("dialog", { name: "Pull request #418" })).toBeInTheDocument();
    await expect(canvas.getByRole("status", { name: "Reading pull request #418" })).toBeInTheDocument();
    await expect(canvas.getByRole("heading", { name: "Keep the tide tables a day" })).toBeInTheDocument();
    await expect(canvas.queryByRole("tab")).toBeNull();
  },
};

/** Pressed from a bare number: nothing to draw but the head until the read answers. */
export const ReadingFromANumber: Story = {
  render: () => <PullRequestPanelWaiting {...waiting} />,
};

/** The forge answered with nothing: the panel stays, says so, and Open in browser still opens it. */
export const NotRead: Story = {
  render: () => <PullRequestPanelWaiting {...waiting} branch="ines/tide-cache" said="The forge did not answer for this pull request." />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("The forge did not answer for this pull request.");
    await expect(canvas.queryByRole("status")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open in browser" }));
    await expect(opener).toHaveBeenCalledWith(waiting.address);
  },
};
