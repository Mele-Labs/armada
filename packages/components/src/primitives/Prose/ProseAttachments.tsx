import { createContext, useContext, useEffect, useState } from "react";
import { Play } from "lucide-react";

import { Enlargeable } from "../Enlargeable/Enlargeable";

/** What a forge attachment came back as. */
export type AttachmentBytes = { bytes: Uint8Array; mime: string };

/**
 * Fetches an attachment's bytes, which the renderer cannot (its CSP reaches nowhere). Provided once at the root
 * of `App`; without one a `Prose` draws attachments as the links they were.
 */
export const ProseAttachments = createContext<((address: string) => Promise<AttachmentBytes | null>) | null>(null);

const HOSTS = ["github.com", "githubusercontent.com"];

/** A forge-hosted address; main enforces the same list, this only decides what is worth asking for. */
export function isForgeAddress(address: string): boolean {
  try {
    const url = new URL(address);
    return url.protocol === "https:" && HOSTS.some((host) => url.hostname === host || url.hostname.endsWith(`.${host}`));
  } catch {
    return false;
  }
}

/** A bare attachment: a `user-attachments` asset, or a forge file that is a video by its extension. */
export function isVideoAddress(address: string): boolean {
  if (!isForgeAddress(address)) return false;
  const { pathname } = new URL(address);
  return pathname.startsWith("/user-attachments/assets/") || /\.(mp4|webm|mov)$/i.test(pathname);
}

type Loaded = { state: "idle" } | { state: "loading" } | { state: "failed" } | { state: "ready"; url: string; mime: string };

/** The attachment as a `blob:` URL once `fetching` is true, revoked when this goes. */
function useAttachment(address: string, fetching: boolean): Loaded {
  const fetch = useContext(ProseAttachments);
  const [loaded, setLoaded] = useState<Loaded>({ state: "idle" });
  useEffect(() => {
    if (!fetching || fetch === null) return;
    let made: string | null = null;
    let live = true;
    setLoaded({ state: "loading" });
    fetch(address).then(
      (got) => {
        if (!live) return;
        if (got === null) return setLoaded({ state: "failed" });
        made = URL.createObjectURL(new Blob([got.bytes as BlobPart], { type: got.mime }));
        setLoaded({ state: "ready", url: made, mime: got.mime });
      },
      () => live && setLoaded({ state: "failed" }),
    );
    return () => {
      live = false;
      if (made !== null) URL.revokeObjectURL(made);
    };
  }, [address, fetching, fetch]);
  return loaded;
}

/** A video: a poster with a play button, fetched on the press, then the native controls. */
export function AttachedVideo({ address }: { address: string }) {
  const [asked, setAsked] = useState(false);
  const loaded = useAttachment(address, asked);
  if (loaded.state === "ready") {
    return (
      <span className="armada-prose__attachment">
        <video className="armada-prose__video" src={loaded.url} controls autoPlay />
      </span>
    );
  }
  return (
    <span className="armada-prose__attachment">
      <button type="button" className="armada-prose__poster" disabled={loaded.state === "loading"} onClick={() => setAsked(true)} aria-label="Play video">
        <Play size={24} strokeWidth={2} aria-hidden />
        <span className="armada-prose__poster-note">{loaded.state === "loading" ? "Loading" : loaded.state === "failed" ? "Could not load. Press to retry" : "Video"}</span>
      </button>
    </span>
  );
}

/** A picture: a thumbnail that opens larger. `fallback` is what draws where the fetch comes to nothing. */
export function AttachedImage({ address, name, fallback }: { address: string; name: string; fallback: React.ReactNode }) {
  const loaded = useAttachment(address, true);
  if (loaded.state === "failed") return <>{fallback}</>;
  if (loaded.state !== "ready") return <span className="armada-prose__thumb-wait" aria-hidden />;
  return <Enlargeable src={loaded.url} name={name} className="armada-prose__thumb" />;
}
