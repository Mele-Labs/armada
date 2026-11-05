// The address a video a Session showed plays from: main answers it (`main/session-video.ts`) and the renderer builds it, so no
// preload member carries it. The session and the path travel as a query, since a standard scheme lowercases its host.

export const VIDEO_SCHEME = "armada-video";

export const videoAddressOf = (sessionId: string, path: string): string =>
  `${VIDEO_SCHEME}://shown/?session=${encodeURIComponent(sessionId)}&path=${encodeURIComponent(path)}`;
