import type { Meta, StoryObj } from "@storybook/react-vite";
import { useLayoutEffect } from "react";
import { expect, fireEvent, fn, screen, userEvent, waitFor, within } from "storybook/test";

import { MediaFrame, ShownVideo, type ShownMedia } from "./MediaRow";
import { SessionThread } from "./SessionThread";
// Storybook's plain CSS pipeline drops `index.css`'s @imports after its first rule, this one among them; the app's Tailwind inlines them.
import "./MediaRow.css";

/** A picture of a known size, so a story fetches nothing. */
const PICTURE =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="teal"/></svg>',
  );
/** A real video, two seconds of one colour: the player's own events are what these stories read. */
const VIDEO = "data:video/webm;base64,GkXfo59ChoEBQveBAULygQRC84EIQoKEd2VibUKHgQJChYECGFOAZwEAAAAAAAO9EU2bdLpNu4tTq4QVSalmU6yBoU27i1OrhBZUrmtTrIHWTbuMU6uEElTDZ1OsggEyTbuMU6uEHFO7a1OsggNe7AEAAAAAAABZAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAVSalmsCrXsYMPQkBNgIxMYXZmNjMuMS4xMDJXQYxMYXZmNjMuMS4xMDJEiYhAn0AAAAAAABZUrmvXrgEAAAAAAABO14EBc8WIyg0TegRUMt6cgQAitZyDdW5kiIEAhoVWX1ZQOIOBASPjg4QL68IA4JCwgRC6gRCagQJVsIRVuYEBVe6BAOwBAAAAAAAAAgAAElTDZ/pzc59jwIBnyJlFo4dFTkNPREVSRIeMTGF2ZjYzLjEuMTAyc3PVY8CLY8WIyg0TegRUMt5nyKBFo4dFTkNPREVSRIeTTGF2YzYzLjEuMTAyIGxpYnZweGfIoUWjiERVUkFUSU9ORIeTMDA6MDA6MDIuMDAwMDAwMDAwAB9DtnVBp+eBAKO7gQAAgLACAJ0BKhAAEAAARwiFhYiFhIgCAgJ1qgP4Agz9+wD++2iX/z0zXmD/Jz/9uR/tyP9uR/+2wgCjlYEAyACxAQABEBAAGAAYWC/0AAgAAKO7gQGQgLACAJ0BKhAAEAAARwiFhYiFhIgCAgJ1qgP4Agz9+wD++2iX/z0zXmD/Jz/9uR/tyP9uR/+2wgCjlYECWACxAQABEBAAGAAYWC/0AAgAAKO7gQMggLACAJ0BKhAAEAAARwiFhYiFhIgCAgJ1qgP4Agz9+wD++2iX/z0zXmD/Jz/9uR/tyP9uR/+2wgCjlYED6ACxAQABEBAAGAAYWC/0AAgAAKO7gQSwgLACAJ0BKhAAEAAARwiFhYiFhIgCAgJ1qgP4Agz9+wD++2iX/z0zXmD/Jz/9uR/tyP9uR/+2wgCjlYEFeACxAQABEBAAGAAYWC/0AAgAAKO7gQZAgLACAJ0BKhAAEAAARwiFhYiFhIgCAgJ1qgP4Agz9+wD++2iX/z0zXmD/Jz/9uR/tyP9uR/+2wgCjlYEHCACxAQABEBAAGAAYWC/0AAgAABxTu2vau4+zgQC3iveBAfGCAbHwgQO7kLOCAZC3iveBAfGCAbHwgVe7kLOCAyC3iveBAfGCAbHwgau7kLOCBLC3iveBAfGCAbHwgf+7kbOCBkC3i/eBAfGCAbHwggFT";

const PATH = "/work/shown/run.png";
const NAME = "run.png";

/** Stands in for the browser's observer: the row reads only when a story says it came near. */
const observers: Array<{
  come: () => void;
  gone: boolean;
}> = [];
function WatchedByHand({ children }: { children: React.ReactNode }) {
  useLayoutEffect(() => {
    const real = window.IntersectionObserver;
    window.IntersectionObserver = class {
      private held = {
        come: () => {},
        gone: false,
      };
      constructor(seen: IntersectionObserverCallback) {
        this.held.come = () =>
          seen(
            [
              {
                isIntersecting: true,
              } as IntersectionObserverEntry,
            ],
            this as unknown as IntersectionObserver,
          );
        observers.push(this.held);
      }
      observe() {}
      unobserve() {}
      takeRecords() {
        return [];
      }
      disconnect() {
        this.held.gone = true;
      }
    } as unknown as typeof IntersectionObserver;
    return () => {
      window.IntersectionObserver = real;
    };
  }, []);
  return <>{children}</>;
}

