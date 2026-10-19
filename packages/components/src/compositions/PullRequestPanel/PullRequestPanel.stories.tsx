import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import type { PullRequestPage } from "@armada/protocol";

import type { PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestPanel, type PullRequestPatches } from "./PullRequestPanel";

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
  reviewers: [
    { login: "marta", kind: "person", verdict: "approved", name: "Marta Ruiz", url: "https://github.com/marta" },
    { login: "reviewer", kind: "person", verdict: "changes_requested" },
    { login: "nick", kind: "person", verdict: "commented" },
    { login: "dev", kind: "person" },
    { login: "platform", kind: "team" },
  ],
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
    await expect(canvas.getAllByRole("img", { name: "Approved" }).length).toBeGreaterThan(0);
    await expect(canvas.getAllByRole("img", { name: "Changes requested" }).length).toBeGreaterThan(0);
    await expect(canvas.getByText("Approved", { selector: ".armada-pr-status__fact" })).toBeInTheDocument();
  },
};

/** Who is a reviewer, floating off the panel's leading edge: a row each, a team marked, and the verdict as a glyph named on hover. A reviewer with no verdict is waiting. */
export const Reviewers: Story = {
  args: withPage,
  play: async ({ canvas, args, userEvent }) => {
    const card = within(canvas.getByRole("region", { name: "Reviewers" }));
    await expect(card.getAllByRole("listitem")).toHaveLength(5);
    await expect(card.getByRole("img", { name: "Team" })).toBeInTheDocument();
    await expect(card.getByRole("img", { name: "Waiting" })).toBeInTheDocument();
    await expect(card.getByRole("img", { name: "Commented" })).toBeInTheDocument();
    // A person with a url is a link through the opener; one with none stays text.
    await userEvent.click(card.getByRole("button", { name: "marta" }));
    await expect(args.onOpenLink).toHaveBeenCalledWith("https://github.com/marta");
    await expect(card.queryByRole("button", { name: "dev" })).toBeNull();
    await userEvent.hover(card.getByRole("button", { name: "marta" }));
    await expect(await within(document.body).findByText("Marta Ruiz")).toBeInTheDocument();
  },
};

/** No reviewer named: the plate is not drawn. */
export const NoReviewers: Story = {
  args: { page: { ...PAGE, reviewers: [] } },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("region", { name: "Reviewers" })).toBeNull();
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

const NO_PATCH = "The forge sent no patch for this file. It is binary, or the change is too large for the forge to show.";
const lines = (removed: string, added: string) => [
  { kind: "hunk" as const, text: "@@ -1,2 +1,2 @@" },
  { kind: "removed" as const, text: `-${removed}` },
  { kind: "added" as const, text: `+${added}` },
  { kind: "context" as const, text: " export {};" },
];
const diffOf = (path: string, removed: string, added: string) => ({ path, diff: { files: [{ path, lines: lines(removed, added) }] } });

/** Every file's diff as the host drew them, the last a file the forge sent no patch for. */
const PATCHES: PullRequestPatches = {
  state: "read",
  files: [
    diffOf(PAGE.files[0]!.path, "const opens = \"browser\";", "const opens = setting();"),
    diffOf(PAGE.files[1]!.path, "", "export function PullRequestTabs() {}"),
    diffOf(PAGE.files[2]!.path, "export function PullRequestCard() {}", ""),
    { path: PAGE.files[3]!.path, diff: { files: [], emptyNote: NO_PATCH } },
  ],
};

const filesOpen = { ...withPage, tab: "files", onTab: fn() };
const list = () => within(document.body).queryByRole("dialog", { name: "Files" });

/** The Files tab with every file's diff, one under the other, and the file list beside the panel. A press marks the file. */
export const FilesWithDiffs: Story = {
  args: { ...filesOpen, patches: PATCHES },
  play: async ({ canvas, userEvent }) => {
    const beside = within(list()!);
    const row = () => beside.getByRole("button", { name: (name) => name.includes("PullRequestCard.tsx") });
    // The shared folders are drawn once, and a file is its name under them.
    await expect(beside.getByText("packages/components/src/compositions/")).toBeVisible();
    await expect(row()).not.toHaveAttribute("aria-current");
    await userEvent.click(row());
    await expect(row()).toHaveAttribute("aria-current", "true");
    await expect(canvas.getByRole("region", { name: PAGE.files[3]!.path })).toHaveTextContent(NO_PATCH);
  },
};

/** The patches are being read: each file's header from the page, a stand-in for its lines. */
export const FilesReading: Story = {
  args: { ...filesOpen, patches: { state: "reading" } },
};

/** Fleet refused the read: its words in the tab, the list still beside the panel. */
export const FilesRefused: Story = {
  args: { ...filesOpen, patches: { state: "refused", said: "The forge refused the read of these files." } },
};

/** At the window floor a sheet beside the panel would lie over it, so the list heads the tab. */
export const FilesAtTheFloor: Story = {
  args: { ...filesOpen, patches: PATCHES, floor: true },
  play: async ({ canvas }) => {
    await expect(list()).toBeNull();
    await expect(canvas.getByRole("list", { name: "Files" })).toBeInTheDocument();
  },
};

/** Leaving Files puts the list away. */
export const LeavingFiles: Story = {
  args: { ...withPage, patches: PATCHES },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("tab", { name: "Files" }));
    await expect(list()).not.toBeNull();
    await userEvent.click(canvas.getByRole("tab", { name: "Commits" }));
    await expect(list()).toBeNull();
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
const waiting = { reading: {}, number: 418, address: "https://forge.example/harbor-labs/tidewater/pull/418", repository: "harbor-labs/tidewater", onOpenBrowser: opener, onClose: fn() };

/** Pressed, and the forge has not answered: the head, the title and branch the press carried, stand-ins for the rest. */
export const Reading: Story = {
  render: () => <PullRequestPanel {...waiting} title="Keep the tide tables a day" branch="ines/tide-cache" />,
  play: async ({ canvas }) => {
    await expect(within(document.body).getByRole("dialog", { name: "Pull request #418" })).toBeInTheDocument();
    await expect(canvas.getByRole("status", { name: "Reading pull request #418" })).toBeInTheDocument();
    await expect(canvas.getByRole("heading", { name: "Keep the tide tables a day" })).toBeInTheDocument();
    await expect(canvas.queryByRole("tab")).toBeNull();
  },
};

/** Pressed from a bare number: nothing to draw but the head until the read answers. */
export const ReadingFromANumber: Story = {
  render: () => <PullRequestPanel {...waiting} />,
};

/** The forge answered with nothing: the panel stays, says so, and Open in browser still opens it. */
export const NotRead: Story = {
  render: () => <PullRequestPanel {...waiting} branch="ines/tide-cache" reading={{ said: "The forge did not answer for this pull request." }} />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("The forge did not answer for this pull request.");
    await expect(canvas.queryByRole("status")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open in browser" }));
    await expect(opener).toHaveBeenCalledWith(waiting.address);
  },
};
