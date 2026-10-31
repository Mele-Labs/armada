// The annotation layer's save path for a renderer in a browser (#1226, #1223):
// there is no main there, so the vite dev server writes the same files.
// Add `annotationsServer()` to the dev server's `plugins`; it is inert in a build.
//
//   GET    /__armada/annotations        every note, oldest first
//   PUT    /__armada/annotations/<id>   one note, as JSON; the id must match
//   DELETE /__armada/annotations/<id>          the note and its picture
//   GET    /__armada/annotations/<id>.png      its picture, or 404
//   PUT    /__armada/annotations/<id>.png      its picture, as PNG bytes
//   GET    /__armada/annotations/<id>/examples/<name>   one of its example pictures, or 404
//   PUT    /__armada/annotations/<id>/examples/<name>   one, as PNG bytes
//   DELETE /__armada/annotations/<id>/examples/<name>   one
//
// Under `src/main` because it is Node, and the renderer's tsconfig has none.

import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";

import { ANNOTATIONS_PATH, isAnnotation, isAnnotationId, isExampleOf, isPng } from "../shared/annotations";
import { annotationsDir, listAnnotations, readExample, readShot, removeAnnotation, removeExample, repositoryRoot, saveAnnotation, saveExample, saveShot } from "./annotations";

export type AnnotationsServerOptions = {
  /** Where to look for the repository from. Defaults to the dev server's root. */
  from?: string;
};

export function annotationsServer(options: AnnotationsServerOptions = {}): Plugin {
  return {
    name: "armada-annotations",
    apply: "serve",
    configureServer(server) {
      const root = repositoryRoot(options.from ?? server.config.root);
      if (root === null) {
        server.config.logger.warn(`armada-annotations: no repository above ${server.config.root}`);
        return;
      }
      const dir = annotationsDir(root);
      server.middlewares.use(ANNOTATIONS_PATH, (req, res) => {
        void answer(dir, req, res);
      });
    },
  };
}

/** One request, with the mount path already stripped from `req.url`. */
export async function answer(dir: string, req: IncomingMessage, res: ServerResponse): Promise<void> {
  const id = decodeURIComponent((req.url ?? "/").split("?")[0]!.replace(/^\/+|\/+$/g, ""));
  try {
    if (req.method === "GET" && id === "") {
      return send(res, 200, await listAnnotations(dir));
    }
    const example = /^([^/]+)\/examples\/([^/]+)$/.exec(id);
    if (example !== null && isExampleOf(example[1], example[2])) return await answerExample(dir, example[1]!, example[2]!, req, res);
    const shotOf = id.endsWith(".png") ? id.slice(0, -".png".length) : null;
    if (shotOf !== null && isAnnotationId(shotOf)) return await answerShot(dir, shotOf, req, res);
    if (!isAnnotationId(id)) return send(res, 404, { error: "no such route" });
    if (req.method === "PUT") {
      const body: unknown = JSON.parse((await read(req)).toString("utf8"));
      if (!isAnnotation(body) || body.id !== id) return send(res, 400, { error: "not an annotation" });
      await saveAnnotation(dir, body);
      return send(res, 204);
    }
    if (req.method === "DELETE") {
      await removeAnnotation(dir, id);
      return send(res, 204);
    }
    return send(res, 405, { error: "method not allowed" });
  } catch (error) {
    return send(res, 500, { error: error instanceof Error ? error.message : String(error) });
  }
}

async function answerShot(dir: string, id: string, req: IncomingMessage, res: ServerResponse): Promise<void> {
  if (req.method === "GET") {
    const png = await readShot(dir, id);
    if (png === null) return send(res, 404, { error: "no picture" });
    res.statusCode = 200;
    res.setHeader("Content-Type", "image/png");
    res.end(Buffer.from(png));
    return;
  }
  if (req.method === "PUT") {
    const body = await read(req);
    if (!isPng(body)) return send(res, 400, { error: "not a PNG" });
    await saveShot(dir, id, body);
    return send(res, 204);
  }
  return send(res, 405, { error: "method not allowed" });
}

async function answerExample(dir: string, id: string, name: string, req: IncomingMessage, res: ServerResponse): Promise<void> {
  if (req.method === "GET") {
    const png = await readExample(dir, id, name);
    if (png === null) return send(res, 404, { error: "no picture" });
    res.statusCode = 200;
    res.setHeader("Content-Type", "image/png");
    res.end(Buffer.from(png));
    return;
  }
  if (req.method === "PUT") {
    const body = await read(req);
    if (!isPng(body)) return send(res, 400, { error: "not a PNG" });
    await saveExample(dir, id, name, body);
    return send(res, 204);
  }
  if (req.method === "DELETE") {
    await removeExample(dir, id, name);
    return send(res, 204);
  }
  return send(res, 405, { error: "method not allowed" });
}

function read(req: IncomingMessage): Promise<Buffer> {
  return new Promise((done, fail) => {
    const chunks: Buffer[] = [];
    req.on("data", (chunk: Buffer) => chunks.push(chunk));
    req.on("end", () => done(Buffer.concat(chunks)));
    req.on("error", fail);
  });
}

function send(res: ServerResponse, status: number, body?: unknown): void {
  res.statusCode = status;
  if (body === undefined) {
    res.end();
    return;
  }
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(body));
}
