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
    listAssets: async () => fixture.page,
    listLibraries: async () => fixture.savedLibraries,
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
    listAssets: () => { if (!scanned || selected.id !== fixture.library.id) return Promise.resolve(fixture.emptyPage); requested.resolve(); return pending.promise; },
  }));
  await controller.selectDirectory();
  const scanning = controller.scan();
  await requested.promise;
  selected = { id: "other-library", displayPath: "/synthetic/other" };
  await controller.selectDirectory();
  pending.resolve(fixture.page);
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

test("partial and empty results remain usable, revoked authorization keeps the saved library", async () => {
  let scanned = false;
  let revoked = false;
  const controller = new LibraryController(api({
    startScan: async () => {
      if (revoked) throw fixture.error;
      scanned = true;
      return fixture.running;
    },
    getStatus: async () => scanned ? fixture.partial : fixture.idle,
    listAssets: async () => ({ ...fixture.emptyPage, scanId: fixture.partial.scanId }),
  }));
  await controller.selectDirectory();
  await controller.scan();
  assert.equal(controller.getSnapshot().status.phase, "partial");
  assert.deepEqual(controller.getSnapshot().assets, []);
  assert.equal(controller.getSnapshot().error, null);
  revoked = true;
  await controller.scan();
  assert.deepEqual(controller.getSnapshot().library, fixture.library);
  assert.equal(controller.getSnapshot().active, false);
  assert.equal(controller.getSnapshot().error.code, "notAuthorized");
  controller.cancelPending();
});

test("busy scans are polled and mismatched scan results are rejected", async () => {
  let scanned = false;
  const controller = new LibraryController(api({
    startScan: async () => { scanned = true; throw { code: "busy", message: "Already scanning." }; },
    getStatus: async () => scanned ? fixture.complete : fixture.idle,
    listAssets: async () => ({ ...fixture.page, scanId: "stale-scan" }),
  }));
  await controller.selectDirectory();
  await controller.scan();
  assert.deepEqual(controller.getSnapshot().assets, fixture.page.assets); // Last stored snapshot remains visible.
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


test("restart loads saved assets while inactive, reselecting the same library authorizes scanning", async () => {
  let starts = 0;
  const controller = new LibraryController(api({ startScan: async () => { starts++; return fixture.running; }, getStatus: async () => fixture.idle }));
  await controller.initialize();
  assert.deepEqual(controller.getSnapshot().assets, fixture.page.assets);
  assert.equal(controller.getSnapshot().active, false);
  await controller.scan();
  assert.equal(starts, 0);
  await controller.selectDirectory();
  assert.equal(controller.getSnapshot().active, true);
  controller.cancelPending();
});

test("new search and navigation discard late queries, and search resets pagination", async () => {
  const old = deferred();
  const requests = [];
  const controller = new LibraryController(api({ listAssets: async (request) => {
    requests.push(request);
    if (request.search === "old") return old.promise;
    return { ...fixture.page, offset: request.offset, total: 120 };
  } }));
  await controller.initialize();
  await controller.page(50);
  assert.equal(controller.getSnapshot().offset, 50);
  const searching = controller.search("old");
  await controller.search(fixture.query.search);
  assert.equal(controller.getSnapshot().offset, 0);
  assert.deepEqual(requests.at(-1), fixture.query);
  old.resolve({ ...fixture.emptyPage });
  await searching;
  assert.equal(controller.getSnapshot().search, fixture.query.search);
  assert.equal(controller.getSnapshot().total, 120);
  controller.cancelPending();
});

test("database query failure is recoverable and retains the saved library", async () => {
  let fail = true;
  const controller = new LibraryController(api({ listAssets: async () => {
    if (fail) throw { code: "databaseError", message: "Database unavailable." };
    return fixture.page;
  } }));
  await controller.initialize();
  assert.equal(controller.getSnapshot().error.code, "databaseError");
  assert.equal(controller.getSnapshot().loading, false);
  assert.deepEqual(controller.getSnapshot().library, fixture.library);
  fail = false;
  await controller.reload();
  assert.equal(controller.getSnapshot().error, null);
  assert.deepEqual(controller.getSnapshot().assets, fixture.page.assets);
  controller.cancelPending();
});

test("a shrinking last page returns to the final valid page", async () => {
  const offsets = [];
  const controller = new LibraryController(api({ listAssets: async (request) => {
    offsets.push(request.offset);
    return { ...fixture.page, assets: request.offset < 51 ? fixture.page.assets : [], offset: request.offset, total: 51 };
  } }));
  await controller.initialize();
  await controller.page(100);
  assert.equal(controller.getSnapshot().offset, 50);
  assert.deepEqual(offsets, [0,100,50]);
  controller.cancelPending();
});
