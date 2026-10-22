// Receives a push, shows it, and opens the Job it is about when it is tapped.
// Payload: {session_id, title, reason, origin}. `origin` is the Mac that sent it; where it is absent
// the app finds the Mac that holds the Job (`/jobs/_/<id>`).

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("push", (event) => {
  const push = event.data ? event.data.json() : {};
  event.waitUntil(
    self.registration.showNotification(push.title ?? "Armada", {
      body: push.reason ?? "",
      tag: push.session_id,
      icon: "/icon-256.png",
      data: push,
    }),
  );
});

/** The app's path for the Job a push is about. A push with no Job opens the app as it is. */
function pathOf(push) {
  if (typeof push !== "object" || push === null || typeof push.session_id !== "string" || push.session_id === "") return "/";
  const mac = typeof push.origin === "string" && push.origin !== "" ? encodeURIComponent(push.origin) : "_";
  return `/jobs/${mac}/${encodeURIComponent(push.session_id)}`;
}

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const path = pathOf(event.notification.data);
  event.waitUntil(
    self.clients.matchAll({ type: "window", includeUncontrolled: true }).then(async (windows) => {
      for (const window of windows) {
        if ("navigate" in window) {
          await window.navigate(path);
          return window.focus();
        }
      }
      return self.clients.openWindow(path);
    }),
  );
});
