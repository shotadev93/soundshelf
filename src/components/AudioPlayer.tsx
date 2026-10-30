import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { preparePlayback } from "../lib/ipc";
import { emptyPlayback, formatTime, PlaybackController } from "../lib/playback-controller";
import type { PlaybackSelection } from "../lib/playback-controller";

const noopSubscribe = () => () => {};
const emptySnapshot = () => emptyPlayback;

export function AudioPlayer({ libraryId, active, selection }: { libraryId: string | undefined; active: boolean; selection: PlaybackSelection | null }) {
  const audio = useRef<HTMLAudioElement>(null);
  const [controller, setController] = useState<PlaybackController | null>(null);
  useEffect(() => {
    if (!audio.current) return;
    const player = new PlaybackController(audio.current, preparePlayback);
    setController(player);
    return player.clear;
  }, []);
  const view = useSyncExternalStore(controller?.subscribe ?? noopSubscribe, controller?.getSnapshot ?? emptySnapshot);
  useEffect(() => {
    if (active && selection && selection.libraryId === libraryId) void controller?.select(selection);
    else controller?.clear();
  }, [controller, libraryId, active, selection]);
  const ready = view.phase === "playing" || view.phase === "paused";
  return <section className="audio-player" aria-labelledby="player-heading">
    <h2 id="player-heading">Audio player</h2>
    <audio ref={audio} preload="metadata" />
    <p className="path">{view.selection?.name ?? "Select an asset to play."}</p>
    <div className="toolbar">
      <button type="button" disabled={!ready} onClick={() => { void controller?.toggle(); }}>{view.phase === "playing" ? "Pause" : "Play"}</button>
      <button type="button" disabled={!view.selection} onClick={() => controller?.clear()}>Stop</button>
    </div>
    <label htmlFor="playback-seek">Playback position</label>
    <input id="playback-seek" type="range" min="0" max={view.duration || 0} step="0.1" value={Math.min(view.currentTime, view.duration)} disabled={!ready || !view.duration}
      aria-valuetext={`${formatTime(view.currentTime)} of ${formatTime(view.duration)}`} onChange={(event) => controller?.seek(Number(event.target.value))} />
    <p><span aria-label="Current time">{formatTime(view.currentTime)}</span> / <span aria-label="Total duration">{formatTime(view.duration)}</span></p>
    <p role="status" aria-live="polite">{view.phase === "loading" ? "Loading audio…" : view.phase === "playing" ? "Playing" : view.phase === "paused" ? "Paused" : ""}</p>
    {view.message && <div className="error" role="alert"><p>{view.message}</p>
      <button type="button" disabled={!active} onClick={() => { void controller?.retry(); }}>Retry playback</button>
    </div>}
  </section>;
}
