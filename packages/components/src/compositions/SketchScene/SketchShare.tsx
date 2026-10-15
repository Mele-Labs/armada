import { createContext, useContext } from "react";

import { MIME, type SketchShareFile, type SketchShareHost, type SketchShareSaved } from "./share";

/**
 * What the app around a sketch lends it for sharing. Provided once by the shell that owns the file
 * dialog; every `SketchScene` with a rail reads it, so a new surface drawing one gets Share for nothing.
 */
export const SketchShareContext = createContext<SketchShareHost>({});

export const useSketchShareHost = (): SketchShareHost => useContext(SketchShareContext);

/** The host's save, or a download where the host has none: the first of `files`, since a browser asks for no place. */
export async function saveWith(host: SketchShareHost, files: readonly SketchShareFile[]): Promise<SketchShareSaved> {
  if (host.save !== undefined) return host.save(files);
  const file = files[0];
  if (file === undefined) return { saved: false, why: "failed" };
  const url = URL.createObjectURL(new Blob([file.data as BlobPart], { type: MIME[file.kind] }));
  const link = document.createElement("a");
  link.href = url;
  link.download = file.name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
  return { saved: true, name: file.name };
}

/** The host's clipboard, or the browser's. */
export const copyWith = (host: SketchShareHost, text: string): Promise<void> => (host.copy === undefined ? navigator.clipboard.writeText(text) : host.copy(text));

/** An SVG drawn onto a canvas at `scale` times its own size, as PNG bytes. The size is the SVG's own width and height. */
export async function svgToPng(svg: string, width: number, height: number, scale = 2): Promise<Uint8Array> {
  const url = URL.createObjectURL(new Blob([svg], { type: MIME.svg }));
  try {
    const image = new Image();
    await new Promise<void>((done, fail) => {
      image.onload = () => done();
      image.onerror = () => fail(new Error("The sketch did not draw as an image."));
      image.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = Math.round(width * scale);
    canvas.height = Math.round(height * scale);
    const context = canvas.getContext("2d");
    if (context === null) throw new Error("No canvas to draw on.");
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob | null>((done) => canvas.toBlob(done, MIME.png));
    if (blob === null) throw new Error("The canvas gave no image.");
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    URL.revokeObjectURL(url);
  }
}
