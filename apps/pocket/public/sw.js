// Receives a push, shows it, and opens the Job when it is tapped.
// Payload: {job_id, title, reason, step: {at, of}}.

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("push", (event) => {
  const push = event.data ? event.data.json() : {};
  const body = [push.reason, push.step ? `${push.step.at}/${push.step.of}` : undefined].filter(Boolean).join("  ");
  event.waitUntil(
    self.registration.showNotification(push.title ?? "Armada", {
      body,
      tag: push.job_id,
      icon: "/icon-256.png",
      data: { job_id: push.job_id },
    }),
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const path = event.notification.data?.job_id ? `/jobs/${event.notification.data.job_id}` : "/";
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
