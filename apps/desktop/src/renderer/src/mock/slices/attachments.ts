// Attachments: a short clip drawn on a canvas and recorded in the page, or a small picture, so a walk plays
// something real. Where a page cannot record (a test's jsdom) the bytes are a placeholder and nothing plays.

import type { AttachmentsApi } from "../../../../shared/api/attachments";
import type { Slice } from "../fake-context";

const drawn = (width: number, height: number, paint: (context: CanvasRenderingContext2D, frame: number) => void) => {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  return context === null ? null : { canvas, context, paint: (frame: number) => paint(context, frame) };
};

const scene = (context: CanvasRenderingContext2D, frame: number) => {
  context.fillStyle = "#1f2430";
  context.fillRect(0, 0, 320, 180);
  context.fillStyle = "#6aa9ff";
  context.fillRect(20 + ((frame * 6) % 260), 70, 40, 40);
};

async function clip(): Promise<{ bytes: Uint8Array; mime: string } | null> {
  const surface = drawn(320, 180, scene);
  if (surface === null || typeof MediaRecorder === "undefined" || typeof surface.canvas.captureStream !== "function") return null;
  const recorder = new MediaRecorder(surface.canvas.captureStream(20), { mimeType: "video/webm" });
  const parts: Blob[] = [];
  recorder.ondataavailable = (event) => parts.push(event.data);
  const stopped = new Promise<void>((resolve) => (recorder.onstop = () => resolve()));
  recorder.start();
  for (let frame = 0; frame < 30; frame++) {
    surface.paint(frame);
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  recorder.stop();
  await stopped;
  return { bytes: new Uint8Array(await new Blob(parts).arrayBuffer()), mime: "video/webm" };
}

async function picture(): Promise<{ bytes: Uint8Array; mime: string } | null> {
  const surface = drawn(320, 180, scene);
  if (surface === null) return null;
  surface.paint(8);
  const blob = await new Promise<Blob | null>((resolve) => surface.canvas.toBlob(resolve, "image/png"));
  return blob === null ? null : { bytes: new Uint8Array(await blob.arrayBuffer()), mime: "image/png" };
}

export const attachments: Slice<AttachmentsApi, Record<string, never>> = {
  name: "attachments",
  state: {},
  api: () => ({
    fetchAttachment: async (url) => {
      try {
        const wanted = /\.(png|jpe?g|gif)$/i.test(url) || url.includes("/images/") ? await picture() : await clip();
        return wanted ?? { bytes: new Uint8Array([0]), mime: "application/octet-stream" };
      } catch {
        return null;
      }
    },
  }),
};
