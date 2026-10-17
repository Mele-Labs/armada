// Receives a push, shows it, and opens the app when it is tapped.
// Payload: {session_id, title, reason}.

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("push", (event) => {
  const push = event.data ? event.data.json() : {};
  event.waitUntil(
    self.registration.showNotification(push.title ?? "Armada", {
      body: push.reason ?? "",
      tag: push.session_id,
      icon: "/icon-256.png",
    }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const path = "/";
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
