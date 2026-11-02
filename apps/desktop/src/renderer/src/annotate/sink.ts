// Where the layer's notes are kept: main, over the dev-only preload entry, or a
// vite dev server when the renderer runs in a browser. Both write the same files.

import {
  ANNOTATIONS_PATH,
  STORIES_PATH,
  type Annotation,
  type AnnotationsDevApi,
  type StoryIndex,
} from "../../../shared/annotations";
import type { StoryModule } from "./story";

export type Sink = AnnotationsDevApi & {
  /** Named in the indicator, so a failed save says which path failed. */
  via: "main" | "dev server";
  /**
   * The index of the components' stories, `annotations-stories.ts`. Absent where nothing serves it,
   * which is Electron: the layer then draws no file line and no Variant tool.
   */
  stories?: () => Promise<StoryIndex>;
  /**
   * A story file's module, `file` being the index's path from the repository root, so the Variant tool can draw each story
   * over its name. Only the mock sets it: Electron and the dev server's sink cannot import a story, and draw the names alone.
   * Null where the file is unknown.
   */
  story?: (file: string) => Promise<StoryModule | null>;
};

export function mainSink(api: AnnotationsDevApi): Sink {
  return {
    via: "main",
    list: () => api.list(),
    save: (note) => api.save(note),
    remove: (id) => api.remove(id),
    root: () => api.root(),
    capture: (box) => api.capture(box),
    saveShot: (id, png) => api.saveShot(id, png),
    shot: (id) => api.shot(id),
    saveExample: (id, name, png) => api.saveExample(id, name, png),
    example: (id, name) => api.example(id, name),
    removeExample: (id, name) => api.removeExample(id, name),
  };
}

type Fetch = (url: string, init?: RequestInit) => Promise<Response>;

/**
 * The dev server's routes, `annotations-server.ts`. A `fetch` from the renderer
 * is refused everywhere else in Bridge; this runs only in a browser, where no main exists.
 */
export function devServerSink(fetcher: Fetch = (url, init) => fetch(url, init)): Sink {
  async function ok(response: Response): Promise<Response> {
    if (response.ok) return response;
    throw new Error(
      response.status === 404
        ? `${ANNOTATIONS_PATH} is not served — add annotationsServer() to the dev server's plugins`
        : `${ANNOTATIONS_PATH} answered ${response.status}`,
    );
  }
  const one = (id: string): string => `${ANNOTATIONS_PATH}/${encodeURIComponent(id)}`;
  const exampleAt = (id: string, name: string): string =>
    `${one(id)}/examples/${encodeURIComponent(name)}`;
  return {
    via: "dev server",
    list: async () => (await (await ok(await fetcher(ANNOTATIONS_PATH))).json()) as Annotation[],
    // A server without the plugin answers 404 or an HTML page; either is an empty index.
    stories: async () => {
      const response = await fetcher(STORIES_PATH);
      if (!response.ok) return {};
      return (await response.json().catch(() => ({}))) as StoryIndex;
    },
    save: async (note) => {
      await ok(
        await fetcher(one(note.id), {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(note),
        }),
      );
    },
    remove: async (id) => {
      await ok(
        await fetcher(one(id), {
          method: "DELETE",
        }),
      );
    },
    // A browser has no Fleet to send to and nothing to capture a window with.
    root: async () => null,
    capture: async () => null,
    // Never called by the layer here, since nothing captures; kept so the routes answer as main does.
    saveShot: async (id, png) => {
      await ok(
        await fetcher(one(`${id}.png`), {
          method: "PUT",
          headers: {
            "Content-Type": "image/png",
          },
          body: png,
        }),
      );
    },
    shot: async (id) => {
      const response = await fetcher(one(`${id}.png`));
      return response.status === 404 ? null : (await ok(response)).arrayBuffer();
    },
    saveExample: async (id, name, png) => {
      await ok(
        await fetcher(exampleAt(id, name), {
          method: "PUT",
          headers: {
            "Content-Type": "image/png",
          },
          body: png,
        }),
      );
    },
    example: async (id, name) => {
      const response = await fetcher(exampleAt(id, name));
      return response.status === 404 ? null : (await ok(response)).arrayBuffer();
    },
    removeExample: async (id, name) => {
      await ok(
        await fetcher(exampleAt(id, name), {
          method: "DELETE",
        }),
      );
    },
  };
}
