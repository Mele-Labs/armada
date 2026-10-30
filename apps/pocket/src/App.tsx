// The phone app: one route at a time, over every paired Mac. A phone with no Mac paired goes to Pair.

import { useEffect } from "react";

import { Dispatch } from "./Dispatch";
import { Draft } from "./Draft";
import { start, useFleet } from "./fleet";
import { Home } from "./Home";
import { Job } from "./Job";
import { Pair } from "./Pair";
import { subscribe } from "./push";
import { go, routeOf, usePath, type Route } from "./router";
import { FileView, SketchView } from "./Shown";

let subscribed = false;

function Screen({ route }: { route: Route }) {
  switch (route.screen) {
    case "home":
      return <Home />;
    case "pair":
      return <Pair code={route.code} {...(route.origin === undefined ? {} : { origin: route.origin })} />;
    case "dispatch":
      return <Dispatch />;
    case "job":
      return <Job key={`${route.origin ?? "_"} ${route.id}`} origin={route.origin} id={route.id} />;
    case "draft":
      return <Draft key={`${route.origin} ${route.id}`} origin={route.origin} id={route.id} />;
    case "file":
      return <FileView origin={route.origin} id={route.id} path={route.path} />;
    case "sketch":
      return <SketchView origin={route.origin} id={route.id} sketch={route.sketch} />;
  }
}

export function App({ path }: { path: string }) {
  const fleet = useFleet();
  const route = routeOf(path);
  useEffect(() => void start(), []);
  useEffect(() => {
    if (!fleet.loaded) return;
    if (fleet.macs.length === 0 && route.screen !== "pair") go("/pair", true);
    else if (fleet.macs.length > 0 && !subscribed) {
      subscribed = true;
      void subscribe(fleet.macs.map((mac) => mac.machine)).catch(() => undefined);
    }
  }, [fleet.loaded, fleet.macs.length, route.screen]);
  return (
    <main className="pk-app" aria-label="Armada">
      <Screen route={route} />
    </main>
  );
}

export function Root() {
  return <App path={usePath()} />;
}