const comeNear = () => {
  for (const one of observers.splice(0)) if (!one.gone) one.come();
};

/** Reads as the host does, answered when a story says so. */
function heldRead(answer: ShownMedia) {
  let release = () => {};
  const held = new Promise<void>((done) => {
    release = done;
  });
  const onRead = fn(async (_path: string) => {
    await held;
    return answer;
  });
  return {
    onRead,
    release,
  };
}

const meta: Meta<typeof MediaFrame> = {
  title: "Compositions/Media row",
  component: MediaFrame,
  args: {
    path: PATH,
    name: NAME,
  },
  decorators: [
    (Story) => (
      <WatchedByHand>
        <div data-testid="host" style={{ width: "calc(var(--space-12) * 10)" }}>
          <Story />
        </div>
      </WatchedByHand>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof MediaFrame>;

const heldPicture = heldRead({
  kind: "picture",
  url: PICTURE,
});

/** **Nothing is read until the row is near**, and its height is the same waiting and loaded, so the thread never moves. */
export const ReadsWhenNearAtOneHeight: Story = {
  args: {
    onRead: heldPicture.onRead,
  },
  play: async ({ canvas }) => {
    const host = canvas.getByTestId("host");
    await expect(heldPicture.onRead).not.toHaveBeenCalled();
    const waiting = host.getBoundingClientRect().height;
    // Past a skeleton's own line: the frame holds it open.
    await expect(waiting).toBeGreaterThan(100);

    comeNear();
    await waitFor(() => expect(heldPicture.onRead).toHaveBeenCalledTimes(1));
    await expect(heldPicture.onRead.mock.calls[0]?.[0]).toBe(PATH);
    await expect(host.getBoundingClientRect().height).toBe(waiting);

    heldPicture.release();
    const picture = await canvas.findByRole("button", {
      name: NAME,
    });
    await waitFor(() =>
      expect((picture.querySelector("img") as HTMLImageElement).naturalWidth).toBe(640),
    );
    await expect(host.getBoundingClientRect().height).toBe(waiting);
  },
};

/** A press on the picture opens the viewer. */
export const PictureOpensTheViewer: Story = {
  args: {
    onRead: async () => ({
      kind: "picture",
      url: PICTURE,
    }),
  },
  play: async ({ canvas }) => {
    comeNear();
    await userEvent.click(
      await canvas.findByRole("button", {
        name: NAME,
      }),
    );
    await screen.findByRole("dialog", {
      name: NAME,
    });
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  },
};

/** A refusal draws its own words, and a picture has nowhere else to open. */
export const RefusedDrawsItsLine: Story = {
  args: {
    onRead: async () => ({
      kind: "refused",
      says: "This picture is larger than 8 MB.",
    }),
  },
  play: async ({ canvas }) => {
    comeNear();
    await canvas.findByText("This picture is larger than 8 MB.");
    await expect(
      canvas.queryByRole("button", {
        name: "Open video",
      }),
    ).toBeNull();
  },
};

/** A read that throws says so, in place of a frame that waits for ever. */
export const AReadThatFailsDrawsALine: Story = {
  args: {
    onRead: async () => {
      throw new Error("gone");
    },
  },
  play: async ({ canvas }) => {
    comeNear();
    await canvas.findByText("This file could not be read.");
  },
};

const openRefused = fn();

/** A video refused outright still opens outside the thread, where the host can. */
export const RefusedVideoOpens: Story = {
  args: {
    path: "/work/shown/run.mp4",
    name: "run.mp4",
    onRead: async () => ({
      kind: "refused",
      says: "This video cannot play here.",
      open: openRefused,
    }),
  },
  play: async ({ canvas }) => {
    comeNear();
    await userEvent.click(
      await canvas.findByRole("button", {
        name: "Open video",
      }),
    );
    await expect(openRefused).toHaveBeenCalledTimes(1);
  },
};

const playerOf = (canvas: HTMLElement) =>
  within(canvas)
    .getByRole("group", {
      name: "run.mp4",
    })
    .querySelector("video") as HTMLVideoElement;

const loaded = (video: HTMLVideoElement) =>
  waitFor(() => expect(video.readyState).toBeGreaterThanOrEqual(HTMLMediaElement.HAVE_METADATA));

type VideoStory = StoryObj<typeof ShownVideo>;

const again = fn(async () => VIDEO);

/** **An address can lapse while a video sits paused**: an error asks `again` once and carries on from where it stopped. */
export const ErrorAsksAgainAndKeepsItsPlace: VideoStory = {
  render: () => (
    <ShownVideo
      name="run.mp4"
      media={{
        kind: "video",
        url: VIDEO,
        again,
      }}
    />
  ),
  play: async ({ canvasElement }) => {
    const video = playerOf(canvasElement);
    await loaded(video);
    video.currentTime = 1.25;
    await waitFor(() => expect(video.currentTime).toBeCloseTo(1.25, 1));

    const reloaded = fn();
    video.addEventListener("emptied", reloaded);
    fireEvent.error(video);
    await waitFor(() => expect(again).toHaveBeenCalledTimes(1));
    // The address came back unchanged, which is no change to React: the element has to load it again regardless.
    await waitFor(() => expect(reloaded).toHaveBeenCalled());
    await waitFor(() => expect(video.currentTime).toBeCloseTo(1.25, 1));
    await expect(video.error).toBeNull();
    await expect(
      within(canvasElement).queryByText("This video cannot play here."),
    ).toBeNull();
  },
};

const againOnce = fn(async () => VIDEO);
const openFailed = fn();

/** A second failure stops asking and draws the line, with the way out when the host has one. */
export const SecondFailureDrawsTheLine: VideoStory = {
  render: () => (
    <ShownVideo
      name="run.mp4"
      media={{
        kind: "video",
        url: VIDEO,
        again: againOnce,
        open: openFailed,
      }}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const video = playerOf(canvasElement);
    await loaded(video);
    fireEvent.error(video);
    await waitFor(() => expect(againOnce).toHaveBeenCalledTimes(1));
    fireEvent.error(playerOf(canvasElement));
    await canvas.findByText("This video cannot play here.");
    await expect(againOnce).toHaveBeenCalledTimes(1);
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Open video",
      }),
    );
    await expect(openFailed).toHaveBeenCalledTimes(1);
  },
};

