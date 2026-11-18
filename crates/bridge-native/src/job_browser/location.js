// Runs at document start in the page's own world, in every frame (`site::scripts`). WebKit on macOS has no per-site geolocation
// delegate, so the page's own `navigator.geolocation` is gated here: each call asks Bridge, through the handler it registers
// (`armadaSite`), before it reaches the real one, and is refused with PERMISSION_DENIED on a Block or a dismissal.
// The page's host is read by Bridge from the frame the message came from, never from what is said here.
(() => {
  const handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.armadaSite;
  if (!handler || typeof Geolocation === "undefined") return;
  const proto = Geolocation.prototype;
  const real = { get: proto.getCurrentPosition, watch: proto.watchPosition, clear: proto.clearWatch };
  const refusal = () => ({ code: 1, message: "User denied Geolocation", PERMISSION_DENIED: 1, POSITION_UNAVAILABLE: 2, TIMEOUT: 3 });
  const given = () => handler.postMessage({ kind: "location" }).then((answer) => answer === true, () => false);
  const watches = new Map();
  let last = 0;
  Object.defineProperties(proto, {
    getCurrentPosition: {
      configurable: true,
      writable: true,
      value: function getCurrentPosition(ok, fail, options) {
        given().then((yes) => {
          if (yes) real.get.call(this, ok, fail, options);
          else if (typeof fail === "function") fail(refusal());
        });
      },
    },
    watchPosition: {
      configurable: true,
      writable: true,
      value: function watchPosition(ok, fail, options) {
        const id = ++last;
        watches.set(id, null);
        given().then((yes) => {
          if (!watches.has(id)) return;
          if (yes) watches.set(id, real.watch.call(this, ok, fail, options));
          else {
            watches.delete(id);
            if (typeof fail === "function") fail(refusal());
          }
        });
        return id;
      },
    },
    clearWatch: {
      configurable: true,
      writable: true,
      value: function clearWatch(id) {
        const own = watches.get(id);
        watches.delete(id);
        if (own !== null && own !== undefined) real.clear.call(this, own);
      },
    },
  });
})();
