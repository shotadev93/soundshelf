import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parseAsset, parseAssets, parseError, parseLibrary, parseStatus } from "./contracts.ts";
const fixture = JSON.parse(readFileSync(new URL("./fixtures/ipc.json", import.meta.url), "utf8"));

test("real IPC fixtures match the checked TypeScript boundary", () => {
  assert.deepEqual(parseLibrary(fixture.library), fixture.library);
  assert.equal(parseLibrary(fixture.cancellation), null);
  assert.deepEqual(parseAssets(fixture.assets), fixture.assets);
  assert.deepEqual(parseAsset(fixture.inaccessible), fixture.inaccessible);
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
