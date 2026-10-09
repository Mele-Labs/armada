import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./styles.css";
import "./pocket.css";
import { App } from "./Screens";
import { usePath } from "./router";

function Routed() {
  return <App path={usePath()} />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Routed />
  </StrictMode>,
);
