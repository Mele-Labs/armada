// The map drawn on a canvas: it takes the camera the page already keeps and the scene the battlefield has
// already read, and hands both to the stage, which draws when they change. It holds no state of its own. The pictures a person can press
// are `Hits.tsx`'s: `children` is the layer they go in, over the canvas and under the controls, and `onStage` gives them the stage to
// place themselves by.

import { useLayoutEffect, useRef, type MouseEvent, type ReactNode } from "react";

import type { Cam } from "../camera";
import type { Size } from "../layout";
import type { PixiSceneIn } from "./scene";
import { Stage } from "./stage";
import "./pixi.css";

export type PixiMapProps = {
  size: Size;
  /** Where the camera is going, and whether a hand is on it (it follows at once) or not (it eases). */
  cam: Cam;
  live: boolean;
  scene: PixiSceneIn;
  /** A press on the pane. It is told where it landed, since the pictures over the canvas are inside the pane. */
  onGround?: (event: MouseEvent) => void;
  onStage?: (stage: Stage | undefined) => void;
  /** The pane is folded away: the stage draws nothing until it is shown. */
  hidden?: boolean;
  children?: ReactNode;
};

export function PixiMap({ size, cam, live, scene, onGround, onStage, hidden = false, children }: PixiMapProps) {
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<Stage | undefined>(undefined);
  useLayoutEffect(() => {
    const made = Stage.start(host.current!);
    stage.current = made;
    onStage?.(made);
    return () => {
      onStage?.(undefined);
      made.destroy();
      stage.current = undefined;
    };
  }, []);
  useLayoutEffect(() => stage.current?.setHidden(hidden), [hidden]);
  // In this order: the size is the camera's, and the camera is the scene's.
  useLayoutEffect(() => stage.current?.setSize(size), [size.width, size.height]);
  useLayoutEffect(() => stage.current?.setCamera(cam, live), [cam.x, cam.y, cam.eye, cam.tilt, live]);
  useLayoutEffect(() => stage.current?.setScene(scene), [scene]);
  return (
    <div ref={host} className="bf-pixi" onClick={onGround}>
      {children}
    </div>
  );
}
