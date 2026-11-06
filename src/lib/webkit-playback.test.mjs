import test from "node:test";
import assert from "node:assert/strict";
import { waitForDuration } from "../../src-tauri/tests/support/webkit-playback.mjs";

class MetadataAudio extends EventTarget {
  duration = 0;
  error = null;
}

test("WebKit duration gate waits through zero metadata until loadeddata supplies the duration", async () => {
  const audio = new MetadataAudio();
  let settled = false;
  const ready = waitForDuration(audio, 10).then(() => { settled = true; });
  audio.dispatchEvent(new Event("loadedmetadata"));
  await Promise.resolve();
  assert.equal(settled, false);
  // Observed WebKit sequence: no second durationchange before loadeddata.
  audio.duration = 10.152;
  audio.dispatchEvent(new Event("loadeddata"));
  await ready;
  assert.equal(settled, true);
});

test("WebKit duration gate accepts already available metadata and later duration changes", async () => {
  for (const event of [null, "durationchange", "canplay", "timeupdate"]) {
    const audio = new MetadataAudio();
    if (!event) audio.duration = 10;
    const ready = waitForDuration(audio, 10);
    audio.duration = 10;
    if (event) audio.dispatchEvent(new Event(event));
    await ready;
  }
});

test("WebKit duration gate rejects persistent zero, NaN, infinity and incorrect finite durations", async () => {
  for (const duration of [0, NaN, Infinity, 20]) {
    const audio = new MetadataAudio();
    audio.duration = duration;
    await assert.rejects(waitForDuration(audio, 10, 10), /duration did not settle/);
  }
});

test("WebKit duration gate rejects media errors without converting them into a pass", async () => {
  const audio = new MetadataAudio();
  const ready = waitForDuration(audio, 10);
  audio.error = { code: 3 };
  audio.dispatchEvent(new Event("error"));
  await assert.rejects(ready, /media error 3/);
  await assert.rejects(waitForDuration(audio, 10), /media error 3/);
});
