"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

class FakeClassList {
  constructor(owner) { this.owner = owner; this.values = new Set(); }
  add(...values) { values.forEach((value) => this.values.add(value)); }
  remove(...values) { values.forEach((value) => this.values.delete(value)); }
  contains(value) { return this.values.has(value); }
}

class FakeElement {
  constructor(tagName = "div") {
    this.tagName = tagName.toUpperCase();
    this.id = "";
    this.hidden = false;
    this.dataset = {};
    this.children = [];
    this.parentNode = null;
    this.parentElement = null;
    this.className = "";
    this.classList = new FakeClassList(this);
    this.listeners = new Map();
    this.paused = true;
    this.currentTime = 0;
    this.duration = Number.NaN;
    this.ended = false;
    this.playCount = 0;
    this.pauseCount = 0;
  }
  setAttribute() {}
  append(child) {
    if (child.parentNode) child.parentNode.children = child.parentNode.children.filter((entry) => entry !== child);
    child.parentNode = this;
    child.parentElement = this;
    this.children.push(child);
  }
  replaceChildren(...children) {
    this.children.forEach((child) => { child.parentNode = null; child.parentElement = null; });
    this.children = [];
    children.forEach((child) => this.append(child));
  }
  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) || [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }
  dispatch(type) { (this.listeners.get(type) || []).forEach((listener) => listener({ type })); }
  matches(selector) {
    if (selector === "audio.native-media") return this.tagName === "AUDIO" && this.className.includes("native-media");
    if (selector === "video.native-media") return this.tagName === "VIDEO" && this.className.includes("native-media");
    return false;
  }
  querySelectorAll(selector) {
    const output = [];
    const visit = (node) => {
      for (const child of node.children) {
        if (
          selector === "audio.native-media,video.native-media"
          && (child.matches("audio.native-media") || child.matches("video.native-media"))
        ) output.push(child);
        visit(child);
      }
    };
    visit(this);
    return output;
  }
  querySelector(selector) {
    if (selector === "audio.native-media,video.native-media") return this.querySelectorAll(selector)[0] || null;
    return null;
  }
  play() {
    this.playCount += 1;
    this.paused = false;
    this.dispatch("play");
    return Promise.resolve();
  }
  pause() {
    this.pauseCount += 1;
    this.paused = true;
    this.dispatch("pause");
  }
}

let audioCreateCount = 0;
const document = {
  body: new FakeElement("body"),
  createElement(tagName) {
    if (tagName === "audio") audioCreateCount += 1;
    return new FakeElement(tagName);
  },
  getElementById(id) {
    let match = null;
    const visit = (node) => {
      if (node.id === id) match = node;
      node.children.forEach(visit);
    };
    visit(this.body);
    return match;
  },
};
const context = { document };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, "media_runtime.js"), "utf8"), context);

function mediaObject(id = "audio-1") {
  return {
    id,
    sourceShapeId: id,
    asset: null,
    media: {
      kind: "audio",
      asset: "data:audio/mpeg;base64,AAAA",
      sourcePartName: "/ppt/media/media3.mp3",
      relationshipId: "rId3",
      trimStartMs: 0,
      trimEndMs: 2863,
      volume: 1,
      loopPlayback: true,
      playAcrossSlides: true,
    },
  };
}

const derivedOwner = new FakeElement("div");
const derivedObject = mediaObject("derived-video");
derivedObject.media = {
  ...derivedObject.media,
  kind: "video",
  asset: "data:video/x-msvideo;base64,TkFUSVZF",
  mimeType: "video/x-msvideo",
  playbackAsset: "data:video/mp4;base64,UExBWUFCTEU=",
  playbackMimeType: "video/mp4",
  playAcrossSlides: false,
};
const derivedPlayer = context.UniPptMedia.attach(derivedOwner, derivedObject, false);
assert.equal(
  derivedPlayer.src,
  "data:video/mp4;base64,UExBWUFCTEU=",
  "browser playback must prefer the compatible derivative",
);
assert.equal(
  derivedObject.media.asset,
  "data:video/x-msvideo;base64,TkFUSVZF",
  "attaching a derivative must retain the original native asset",
);

const fallbackOwner = new FakeElement("div");
const fallbackObject = mediaObject("fallback-video");
fallbackObject.media = {
  ...fallbackObject.media,
  kind: "video",
  asset: "data:video/mp4;base64,TkFUSVZF",
  mimeType: "video/mp4",
  playbackAsset: null,
  playbackMimeType: null,
  playAcrossSlides: false,
};
const fallbackPlayer = context.UniPptMedia.attach(fallbackOwner, fallbackObject, false);
assert.equal(
  fallbackPlayer.src,
  "data:video/mp4;base64,TkFUSVZF",
  "native browser-compatible media remains the fallback source",
);

const stage1 = new FakeElement("div");
const owner1 = new FakeElement("div");
stage1.append(owner1);
const descriptor = mediaObject();
const player = context.UniPptMedia.attach(
  owner1,
  descriptor,
  false,
  { scope: "presentation", slideKey: "/ppt/slides/slide1.xml" },
);
player.duration = 133.982;
player.dispatch("loadedmetadata");
assert.ok(Math.abs(Number(player.dataset.playbackEnd) - 131.119) < 0.001, "trim end must be subtracted from native duration");
assert.equal(context.UniPptMedia.effectivePlaybackEnd(133.982, 2863), 131.119);

player.currentTime = 37;
player.play();
context.UniPptMedia.stopAll(stage1, true, true);
const persistent = document.getElementById("unippt-persistent-media");
assert.equal(player.parentElement, persistent, "cross-slide playback must move to the persistent host");
assert.equal(player.currentTime, 37, "moving between slides must not seek or restart audio");

const owner1Again = new FakeElement("div");
const samePlayer = context.UniPptMedia.attach(
  owner1Again,
  descriptor,
  false,
  { scope: "presentation", slideKey: "/ppt/slides/slide1.xml" },
);
assert.equal(samePlayer, player, "returning to the source slide must reuse the same decoder");
assert.equal(audioCreateCount, 1);
assert.equal(samePlayer.currentTime, 37);
context.UniPptMedia.controlNode(owner1Again, "play");
assert.equal(samePlayer.currentTime, 37, "a repeated play effect must be idempotent while audio is active");
assert.equal(context.UniPptMedia.debugRegistrySize(), 1);

const stage2 = new FakeElement("div");
stage2.append(owner1Again);
context.UniPptMedia.stopAll(stage2, true, true);
const owner2 = new FakeElement("div");
const continuedPlayer = context.UniPptMedia.attach(
  owner2,
  { ...mediaObject("audio-2"), media: { ...descriptor.media, relationshipId: "rId9" } },
  false,
  { scope: "presentation", slideKey: "/ppt/slides/slide2.xml" },
);
assert.equal(continuedPlayer, player, "a unique persistent cross-slide source may acquire a new slide owner");

const independentOwner = new FakeElement("div");
const independentPlayer = context.UniPptMedia.attach(
  independentOwner,
  mediaObject("audio-independent"),
  false,
  { scope: "presentation", slideKey: "/ppt/slides/slide2.xml" },
);
assert.notEqual(independentPlayer, player, "two simultaneous insertions of one source must remain independent");
assert.equal(audioCreateCount, 2);

console.log("media reuse tests passed");
