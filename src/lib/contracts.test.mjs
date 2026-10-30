import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parsePlayback, parseAsset, parseAssets, parsePage, parseLibraries, parseError, parseLibrary, parseStatus } from "./contracts.ts";
const fixture = JSON.parse(readFileSync(new URL("./fixtures/ipc.json", import.meta.url), "utf8"));

test("real IPC fixtures match the checked TypeScript boundary", () => {
  assert.deepEqual(parseLibrary(fixture.library), fixture.library);
  assert.equal(parseLibrary(fixture.cancellation), null);
  assert.deepEqual(parseAssets(fixture.assets), fixture.assets);
  for (const key of ["inaccessible", "missing", "unknown"]) assert.deepEqual(parseAsset(fixture[key]), fixture[key]);
  for (const key of ["page", "emptyPage"]) assert.deepEqual(parsePage(fixture[key]), fixture[key]);
  assert.deepEqual(parseLibraries(fixture.savedLibraries), fixture.savedLibraries);
  for (const phase of ["idle", "running", "complete", "partial", "failed"]) {
    assert.deepEqual(parseStatus(fixture[phase]), fixture[phase]);
  }
  for (const code of fixture.errorCodes) {
    assert.deepEqual(parseError({ ...fixture.error, code }), { ...fixture.error, code });
  }
});

test("malformed IPC data cannot pass as typed assets or status", () => {
  for (const invalid of [null, {}, { ...fixture.assets.assets[0], availability: "playable" }, { ...fixture.assets.assets[0], durationSeconds: undefined }]) {
    assert.throws(() => parseAsset(invalid));
  }
  for (const invalid of [{ ...fixture.complete, phase: "done" }, { ...fixture.complete, visitedEntries: -1 }, { ...fixture.complete, warningCount: 2 ** 53 }, { ...fixture.complete, scanId: undefined }]) {
    assert.throws(() => parseStatus(invalid));
  }
});

test("pagination and saved-library response validation reject malformed data", () => {
  for (const invalid of [{ ...fixture.page, limit: 0 }, { ...fixture.page, limit: 201 }, { ...fixture.page, offset: -1 }, { ...fixture.page, total: 0 }, { ...fixture.page, assets: Array(51).fill(fixture.missing) }]) {
    assert.throws(() => parsePage(invalid));
  }
  assert.throws(() => parseLibraries([{ library: null, active: false }]));
  assert.throws(() => parseLibraries([{ library: fixture.library, active: "yes" }]));
});

test("playback IPC accepts only the scoped opaque URL", () => {
  assert.deepEqual(parsePlayback(fixture.playbackSource), fixture.playbackSource);
  for (const url of ["file:///etc/passwd", "https://example.com/audio.wav", "soundshelf-audio://localhost/../../etc/passwd", fixture.playbackSource.url + "?path=secret", fixture.playbackSource.url.replace("127.0.0.1", "example.com"), fixture.playbackSource.url.replace("43123", "65536"), fixture.playbackSource.url.replace("43123", "0"), "soundshelf-audio://localhost/0123456789abcdef0123456789abcdef"]) {
    assert.throws(() => parsePlayback({ url }));
  }
});
