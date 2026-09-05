(function () {
  if (location.origin !== "http://vapurr.localhost") return;
  var id = crypto.randomUUID();
  Object.defineProperty(window, "__vapurrDocument", { value: id });
  var originalFetch = window.fetch.bind(window);
  window.fetch = function (input, options) {
    var req = new Request(input, options);
    if (new URL(req.url).origin === location.origin && new URL(req.url).pathname.includes("/api")) {
      var headers = new Headers(req.headers);
      headers.set("X-Vapurr-Client", __API_TOKEN__);
      headers.set("X-Vapurr-Document", id);
      req = new Request(req, { headers: headers, redirect: "error" });
    }
    return originalFetch(req);
  };
})();
