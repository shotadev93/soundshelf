import { useEffect, useState, useSyncExternalStore } from "react";
import { LibraryController } from "./lib/library-controller";
import { libraryApi } from "./lib/ipc";
import "./App.css";

function App() {
  const [controller] = useState(() => new LibraryController(libraryApi));
  const view = useSyncExternalStore(controller.subscribe, controller.getSnapshot);
  useEffect(() => controller.cancelPending, [controller]);
  const { library, assets, status, selecting, scanning, error } = view;
  const finished = status?.phase === "complete" || status?.phase === "partial";

  return (
    <main className="container">
      <h1>SoundShelf</h1>
      <p>Discover local WAV, MP3, and OGG audio. Original files are read-only.</p>
      <div className="toolbar" aria-label="Library actions">
        <button type="button" disabled={selecting} onClick={() => { void controller.selectDirectory(); }}>
          {selecting ? "Selecting directory…" : "Select audio directory"}
        </button>
        <button type="button" disabled={!library || scanning || selecting} onClick={() => { void controller.scan(); }}>
          {scanning ? "Scanning…" : finished ? "Refresh scan" : "Start scan"}
        </button>
      </div>
      <p className="directory"><strong>Directory:</strong> {library?.displayPath ?? "No directory selected"}</p>
      <div role="status" aria-live="polite" aria-atomic="true">
        {scanning && <p>{status?.phase === "running"
          ? `Scanning: ${status.visitedEntries} entries checked, ${status.discoveredAssets} audio candidates found.`
          : "Loading scan status…"}</p>}
        {!scanning && finished && <p>{assets.length} audio candidates found.{status.phase === "partial" ? " Results are partial." : " Scan complete."}</p>}
        {!scanning && library && status?.phase === "idle" && <p>Start a scan to discover audio files.</p>}
      </div>
      {error && <p className="error" role="alert">{error.message}</p>}
      {status && status.warningCount > 0 && (
        <section aria-labelledby="warnings-heading" className="warnings">
          <h2 id="warnings-heading">Scan warnings ({status.warningCount})</h2>
          <ul>{status.warnings.map((warning, index) => <li key={index}>
            {warning.relativeDisplayPath && <span className="path">{warning.relativeDisplayPath}: </span>}{warning.message}
          </li>)}</ul>
          {status.warningCount > status.warnings.length && <p>Only the first {status.warnings.length} warnings are shown.</p>}
        </section>
      )}
      <section aria-labelledby="assets-heading" aria-busy={scanning}>
        <h2 id="assets-heading">Audio library</h2>
        {!library && <p>Select an audio directory to begin.</p>}
        {finished && !scanning && assets.length === 0 && <p>No supported audio files were discovered.</p>}
        {assets.length > 0 && <>
          <p>Availability reflects the last scan. Extensions do not guarantee playable audio.{scanning ? " Previous results are shown while scanning." : ""}</p>
          <ul className="assets">{assets.map((asset) => <li key={asset.id}>
            <strong className="path">{asset.name}</strong>
            <span>{asset.extension.toUpperCase()} · {asset.availability === "available" ? "Available" : "Inaccessible"}</span>
            <small className="path">{asset.relativeDisplayPath}</small>
          </li>)}</ul>
        </>}
      </section>
    </main>
  );
}

export default App;
