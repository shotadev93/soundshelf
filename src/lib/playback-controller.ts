import { toAppError } from "./contracts.ts";
import type { PlaybackRequest, PlaybackSource } from "./types.ts";

export interface PlaybackSelection extends PlaybackRequest { name: string }
export interface PlaybackView {
  selection: PlaybackSelection | null;
  phase: "idle" | "loading" | "playing" | "paused" | "error";
  currentTime: number;
  duration: number;
  message: string | null;
}
export const emptyPlayback: PlaybackView = { selection: null, phase: "idle", currentTime: 0, duration: 0, message: null };
type Media = Pick<HTMLAudioElement, "error" | "src" | "currentTime" | "duration" | "paused" | "ended" | "play" | "pause" | "load" | "removeAttribute" | "addEventListener" | "removeEventListener">;

export class PlaybackController {
  private view: PlaybackView = emptyPlayback;
  private listeners = new Set<() => void>();
  private generation = 0;
  private removeEvents: () => void = () => {};
  private media: Media;
  private prepare: (request: PlaybackRequest) => Promise<PlaybackSource>;
  constructor(media: Media, prepare: (request: PlaybackRequest) => Promise<PlaybackSource>) {
    this.media = media;
    this.prepare = prepare;
  }
  getSnapshot = (): PlaybackView => this.view;
  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  private update(patch: Partial<PlaybackView>) {
    this.view = { ...this.view, ...patch };
    this.listeners.forEach((listener) => listener());
  }
  clear = () => {
    ++this.generation;
    this.removeEvents();
    this.media.pause();
    this.media.removeAttribute("src");
    this.media.load();
    this.view = emptyPlayback;
    this.listeners.forEach((listener) => listener());
  };
  async select(selection: PlaybackSelection) {
    this.clear();
    const generation = this.generation;
    this.update({ selection, phase: "loading" });
    const current = () => generation === this.generation;
    const fail = (message: string) => {
      if (!current()) return;
      this.removeEvents();
      this.media.pause();
      this.media.removeAttribute("src");
      this.media.load();
      this.update({ phase: "error", message });
    };
    let prepared = false;
    try {
      const source = await this.prepare({ libraryId: selection.libraryId, assetId: selection.assetId });
      if (!current()) return;
      prepared = true;
      const timing = () => {
        if (!current()) return;
        this.update({ currentTime: Number.isFinite(this.media.currentTime) ? this.media.currentTime : 0,
          duration: Number.isFinite(this.media.duration) && this.media.duration > 0 ? this.media.duration : 0 });
      };
      const events: [string, EventListener][] = [
        ["loadedmetadata", timing], ["durationchange", timing], ["timeupdate", timing],
        ["playing", () => { if (current()) this.update({ phase: "playing", message: null }); }],
        ["pause", () => { if (current() && this.view.phase !== "error") this.update({ phase: "paused" }); }],
        ["ended", () => { if (current()) this.update({ phase: "paused" }); }],
        ["waiting", () => { if (current() && !this.media.paused) this.update({ phase: "loading" }); }],
        ["error", () => fail(this.mediaError())],
      ];
      events.forEach(([name, handler]) => this.media.addEventListener(name, handler));
      this.removeEvents = () => {
        events.forEach(([name, handler]) => this.media.removeEventListener(name, handler));
        this.removeEvents = () => {};
      };
      this.media.src = source.url;
      this.media.load();
      await this.media.play();
      if (current() && this.view.phase !== "error") this.update({ phase: this.media.paused ? "paused" : "playing" });
    } catch (error) {
      // An error event can reject play() too; keep the original media diagnosis.
      if (!current() || this.view.phase === "error") return;
      if (current() && this.media.src && typeof error === "object" && error !== null && "name" in error && error.name === "NotAllowedError") {
        this.update({ phase: "paused", message: "Audio is ready. Press Play to start playback." });
      } else {
        fail(prepared ? this.mediaError(error) : toAppError(error).message);
      }
    }
  }
  private mediaError(error?: unknown): string {
    const unsupported = typeof error === "object" && error !== null && "name" in error && error.name === "NotSupportedError";
    switch (this.media.error?.code ?? (unsupported ? 4 : 0)) {
      case 2: return "Audio could not be loaded. Retry playback; select the directory again if authorization is inactive.";
      case 3: return "Audio could not be decoded. The file may be corrupt or its codec may be unsupported.";
      case 4: return "The audio source or codec is unsupported by this system. Try a supported WAV, MP3, or OGG file.";
      default: return "Audio could not be played. The file may be unavailable or its codec may be unsupported. Retry playback.";
    }
  }
  async toggle() {
    if (this.view.phase === "playing") {
      this.media.pause();
      this.update({ phase: "paused" });
    } else if (this.view.phase === "paused") {
      const generation = this.generation;
      this.update({ phase: "loading", message: null });
      try {
        if (this.media.ended) this.media.currentTime = 0;
        await this.media.play();
        if (generation === this.generation && this.getSnapshot().phase !== "error") this.update({ phase: this.media.paused ? "paused" : "playing" });
      } catch (error) {
        if (generation === this.generation && this.getSnapshot().phase !== "error") this.update({ phase: "error", message: this.mediaError(error) });
      }
    }
  }
  seek(seconds: number) {
    if (!Number.isFinite(seconds) || !this.view.duration || this.view.phase === "error" || this.view.phase === "loading") return;
    try {
      this.media.currentTime = Math.max(0, Math.min(seconds, this.view.duration));
      this.update({ currentTime: this.media.currentTime });
    } catch {
      this.update({ message: "Seeking is unavailable. Retry playback." });
    }
  }
  retry = async () => { if (this.view.selection) await this.select(this.view.selection); };
}

export function formatTime(seconds: number): string {
  const value = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  return `${Math.floor(value / 60)}:${String(value % 60).padStart(2, "0")}`;
}
