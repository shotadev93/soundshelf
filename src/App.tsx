import { useEffect, useState, useSyncExternalStore } from "react";
import { LibraryController } from "./lib/library-controller";
import { libraryApi } from "./lib/ipc";
import "./App.css";

function App() {
  const [controller] = useState(() => new LibraryController(libraryApi));
  const view = useSyncExternalStore(controller.subscribe, controller.getSnapshot);
  useEffect(() => {
    void controller.initialize();
    return controller.cancelPending;
  }, [controller]);
  const [search, setSearch] = useState("");
  const { library, assets, status, selecting, scanning, error, loading, active } = view;
  useEffect(() => { setSearch(""); }, [library?.id]);
  const finished = status?.phase === "complete" || status?.phase === "partial";

  return (
    <main className="container">
      <h1>SoundShelf</h1>
      <p>Discover local WAV, MP3, and OGG audio. Original files are read-only.</p>
      <div className="toolbar" aria-label="Library actions">
        <button type="button" disabled={selecting} onClick={() => { void controller.selectDirectory(); }}>
          {selecting ? "Selecting directory…" : "Select audio directory"}
        </button>
        <button type="button" disabled={!library || !active || scanning || selecting} onClick={() => { void controller.scan(); }}>
          {scanning ? "Scanning…" : finished ? "Refresh scan" : "Start scan"}
        </button>
      </div>
      {view.libraries.length > 0 && <label className="saved-libraries">Saved library
        <select value={library?.id ?? ""} disabled={selecting} onChange={(event) => {
          const saved = view.libraries.find((entry) => entry.library.id === event.target.value);
          if (saved) void controller.openLibrary(saved);
        }}>
          {view.libraries.map((entry) => <option key={entry.library.id} value={entry.library.id}>
            {entry.library.displayPath}{entry.active ? "" : " (inactive)"}
          </option>)}
        </select>
      </label>}
      <p className="directory"><strong>Directory:</strong> {library?.displayPath ?? "No directory selected"}</p>
      {library && !active && <p>This saved library is inactive. Search remains available. Select its directory again through the picker to authorize a scan.</p>}
      <div role="status" aria-live="polite" aria-atomic="true">
        {scanning && <p>{status?.phase === "running"
          ? `Scanning: ${status.visitedEntries} entries checked, ${status.discoveredAssets} audio candidates found.`
          : "Loading scan status…"}</p>}
        {!scanning && finished && <p>{status.discoveredAssets} audio candidates found in this scan.{status.phase === "partial" ? " Results are partial." : " Scan complete."}</p>}
        {!scanning && active && library && status?.phase === "idle" && <p>Start a scan to discover audio files.</p>}
      </div>
      {error && <div className="error" role="alert"><p>{error.message}</p>
        <button type="button" disabled={loading} onClick={() => { void (library ? controller.reload() : controller.initialize()); }}>Retry loading</button>
      </div>}
      {status && status.warningCount > 0 && (
        <section aria-labelledby="warnings-heading" className="warnings">
          <h2 id="warnings-heading">Scan warnings ({status.warningCount})</h2>
          <ul>{status.warnings.map((warning, index) => <li key={index}>
            {warning.relativeDisplayPath && <span className="path">{warning.relativeDisplayPath}: </span>}{warning.message}
          </li>)}</ul>
          {status.warningCount > status.warnings.length && <p>Only the first {status.warnings.length} warnings are shown.</p>}
        </section>
      )}
      <section aria-labelledby="assets-heading" aria-busy={scanning || loading}>
        <h2 id="assets-heading">Audio library</h2>
        {!library && !loading && <p>Select an audio directory to begin.</p>}
        {library && <form className="search" onSubmit={(event) => { event.preventDefault(); void controller.search(search); }}>
          <label htmlFor="asset-search">Search filenames and paths</label>
          <input id="asset-search" type="search" maxLength={1024} value={search} onChange={(event) => setSearch(event.target.value)} aria-describedby="search-help" />
          <button type="submit">Search</button>
          <button type="button" onClick={() => { setSearch(""); void controller.search(""); }}>Clear</button>
          <small id="search-help">Literal substring matching; %, _ and \ are ordinary characters. ASCII case-insensitive; full Unicode case folding is unavailable.</small>
        </form>}
        <div role="status" aria-live="polite" aria-atomic="true">
          {loading && <p>Loading stored results…</p>}
          {library && !loading && !error && <p>{view.total} stored {view.total === 1 ? "asset" : "assets"}{view.search ? ` matching “${view.search}”` : ""}.</p>}
          {library && !loading && !error && assets.length === 0 && <p>{view.search ? "No assets match this search." : "No indexed audio files. Authorize the directory and start a scan."}</p>}
        </div>
        {assets.length > 0 && <>
          <p>Availability reflects the last observation; partial or failed scans preserve earlier states for unseen files. Extensions do not guarantee playable audio.{scanning ? " Previous results are shown while scanning." : ""}</p>
          <ul className="assets">{assets.map((asset) => <li key={asset.id}>
            <strong className="path">{asset.name}</strong>
            <span>{asset.extension.toUpperCase()} · {{ available: "Available", missing: "Missing", inaccessible: "Inaccessible", unknown: "Unknown" }[asset.availability]}</span>
            <small className="path">{asset.relativeDisplayPath}</small>
          </li>)}</ul>
        </>}
        {library && <nav className="pagination" aria-label="Asset pages">
          <button type="button" disabled={loading || view.offset === 0} onClick={() => { void controller.page(Math.max(0, view.offset - view.pageSize)); }}>Previous</button>
          <span>Page {Math.floor(view.offset / view.pageSize) + 1} of {Math.max(1, Math.ceil(view.total / view.pageSize))}</span>
          <button type="button" disabled={loading || view.offset + view.pageSize >= view.total || view.offset + view.pageSize > 1_000_000} onClick={() => { void controller.page(view.offset + view.pageSize); }}>Next</button>
        </nav>}
        <p className="help">Metadata follows a relative path: replacing a file at the same path keeps its metadata. Moved files create a new entry; unavailable records are retained.</p>
      </section>
    </main>
  );
}

export default App;
