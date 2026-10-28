import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { LibraryController } from "./library-controller.ts";
const fixture = JSON.parse(readFileSync(new URL("./fixtures/ipc.json", import.meta.url), "utf8"));

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function api(overrides = {}) {
  let scanned = false;
  return {
    selectDirectory: async () => fixture.library,
    startScan: async () => { scanned = true; return fixture.running; },
    getStatus: async () => scanned ? fixture.complete : fixture.idle,
    listAssets: async () => fixture.assets,
    ...overrides,
  };
}

test("selection, scan and results flow; canceled selection preserves the view", async () => {
  let canceled = false;
  const controller = new LibraryController(api({ selectDirectory: async () => canceled ? null : fixture.library }));
  await controller.selectDirectory();
  await controller.scan();
  assert.deepEqual(controller.getSnapshot().assets, fixture.assets.assets);
  const before = controller.getSnapshot();
  canceled = true;
  await controller.selectDirectory();
  assert.deepEqual(controller.getSnapshot(), before);
  controller.cancelPending();
});

test("a late old-library result cannot overwrite the newly selected library", async () => {
  const pending = deferred();
  const requested = deferred();
  let selected = fixture.library;
  let scanned = false;
  const controller = new LibraryController(api({
    selectDirectory: async () => selected,
    startScan: async () => { scanned = true; return fixture.running; },
    getStatus: async () => selected.id === fixture.library.id && scanned ? fixture.complete : fixture.idle,
    listAssets: () => { requested.resolve(); return pending.promise; },
  }));
  await controller.selectDirectory();
  const scanning = controller.scan();
  await requested.promise;
  selected = { id: "other-library", displayPath: "/synthetic/other" };
  await controller.selectDirectory();
  pending.resolve(fixture.assets);
  await scanning;
  assert.equal(controller.getSnapshot().library.id, "other-library");
  assert.deepEqual(controller.getSnapshot().assets, []);
  controller.cancelPending();
});

test("canceling a picker does not cancel an in-progress scan", async () => {
  const pending = deferred();
  const requested = deferred();
  let scanned = false;
  let canceled = false;
  const controller = new LibraryController(api({
    selectDirectory: async () => canceled ? null : fixture.library,
    startScan: async () => { scanned = true; return fixture.running; },
    getStatus: () => {
      if (!scanned) return Promise.resolve(fixture.idle);
      requested.resolve();
      return pending.promise;
    },
  }));
  await controller.selectDirectory();
  const scanning = controller.scan();
  await requested.promise;
  canceled = true;
  await controller.selectDirectory();
  assert.equal(controller.getSnapshot().scanning, true);
  pending.resolve(fixture.complete);
  await scanning;
  assert.deepEqual(controller.getSnapshot().assets, fixture.assets.assets);
  controller.cancelPending();
});

test("partial and empty results remain usable, revoked authorization clears results", async () => {
  let scanned = false;
  let revoked = false;
  const controller = new LibraryController(api({
    startScan: async () => {
      if (revoked) throw fixture.error;
      scanned = true;
      return fixture.running;
    },
    getStatus: async () => scanned ? fixture.partial : fixture.idle,
    listAssets: async () => ({ scanId: fixture.partial.scanId, assets: [] }),
  }));
  await controller.selectDirectory();
  await controller.scan();
  assert.equal(controller.getSnapshot().status.phase, "partial");
  assert.deepEqual(controller.getSnapshot().assets, []);
  assert.equal(controller.getSnapshot().error, null);
  revoked = true;
  await controller.scan();
  assert.equal(controller.getSnapshot().library, null);
  assert.equal(controller.getSnapshot().error.code, "notAuthorized");
  controller.cancelPending();
});

test("busy scans are polled and mismatched scan results are rejected", async () => {
  let scanned = false;
  const controller = new LibraryController(api({
    startScan: async () => { scanned = true; throw { code: "busy", message: "Already scanning." }; },
    getStatus: async () => scanned ? fixture.complete : fixture.idle,
    listAssets: async () => ({ ...fixture.assets, scanId: "stale-scan" }),
  }));
  await controller.selectDirectory();
  await controller.scan();
  assert.deepEqual(controller.getSnapshot().assets, []);
  assert.equal(controller.getSnapshot().scanning, false);
  assert.match(controller.getSnapshot().error.message, /different scan/);
  controller.cancelPending();
});

test("running scans are polled until completion", { timeout: 2000 }, async () => {
  let scanned = false;
  let polls = 0;
  const done = deferred();
  const controller = new LibraryController(api({
    startScan: async () => { scanned = true; return fixture.running; },
    getStatus: async () => !scanned ? fixture.idle : ++polls === 1 ? fixture.running : fixture.complete,
  }), 0);
  const unsubscribe = controller.subscribe(() => {
    const view = controller.getSnapshot();
    if (view.status?.phase === "complete" && !view.scanning) done.resolve();
  });
  try {
    await controller.selectDirectory();
    await controller.scan();
    assert.equal(controller.getSnapshot().scanning, true);
    await done.promise;
    assert.equal(polls, 2);
    assert.deepEqual(controller.getSnapshot().assets, fixture.assets.assets);
  } finally {
    unsubscribe();
    controller.cancelPending();
  }
});

test("late scan-start and picker responses are discarded after the view changes", async () => {
  const start = deferred();
  const requested = deferred();
  let selected = fixture.library;
  const controller = new LibraryController(api({
    selectDirectory: async () => selected,
    startScan: () => { requested.resolve(); return start.promise; },
    getStatus: async () => fixture.idle,
  }));
  await controller.selectDirectory();
  const scanning = controller.scan();
  await requested.promise;
  selected = { id: "new-library", displayPath: "/synthetic/new" };
  await controller.selectDirectory();
  start.resolve(fixture.running);
  await scanning;
  assert.equal(controller.getSnapshot().library.id, "new-library");
  assert.equal(controller.getSnapshot().status.phase, "idle");
  assert.equal(controller.getSnapshot().scanning, false);
  controller.cancelPending();

  const selection = deferred();
  const closed = new LibraryController(api({ selectDirectory: () => selection.promise }));
  const selecting = closed.selectDirectory();
  closed.cancelPending();
  selection.resolve(fixture.library);
  await selecting;
  assert.equal(closed.getSnapshot().library, null);
});
