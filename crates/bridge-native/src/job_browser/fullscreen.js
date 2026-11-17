// Runs at document start in the page's own world, in every frame (`site::scripts`). Fullscreen is on for a Job browser's pages and
// refused here for a site the owner blocked it for. The choice is asked of Bridge when the page starts and when it takes the focus
// again, and kept, so a press that asks for fullscreen is answered in the same turn and keeps its user gesture. A change made in
// Site settings reaches a page the next time it takes the focus.
(() => {
  const handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.armadaSite;
  if (!handler || typeof Element === "undefined") return;
  let blocked = false;
  const ask = () => handler.postMessage({ kind: "fullscreen" }).then((answer) => { blocked = answer === false; }, () => {});
  ask();
  window.addEventListener("focus", ask);
  const refuse = () => Promise.reject(new TypeError("Fullscreen is blocked for this site"));
  for (const name of ["requestFullscreen", "webkitRequestFullscreen", "webkitRequestFullScreen"]) {
    const real = Element.prototype[name];
    if (typeof real !== "function") continue;
    Object.defineProperty(Element.prototype, name, {
      configurable: true,
      writable: true,
      value: function (...args) {
        return blocked ? refuse() : real.apply(this, args);
      },
    });
  }
})();
