(function installDocumentHandoff(global) {
  "use strict";

  const PROTOCOL = "unippt-document-handoff-v1";
  const HASH_KEY = "unippt-handoff";
  const DEFAULT_TIMEOUT_MS = 60_000;
  const TOKEN_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

  function receiverToken(location = global.location) {
    const hash = String(location?.hash || "").replace(/^#/, "");
    const token = new URLSearchParams(hash).get(HASH_KEY) || "";
    return TOKEN_PATTERN.test(token) ? token : "";
  }

  function channelName(token) {
    return `${PROTOCOL}:${token}`;
  }

  function message(type, token, payload = {}) {
    return { protocol: PROTOCOL, type, token, ...payload };
  }

  function receiverUrl(token) {
    const url = new URL(global.location.href);
    url.hash = `${HASH_KEY}=${encodeURIComponent(token)}`;
    return url.href;
  }

  function open(options = {}) {
    if (typeof global.BroadcastChannel !== "function") throw new Error("浏览器不支持安全的新文稿窗口传输");
    const token = global.crypto?.randomUUID?.();
    if (!TOKEN_PATTERN.test(String(token || ""))) throw new Error("无法创建安全的新文稿窗口令牌");
    const channel = new global.BroadcastChannel(channelName(token));
    const child = global.open(receiverUrl(token), "_blank");
    if (!child) {
      channel.close();
      throw new Error("浏览器阻止了新窗口，请允许此站点打开弹窗后重试");
    }
    try { child.opener = null; } catch (_) {}

    let payload = null;
    let settled = false;
    let receiverReady = false;
    let deckSent = false;
    let resolveAck, rejectAck;
    const ack = new Promise((resolve, reject) => { resolveAck = resolve; rejectAck = reject; });
    // A popup can be cancelled before `publish()` obtains this promise.  Mark
    // the internal promise handled while still returning the same rejecting
    // promise to callers that do publish, avoiding a stray unhandled rejection.
    void ack.catch(() => {});
    const close = () => {
      if (settled) return;
      settled = true;
      global.clearTimeout(timeout);
      global.clearInterval(childWatch);
      channel.close();
    };
    const fail = (error, closeWindow = false) => {
      if (settled) return;
      close();
      if (closeWindow) try { child.close(); } catch (_) {}
      rejectAck(error instanceof Error ? error : new Error(String(error || "新窗口传输失败")));
    };
    const sendDeck = () => {
      if (!payload || !receiverReady || deckSent || settled) return;
      try {
        channel.postMessage(message("deck", token, payload));
        deckSent = true;
      } catch (error) {
        fail(error, true);
      }
    };
    channel.onmessage = (event) => {
      const data = event?.data || {};
      if (data.protocol !== PROTOCOL || data.token !== token) return;
      if (data.type === "ready") {
        receiverReady = true;
        sendDeck();
      }
      else if (data.type === "ack") {
        close();
        resolveAck({ token, opened: true });
      } else if (data.type === "error") fail(new Error(String(data.error || "新窗口无法装载文稿")));
    };
    const timeout = global.setTimeout(() => fail(new Error("新窗口装载文稿超时"), true), Math.max(1000, Number(options.timeoutMs) || DEFAULT_TIMEOUT_MS));
    const childWatch = global.setInterval(() => {
      if (child.closed === true) fail(new Error("新文稿窗口已关闭，当前文稿保持不变"));
    }, 250);

    return Object.freeze({
      kind: "new-window",
      publish(deck, outcome = {}) {
        if (settled) return Promise.reject(new Error("新窗口传输会话已经结束"));
        payload = { deck, outcome };
        sendDeck();
        return ack;
      },
      cancel(reason = "已取消新文稿窗口") { fail(new Error(reason), true); },
    });
  }

  function receive(options = {}) {
    const token = receiverToken();
    if (!token) return Promise.reject(new Error("新文稿窗口缺少有效令牌"));
    if (typeof global.BroadcastChannel !== "function") return Promise.reject(new Error("浏览器不支持安全的新文稿窗口传输"));
    const channel = new global.BroadcastChannel(channelName(token));
    let settled = false;
    let delivered = false;
    return new Promise((resolve, reject) => {
      const clearFragment = () => {
        try {
          const url = new URL(global.location.href);
          url.hash = "";
          global.history?.replaceState?.(global.history.state, "", url.href);
        } catch (_) {}
      };
      const close = () => {
        if (settled) return;
        settled = true;
        global.clearTimeout(timeout);
        global.clearInterval(readyTimer);
        channel.close();
      };
      const post = (type, payload = {}) => {
        try {
          channel.postMessage(message(type, token, payload));
          return true;
        } catch (_) {
          return false;
        }
      };
      const ready = () => {
        if (!delivered && !settled) post("ready");
      };
      channel.onmessage = (event) => {
        const data = event?.data || {};
        if (data.protocol !== PROTOCOL || data.token !== token || data.type !== "deck" || delivered) return;
        const deck = data.deck;
        if (!deck || typeof deck !== "object" || !Array.isArray(deck.slides)) {
          post("error", { error: "收到的文稿结构无效" });
          clearFragment();
          close();
          reject(new Error("收到的文稿结构无效"));
          return;
        }
        delivered = true;
        global.clearInterval(readyTimer);
        resolve({
          deck,
          outcome: data.outcome || {},
          accept() {
            if (settled) return false;
            post("ack");
            clearFragment();
            close();
            return true;
          },
          reject(error) {
            if (settled) return false;
            post("error", { error: String(error?.message || error || "新窗口无法装载文稿") });
            clearFragment();
            close();
            return true;
          },
        });
      };
      const timeout = global.setTimeout(() => {
        post("error", { error: delivered ? "新窗口装载文稿超时" : "等待父窗口发送文稿超时" });
        clearFragment();
        close();
        if (!delivered) reject(new Error("等待生成的演示文稿超时；原窗口中的文稿未受影响"));
      }, Math.max(1000, Number(options.timeoutMs) || DEFAULT_TIMEOUT_MS));
      const readyTimer = global.setInterval(ready, 250);
      ready();
    });
  }

  global.UniPptDocumentHandoff = Object.freeze({
    open, receive, receiverToken,
    __test: Object.freeze({ PROTOCOL, HASH_KEY, channelName, message, receiverUrl }),
  });
})(globalThis);
