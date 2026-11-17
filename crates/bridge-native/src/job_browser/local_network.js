// Runs at document start in the page's own world, in every frame (`site::scripts`). WebKit gives Bridge no hook on each request, so a
// page's own fetch, XMLHttpRequest, WebSocket and EventSource are watched here for an address on this Mac or the person's network,
// and Bridge is told which host was reached (`armadaSite`). Bridge decides whether that is a request to ask about; the content rule
// list (`site::rules`) is what refuses the load. Nothing here changes what a request does.
(() => {
  const handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.armadaSite;
  if (!handler) return;
  const told = new Set();
  const reached = (address) => {
    let url;
    try {
      url = new URL(String(address), document.baseURI);
    } catch (_) {
      return;
    }
    if (!/^(https?|wss?):$/.test(url.protocol) || told.has(url.host)) return;
    // Only a name or number that is plainly local is told of: a public name is not looked up here.
    const name = url.hostname.toLowerCase();
    if (!/^\d+\.\d+\.\d+\.\d+$|^\[|^localhost$|\.localhost$|\.local$/.test(name)) return;
    told.add(url.host);
    // A page that keeps reaching one host is told of once in a while, so a dismissed ask is raised again by a later attempt.
    setTimeout(() => told.delete(url.host), 5000);
    handler.postMessage({ kind: "local_network", target: url.hostname });
  };
  const wrap = (owner, name, address) => {
    const original = owner[name];
    if (typeof original !== "function") return;
    owner[name] = function (...args) {
      try {
        reached(address(args));
      } catch (_) {}
      return Reflect.apply(original, this, args);
    };
  };
  wrap(window, "fetch", (args) => (args[0] && args[0].url) || args[0]);
  if (window.XMLHttpRequest) wrap(XMLHttpRequest.prototype, "open", (args) => args[1]);
  for (const name of ["WebSocket", "EventSource"]) {
    const Original = window[name];
    if (typeof Original !== "function") continue;
    const Wrapped = new Proxy(Original, {
      construct(target, args, newTarget) {
        try {
          reached(args[0]);
        } catch (_) {}
        return Reflect.construct(target, args, newTarget === Wrapped ? target : newTarget);
      },
    });
    window[name] = Wrapped;
  }
})();
