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
    listTags: async () => [],
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

test("tag edits, favorites and assignment use typed IDs while a saved root is inactive", async () => {
  const calls = [];
  let tags = [];
  let assigned = [];
  let favorite = false;
  const controller = new LibraryController(api({
    listTags: async () => tags,
    createTag: async (request) => { calls.push(request); tags = [fixture.tag]; return fixture.tag; },
    renameTag: async (request) => { calls.push(request); tags = [{ ...fixture.tag, name: request.name }]; return tags[0]; },
    deleteTag: async (request) => { calls.push(request); tags = []; assigned = []; },
    setAssetTags: async (request) => { calls.push(request); assigned = tags.filter((tag) => request.tagIds.includes(tag.id)); },
    setAssetFavorite: async (request) => { calls.push(request); favorite = request.favorite; },
    listAssets: async () => ({ ...fixture.page, assets: [{ ...fixture.page.assets[0], tags: assigned, favorite }] }),
  }));
  await controller.initialize();
  assert.equal(controller.getSnapshot().active, false);
  await controller.createTag(" 雨 ");
  assert.deepEqual(controller.getSnapshot().tags, [fixture.tag]);
  await controller.setTags("asset-fixture", [fixture.tag.id]);
  assert.deepEqual(calls.at(-1), fixture.setAssetTagsRequest);
  assert.deepEqual(controller.getSnapshot().assets[0].tags, [fixture.tag]);
  await controller.setFavorite("asset-fixture", true);
  assert.deepEqual(calls.at(-1), fixture.setAssetFavoriteRequest);
  assert.equal(controller.getSnapshot().assets[0].favorite, true);
  await controller.setFavorite("asset-fixture", false);
  assert.equal(controller.getSnapshot().assets[0].favorite, false);
  await controller.renameTag(fixture.tag.id, "Rain");
  assert.deepEqual(calls.at(-1), fixture.renameTagRequest);
  await controller.setTags("asset-fixture", []);
  assert.deepEqual(controller.getSnapshot().assets[0].tags, []);
  await controller.filter([fixture.tag.id], true);
  await controller.deleteTag(fixture.tag.id);
  assert.deepEqual(calls.at(-1), fixture.deleteTagRequest);
  assert.deepEqual(controller.getSnapshot().tags, []);
  assert.deepEqual(controller.getSnapshot().tagIds, []);
  assert.equal(controller.getSnapshot().metadataBusy, false);
  controller.cancelPending();
});

test("filters combine with search and pagination and reject late filter results", async () => {
  const old = deferred();
  const requests = [];
  const controller = new LibraryController(api({ listAssets: async (request) => {
    requests.push(request);
    if (request.tagIds.includes("old")) return old.promise;
    return { ...fixture.page, total: 120, offset: request.offset };
  } }));
  await controller.initialize();
  await controller.search("雨%_");
  const pending = controller.filter(["old"], false);
  await controller.filter(["rain", "wind"], true);
  await controller.page(50);
  assert.deepEqual(requests.at(-1), { libraryId: fixture.library.id, search: "雨%_", tagIds: ["rain", "wind"], favoritesOnly: true, offset: 50, limit: 50 });
  old.resolve(fixture.emptyPage);
  await pending;
  assert.equal(controller.getSnapshot().offset, 50);
  assert.equal(controller.getSnapshot().total, 120);
  await controller.search("new");
  assert.equal(controller.getSnapshot().offset, 0);
  assert.deepEqual(controller.getSnapshot().tagIds, ["rain", "wind"]);
  controller.cancelPending();
});

test("metadata validation and database failures retain state and allow retry", async () => {
  let failure = { code: "invalidInput", message: "Invalid name." };
  const controller = new LibraryController(api({
    createTag: async () => { if (failure) throw failure; return fixture.tag; },
    listTags: async () => failure ? [] : [fixture.tag],
  }));
  await controller.initialize();
  for (const code of ["invalidInput", "duplicateTag", "databaseError", "notFound"]) {
    failure = { code, message: "Cannot save." };
    await controller.createTag("test");
    assert.equal(controller.getSnapshot().metadataError.code, code);
    assert.equal(controller.getSnapshot().metadataBusy, false);
    assert.deepEqual(controller.getSnapshot().assets, fixture.page.assets);
  }
  failure = null;
  await controller.createTag("test");
  assert.equal(controller.getSnapshot().metadataError, null);
  assert.deepEqual(controller.getSnapshot().tags, [fixture.tag]);
  controller.cancelPending();
});

test("pending mutation blocks duplicate writes and refreshes the current query after navigation", async () => {
  const save = deferred();
  const staleQuery = deferred();
  let calls = 0;
  let saved = false;
  const controller = new LibraryController(api({
    setAssetFavorite: async () => { calls++; await save.promise; saved = true; },
    listAssets: async (request) => request.search === "old" ? staleQuery.promise : ({ ...fixture.page, assets: [{ ...fixture.page.assets[0], favorite: saved }] }),
  }));
  await controller.initialize();
  const stale = controller.search("old");
  const saving = controller.setFavorite("asset-fixture", true);
  await controller.setFavorite("asset-fixture", true);
  assert.equal(calls, 1);
  await controller.search("current");
  save.resolve();
  await saving;
  staleQuery.resolve(fixture.emptyPage);
  await stale;
  assert.equal(controller.getSnapshot().assets[0].favorite, true);
  assert.equal(controller.getSnapshot().search, "current");
  controller.cancelPending();
});

test("late annotation errors after library change or cleanup cannot overwrite the new view", async () => {
  for (const cleanup of [false, true]) {
    const save = deferred();
    const controller = new LibraryController(api({ setAssetTags: () => save.promise }));
    await controller.initialize();
    const saving = controller.setTags("asset-fixture", ["tag"]);
    if (cleanup) controller.cancelPending();
    else await controller.openLibrary({ library: { id: "new", displayPath: "New" }, active: false });
    const before = controller.getSnapshot().assets;
    save.reject({ code: "databaseError", message: "Old failure." });
    await saving;
    assert.equal(controller.getSnapshot().metadataError, null);
    assert.deepEqual(controller.getSnapshot().assets, before);
    controller.cancelPending();
  }
});

test("tag loading discards older responses and recovers after database failure", async () => {
  const old = deferred();
  let calls = 0;
  const controller = new LibraryController(api({ listTags: () => ++calls === 1 ? old.promise : Promise.resolve([fixture.tag]) }));
  const loading = controller.loadTags();
  await controller.loadTags();
  old.resolve([]);
  await loading;
  assert.deepEqual(controller.getSnapshot().tags, [fixture.tag]);
  controller.cancelPending();
  const retry = new LibraryController(api({ listTags: async () => { throw { code: "databaseError", message: "Unavailable." }; } }));
  await retry.loadTags();
  assert.equal(retry.getSnapshot().tagsLoading, false);
  assert.equal(retry.getSnapshot().metadataError.code, "databaseError");
  retry.cancelPending();
});

test("initial tag database failures remain visible when saved libraries finish loading", async () => {
  const controller = new LibraryController(api({ listTags: async () => { throw { code: "databaseError", message: "Tag database unavailable." }; } }));
  await controller.initialize();
  assert.equal(controller.getSnapshot().metadataError.code, "databaseError");
  assert.equal(controller.getSnapshot().tagsLoading, false);
  controller.cancelPending();
});
