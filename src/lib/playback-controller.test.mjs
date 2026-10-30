import test from "node:test";
import assert from "node:assert/strict";
import { PlaybackController, formatTime } from "./playback-controller.ts";

class FakeAudio extends EventTarget {
  error = null;
  src = "";
  currentTime = 0;
  duration = 120;
  paused = true;
  ended = false;
  playResult = null;
  loads = 0;
  playCalls = 0;
  play() {
    ++this.playCalls;
    this.paused = false;
    return this.playResult ?? Promise.resolve();
  }
  pause() { this.paused = true; this.dispatchEvent(new Event("pause")); }
  load() { ++this.loads; this.currentTime = 0; }
  removeAttribute(name) { assert.equal(name, "src"); this.src = ""; }
}
const first = { libraryId: "library", assetId: "one", name: "one.wav" };
const second = { libraryId: "library", assetId: "two", name: "two.ogg" };
const source = (id) => ({ url: `http://127.0.0.1:43123/audio/${id.padEnd(32, "0")}` });
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function player(prepare = async (request) => source(request.assetId)) {
  const media = new FakeAudio();
  return { media, controller: new PlaybackController(media, prepare) };
}

test("single player plays, pauses, resumes, seeks and reports duration", async () => {
  const { media, controller } = player();
  await controller.select(first);
  assert.equal(controller.getSnapshot().phase, "playing");
  assert.equal(media.playCalls, 1);
  await controller.toggle();
  assert.equal(media.paused, true);
  assert.equal(controller.getSnapshot().phase, "paused");
  await controller.toggle();
  assert.equal(media.playCalls, 2);
  media.dispatchEvent(new Event("loadedmetadata"));
  assert.equal(controller.getSnapshot().duration, 120);
  controller.seek(42);
  assert.equal(media.currentTime, 42);
  controller.seek(999);
  assert.equal(media.currentTime, 120);
  controller.seek(-1);
  assert.equal(media.currentTime, 0);
  controller.seek(NaN);
  assert.equal(media.currentTime, 0);
  media.currentTime = 18;
  media.dispatchEvent(new Event("timeupdate"));
  assert.equal(controller.getSnapshot().currentTime, 18);
  assert.equal(formatTime(125.9), "2:05");
  assert.equal(formatTime(Infinity), "0:00");
  controller.clear();
});

test("switching stops the previous audio immediately while prepare is pending", async () => {
  const next = deferred();
  const { media, controller } = player(async (request) => request.assetId === "two" ? next.promise : source("one"));
  await controller.select(first);
  const pending = controller.select(second);
  assert.equal(media.paused, true);
  assert.equal(media.src, "");
  assert.equal(controller.getSnapshot().phase, "loading");
  next.resolve(source("two"));
  await pending;
  assert.equal(media.src, source("two").url);
  assert.equal(controller.getSnapshot().selection.assetId, "two");
});

test("late prepare successes and failures cannot replace a newer selection", async () => {
  for (const reject of [false, true]) {
    const old = deferred();
    const { media, controller } = player(async (request) => request.assetId === "one" ? old.promise : source("two"));
    const pending = controller.select(first);
    await controller.select(second);
    if (reject) old.reject({ code: "notAuthorized", message: "Old root revoked" });
    else old.resolve(source("one"));
    await pending;
    assert.equal(media.src, source("two").url);
    assert.equal(controller.getSnapshot().phase, "playing");
    assert.equal(controller.getSnapshot().message, null);
  }
});

test("late play rejection and completion are ignored after selection changes", async () => {
  for (const reject of [false, true]) {
    const old = deferred();
    const { media, controller } = player();
    media.playResult = old.promise;
    const pending = controller.select(first);
    await Promise.resolve();
    media.playResult = null;
    await controller.select(second);
    if (reject) old.reject(new Error("interrupted")); else old.resolve();
    await pending;
    assert.equal(controller.getSnapshot().phase, "playing");
    assert.equal(media.src, source("two").url);
  }
});

test("prepare authorization errors and media decoding errors are recoverable", async () => {
  let denied = true;
  const { media, controller } = player(async () => {
    if (denied) throw { code: "notAuthorized", message: "Select the directory again." };
    return source("one");
  });
  await controller.select(first);
  assert.equal(controller.getSnapshot().phase, "error");
  assert.match(controller.getSnapshot().message, /Select the directory/);
  assert.equal(controller.getSnapshot().selection.assetId, "one");
  denied = false;
  await controller.retry();
  assert.equal(controller.getSnapshot().phase, "playing");
  media.dispatchEvent(new Event("error"));
  assert.equal(controller.getSnapshot().phase, "error");
  assert.equal(media.src, "");
  assert.equal(media.paused, true);
  assert.match(controller.getSnapshot().message, /codec/);
  await controller.retry();
  assert.equal(controller.getSnapshot().phase, "playing");
});

test("clear invalidates pending requests and detaches old media listeners", async () => {
  const pending = deferred();
  const { media, controller } = player(() => pending.promise);
  const selecting = controller.select(first);
  controller.clear();
  pending.resolve(source("one"));
  await selecting;
  assert.equal(media.src, "");
  assert.equal(controller.getSnapshot().phase, "idle");
  const ready = player();
  await ready.controller.select(first);
  ready.controller.clear();
  ready.media.dispatchEvent(new Event("error"));
  ready.media.dispatchEvent(new Event("playing"));
  assert.equal(ready.controller.getSnapshot().phase, "idle");
});

test("unknown duration disables seeking; ended audio resumes from zero", async () => {
  const { media, controller } = player();
  media.duration = Infinity;
  await controller.select(first);
  media.dispatchEvent(new Event("durationchange"));
  controller.seek(5);
  assert.equal(media.currentTime, 0);
  media.ended = true;
  media.currentTime = 120;
  media.dispatchEvent(new Event("ended"));
  await controller.toggle();
  assert.equal(media.currentTime, 0);
  assert.equal(controller.getSnapshot().phase, "playing");
});

test("WebView autoplay policy leaves a prepared source ready for an explicit Play", async () => {
  const { media, controller } = player();
  media.playResult = Promise.reject(new DOMException("Gesture required", "NotAllowedError"));
  await controller.select(first);
  assert.equal(controller.getSnapshot().phase, "paused");
  assert.match(controller.getSnapshot().message, /Press Play/);
  assert.notEqual(media.src, "");
  media.playResult = null;
  await controller.toggle();
  assert.equal(controller.getSnapshot().phase, "playing");
  assert.equal(controller.getSnapshot().message, null);
});


test("NotSupportedError identifies the media source instead of suggesting reauthorization", async () => {
  const { media, controller } = player();
  media.playResult = Promise.reject(new DOMException("Unsupported source", "NotSupportedError"));
  await controller.select(first);
  assert.equal(controller.getSnapshot().phase, "error");
  assert.match(controller.getSnapshot().message, /source or codec is unsupported/);
  assert.doesNotMatch(controller.getSnapshot().message, /operation failed|authorization/);
});

test("play rejection cannot overwrite the preceding media error event", async () => {
  const pending = deferred();
  const { media, controller } = player();
  media.playResult = pending.promise;
  const selecting = controller.select(first);
  await Promise.resolve();
  media.error = { code: 3 };
  media.dispatchEvent(new Event("error"));
  const message = controller.getSnapshot().message;
  pending.reject(new DOMException("Unsupported source", "NotSupportedError"));
  await selecting;
  assert.match(message, /could not be decoded/);
  assert.equal(controller.getSnapshot().message, message);
});
