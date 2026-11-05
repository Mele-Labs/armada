// A Job that showed two pictures and two videos with `show_file`, each drawn in its row of the thread. The files are the phone
// fixture's, and each read is held back so the skeleton can be seen. One video is refused. The walk `thread-media` plays it.

import type {
  ArtifactRead,
  Session,
  SessionRow,
  VideoAddress,
} from "@armada/screens/src/draft/sessions";

import landscape from "../../../../../../pocket/src/fixture/media/landscape.jpg";
import portrait from "../../../../../../pocket/src/fixture/media/portrait.jpg";
import recording from "../../../../../../pocket/src/fixture/media/recording.mp4";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const JOB = "thread-media";
const ROOT = "/Users/user/armada/.playwright-mcp";
const HELD_MS = 800;

const PICTURES: Record<string, string> = {
  [`${ROOT}/cockpit-wide.jpg`]: landscape,
  [`${ROOT}/job-on-the-phone.jpg`]: portrait,
};
const VIDEO = `${ROOT}/cockpit-recording.mp4`;
const REFUSED = `${ROOT}/full-run.mov`;

const held = () => new Promise((done) => setTimeout(done, HELD_MS));

const file = (id: string, at: string, path: string): SessionRow => ({
  id,
  at,
  kind: "tool",
  text: `mcp__armada-fleet__show_file ${path}`,
});
const agent = (id: string, at: string, text: string): SessionRow => ({
  id,
  at,
  kind: "message",
  from: {
    kind: "agent",
  },
  text,
});

const job: Session = {
  id: JOB,
  title: "Draw shown pictures in the thread",
  turn: {
    state: "idle",
  },
  rows: [
    {
      id: "m-1",
      at: "14:02:10",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Show me what the Cockpit looks like now.",
    },
    agent("m-2", "14:06:00", "A short recording of it running, and the whole run."),
    file("m-3", "14:06:04", VIDEO),
    file("m-4", "14:06:09", REFUSED),
    agent("m-5", "14:07:00", "The Cockpit on a wide window, and a Job on the phone."),
    file("m-6", "14:07:04", `${ROOT}/cockpit-wide.jpg`),
    file("m-7", "14:07:09", `${ROOT}/job-on-the-phone.jpg`),
  ],
  attachments: [
    ...Object.keys(PICTURES),
    VIDEO,
    REFUSED,
  ].map((path) => ({
    kind: "artifact" as const,
    form: "file" as const,
    id: path,
    title: path.slice(path.lastIndexOf("/") + 1),
  })),
};

export const threadMedia: Scenario = {
  ...s200Sessions,
  name: "thread-media",
  says: "A Job whose thread draws the pictures and videos it showed: a press opens a picture full screen, where it zooms",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [
        job,
      ]);
      return {
        ...store,
        readArtifact: async (id: string, path: string): Promise<ArtifactRead> => {
          const url = PICTURES[path];
          if (url === undefined)
            return (
              store.readArtifact?.(id, path) ?? {
                ok: false,
                why: "unreadable",
              }
            );
          await held();
          return {
            ok: true,
            bytes: new Uint8Array(await (await fetch(url)).arrayBuffer()),
            type: "image/jpeg",
          };
        },
        videoAddress: async (_id: string, path: string): Promise<VideoAddress> => {
          await held();
          return path === VIDEO
            ? {
                ok: true,
                address: recording,
              }
            : {
                // What the row draws once a video will not play. An address that really fails is an error the walk's engine counts.
                ok: false,
                says: "This video cannot play here.",
              };
        },
        openFile: () => undefined,
      };
    },
  },
};
