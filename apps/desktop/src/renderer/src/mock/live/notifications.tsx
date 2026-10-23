// The page the `now-and-plain-plans` Session builds, drawn in its live view: an ordinary settings page for email notifications, as it
// grows. A heading and one switch, then all three, then each with what it does and a Save. Example work, unlike anything in Bridge.

import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Button, Switch } from "@armada/components";

import "../../styles/index.css";
import "@armada/components/src/look";
import "./live.css";
import { onStage, stageNow } from "./stage";

const ROWS = [
  { name: "Comments", says: "Email me when someone comments on my posts" },
  { name: "Mentions", says: "Email me when someone mentions me" },
  { name: "Weekly digest", says: "Email me a summary every Monday" },
];

function NotificationsPage() {
  const [stage, setStage] = useState(stageNow);
  useEffect(() => onStage(setStage), []);
  const rows = stage === 0 ? ROWS.slice(0, 1) : ROWS;
  return (
    <main className="page">
      <h1 className="page__title">Email notifications</h1>
      <div className="page__rows">
        {rows.map((row) => (
          <Switch key={row.name} defaultChecked={row.name !== "Weekly digest"} {...(stage >= 2 ? { description: row.says } : {})}>
            {row.name}
          </Switch>
        ))}
      </div>
      {stage >= 2 ? <Button variant="primary">Save</Button> : null}
    </main>
  );
}

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <NotificationsPage />
    </StrictMode>,
  );
}