/** Asked for a fresh address and refused, it draws the line at once. */
export const AgainRefusedDrawsTheLine: VideoStory = {
  render: () => (
    <ShownVideo
      name="run.mp4"
      media={{
        kind: "video",
        url: VIDEO,
        again: async () => {
          throw new Error("refused");
        },
      }}
    />
  ),
  play: async ({ canvasElement }) => {
    fireEvent.error(playerOf(canvasElement));
    await within(canvasElement).findByText("This video cannot play here.");
  },
};

/** With no `again` the first failure is the line, and with no `open` there is nothing to press. */
export const NoAgainNoOpen: VideoStory = {
  render: () => (
    <ShownVideo
      name="run.mp4"
      media={{
        kind: "video",
        url: VIDEO,
      }}
    />
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      canvas.queryByRole("button", {
        name: "Open video",
      }),
    ).toBeNull();
    fireEvent.error(playerOf(canvasElement));
    await canvas.findByText("This video cannot play here.");
    await expect(
      canvas.queryByRole("button", {
        name: "Open video",
      }),
    ).toBeNull();
  },
};

const fileRow = (id: string, path: string, name: string) => ({
  id,
  at: "14:03:07",
  kind: "file" as const,
  path,
  name,
});
const readInThread = fn(
  async (): Promise<ShownMedia> => ({
    kind: "picture",
    url: PICTURE,
  }),
);
const openInThread = fn();

/** Only a picture or a video draws a frame: any other file, and a host that reads no media, keep the plain line. */
export const OtherRowsStayThePlainLine: StoryObj<typeof SessionThread> = {
  render: () => (
    <div style={{ display: "flex", height: "calc(var(--space-12) * 8)" }}>
      <SessionThread
        rows={[
          fileRow("a", "/work/plan.md", "plan.md"),
          fileRow("b", PATH, NAME),
        ]}
        onAnswer={fn()}
        onOpenSession={fn()}
        onOpenFile={openInThread}
        onReadMedia={readInThread}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Opened plan.md",
      }),
    );
    await expect(openInThread).toHaveBeenCalledWith("/work/plan.md");
    comeNear();
    await waitFor(() => expect(readInThread).toHaveBeenCalledTimes(1));
    await expect(readInThread.mock.calls[0]).toEqual([PATH]);
    await canvas.findByRole("button", {
      name: NAME,
    });
  },
};

export const NoReadStaysThePlainLine: StoryObj<typeof SessionThread> = {
  render: () => (
    <div style={{ display: "flex", height: "calc(var(--space-12) * 8)" }}>
      <SessionThread
        rows={[
          fileRow("b", PATH, NAME),
        ]}
        onAnswer={fn()}
        onOpenSession={fn()}
      />
    </div>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    canvas.getByRole("button", {
      name: `Opened ${NAME}`,
    });
    comeNear();
    await expect(
      canvas.queryByRole("button", {
        name: NAME,
      }),
    ).toBeNull();
  },
};
