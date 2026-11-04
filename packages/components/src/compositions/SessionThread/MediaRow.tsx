import { useEffect, useRef, useState } from "react";

import { Enlargeable } from "../../primitives/Enlargeable/Enlargeable";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";

/** What the host read for a picture or a video a Job showed. `again` asks for a fresh address, where the first one can lapse. */
export type ShownMedia =
  | {
      kind: "picture";
      url: string;
    }
  | {
      kind: "video";
      url: string;
      again?: () => Promise<string>;
    }
  | {
      kind: "refused";
      says: string;
    };

const PICTURE = /\.(png|jpe?g|gif|webp|heic|svg)$/i;
const VIDEO = /\.(mp4|mov|webm)$/i;

/** Whether a path a Job showed draws as media, by its extension. */
export const mediaKindOf = (path: string): "picture" | "video" | undefined =>
  PICTURE.test(path) ? "picture" : VIDEO.test(path) ? "video" : undefined;

/** The nearest box that scrolls: the observer's margin only reaches past its own root, and this is what clips the row. */
function scrollerOf(element: Element): Element | null {
  for (let at = element.parentElement; at !== null; at = at.parentElement) {
    const { overflowY } = getComputedStyle(at);
    if (overflowY === "auto" || overflowY === "scroll") return at;
  }
  return null;
}

const urlOf = (media: ShownMedia | undefined) => (media?.kind === "picture" ? media.url : undefined);

function MediaLine({ says }: { says: string }) {
  return <p className="armada-session-media__line">{says}</p>;
}

/** A picture in its frame, sized from what it loads as: the frame's height never moves, only the width follows the picture. */
function FramedPicture({ url, name }: { url: string; name: string }) {
  const [natural, setNatural] = useState<
    | {
        width: number;
        height: number;
      }
    | undefined
  >(undefined);
  const [broken, setBroken] = useState(false);
  if (broken) return <MediaLine says="This picture cannot be drawn here." />;
  return (
    <>
      {natural === undefined ? <Skeleton className="armada-session-media__wait" /> : null}
      <div
        className={
          natural === undefined
            ? "armada-session-media__fit armada-session-media__fit--loading"
            : "armada-session-media__fit"
        }
        style={
          natural === undefined
            ? undefined
            : {
                width: `min(${natural.width}px, 100%, calc(var(--session-media-height) * ${natural.width / natural.height}))`,
                aspectRatio: `${natural.width} / ${natural.height}`,
              }
        }
      >
        <Enlargeable
          fills
          className="armada-session-media__picture"
          src={url}
          name={name}
          onLoad={(event) =>
            setNatural({
              width: event.currentTarget.naturalWidth || 1,
              height: event.currentTarget.naturalHeight || 1,
            })
          }
          onError={() => setBroken(true)}
        />
      </div>
    </>
  );
}

/**
 * A video with its native controls. **`#t=0.001` is what makes iOS draw a first frame with no press.** A signed address
 * can lapse while it sits paused, so an error asks `again` once and carries on from where it stopped.
 */
export function ShownVideo({
  media,
  name,
}: {
  media: Extract<
    ShownMedia,
    {
      kind: "video";
    }
  >;
  name: string;
}) {
  const [url, setUrl] = useState(media.url);
  const [failed, setFailed] = useState(false);
  const asked = useRef(false);
  const resume = useRef<number | undefined>(undefined);
  const player = useRef<HTMLVideoElement>(null);
  if (failed) return <MediaLine says="This video cannot play here." />;
  return (
    <div className="armada-session-media__video" role="group" aria-label={name}>
      <video
        ref={player}
        controls
        playsInline
        preload="metadata"
        src={`${url}#t=0.001`}
        onLoadedMetadata={() => {
          if (resume.current === undefined || player.current === null) return;
          player.current.currentTime = resume.current;
          resume.current = undefined;
        }}
        onError={() => {
          if (media.again === undefined || asked.current) {
            setFailed(true);
            return;
          }
          asked.current = true;
          resume.current = player.current?.currentTime;
          void media.again().then(
            (next) => {
              // On the element as well: an address that comes back unchanged is no change to React, and the element would stay failed.
              setUrl(next);
              if (player.current !== null) player.current.src = `${next}#t=0.001`;
            },
            () => setFailed(true),
          );
        }}
      />
    </div>
  );
}

/**
 * What a Job showed, drawn in a frame of fixed height so the thread never moves as it arrives. Read once, when the row
 * nears the screen.
 */
export function MediaFrame({
  path,
  name,
  onRead,
}: {
  path: string;
  name: string;
  onRead: (path: string) => Promise<ShownMedia>;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const [near, setNear] = useState(false);
  const [media, setMedia] = useState<ShownMedia | undefined>(undefined);
  // The host passes a new closure each render; only the path is a reason to read again.
  const read = useRef(onRead);
  read.current = onRead;

  useEffect(() => {
    const held = frame.current;
    if (held === null) return;
    const watching = new IntersectionObserver(
      (seen) => {
        if (!seen.some((one) => one.isIntersecting)) return;
        setNear(true);
        watching.disconnect();
      },
      {
        root: scrollerOf(held),
        rootMargin: "100% 0px",
      },
    );
    watching.observe(held);
    return () => watching.disconnect();
  }, []);

  useEffect(() => {
    if (!near) return;
    let gone = false;
    let made: string | undefined;
    void read
      .current(path)
      .catch(
        (): ShownMedia => ({
          kind: "refused",
          says: "This file could not be read.",
        }),
      )
      .then((got) => {
        const url = urlOf(got);
        if (gone) {
          if (url?.startsWith("blob:")) URL.revokeObjectURL(url);
          return;
        }
        made = url?.startsWith("blob:") ? url : undefined;
        setMedia(got);
      });
    return () => {
      gone = true;
      if (made !== undefined) URL.revokeObjectURL(made);
    };
  }, [
    near,
    path,
  ]);

  return (
    <div ref={frame} className="armada-session-media">
      {media === undefined ? <Skeleton className="armada-session-media__wait" /> : null}
      {media?.kind === "refused" ? <MediaLine says={media.says} /> : null}
      {media?.kind === "picture" ? <FramedPicture url={media.url} name={name} /> : null}
      {media?.kind === "video" ? <ShownVideo media={media} name={name} /> : null}
    </div>
  );
}
