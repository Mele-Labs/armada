// Keeps the app: a page that arrives is kept, and answers when the network does not (a Home Screen app
// opened while the tailnet is still waking would otherwise be a blank page). Receives a push, shows it,
// and opens the Job it is about when it is tapped.
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

const KEPT = "armada-app";
// Every route serves the one page, so one key holds it.
const PAGE = "/index.html";
const WAIT_MS = 3000;

/** The built files a page names. */
const assetsOf = (html) => new Set(html.match(/\/assets\/[^"'\s)>]+/g) ?? []);

/** Keeps a page that arrived, and the assets it names; older builds' assets go. */
async function keep(page) {
  const cache = await caches.open(KEPT);
  const named = assetsOf(await page.clone().text());
  await cache.put(PAGE, page);
  for (const request of await cache.keys()) {
    const { pathname } = new URL(request.url);
    if (pathname.startsWith("/assets/") && !named.has(pathname)) await cache.delete(request);
  }
}

/** Network first; the kept page when the network fails, answers wrongly or takes over three seconds. */
async function page(event) {
  const fresh = fetch(event.request);
  event.waitUntil(fresh.then((response) => (response.ok ? keep(response.clone()) : undefined), () => undefined));
  let timer;
  const late = new Promise((_, reject) => (timer = setTimeout(reject, WAIT_MS)));
  try {
    const response = await Promise.race([fresh, late]);
    if (response.ok) return response;
  } catch {
    // Fall through to the kept page.
  } finally {
    clearTimeout(timer);
  }
  const cache = await caches.open(KEPT);
  return (await cache.match(PAGE)) ?? fresh;
}

/** Built files are named by their content, so a kept one is never stale. */
async function asset(request) {
  const cache = await caches.open(KEPT);
  const kept = await cache.match(request);
  if (kept) return kept;
  const response = await fetch(request);
  if (response.ok) await cache.put(request, response.clone());
  return response;
}

// Only the app's page and its built files. The API, the admin routes, a POST and this file go to the network untouched.
self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) return;
  if (request.mode === "navigate" && !/^\/(api\/|admin\/|sw\.js$)/.test(url.pathname)) event.respondWith(page(event));
  else if (url.pathname.startsWith("/assets/")) event.respondWith(asset(request));
});
