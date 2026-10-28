import { toAppError } from "./contracts.ts";
import type { LibraryApi } from "./ipc.ts";
import type { AppError, AudioAsset, LibraryInfo, ScanStatus } from "./types.ts";

export interface LibraryView {
  library: LibraryInfo | null;
  assets: AudioAsset[];
  status: ScanStatus | null;
  selecting: boolean;
  scanning: boolean;
  error: AppError | null;
}

// Own the concrete directory/scan workflow independently of React rendering so
// cancellation and out-of-order native responses can be exercised without a GUI.
export class LibraryController {
  private api: LibraryApi;
  private interval: number;
  private version = 0;
  private selectionVersion = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private listeners = new Set<() => void>();
  private view: LibraryView = {
    library: null, assets: [], status: null, selecting: false, scanning: false, error: null,
  };

  constructor(api: LibraryApi, pollInterval = 350) {
    this.api = api;
    this.interval = pollInterval;
  }

  getSnapshot = (): LibraryView => this.view;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  private update(update: Partial<LibraryView>) {
    this.view = { ...this.view, ...update };
    for (const listener of this.listeners) listener();
  }

  cancelPending = () => {
    this.version += 1;
    this.selectionVersion += 1;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  };

  async selectDirectory() {
    if (this.view.selecting) return;
    const selection = ++this.selectionVersion;
    this.update({ selecting: true });
    try {
      const library = await this.api.selectDirectory();
      if (selection !== this.selectionVersion) return;
      if (library === null || library.id === this.view.library?.id) {
        this.update({ selecting: false });
        return;
      }
      this.version += 1;
      if (this.timer !== null) clearTimeout(this.timer);
      this.update({ library, assets: [], status: null, selecting: false, scanning: true, error: null });
      await this.poll(this.version, library.id, null);
    } catch (error) {
      if (selection === this.selectionVersion) this.update({ selecting: false, error: toAppError(error) });
    }
  }

  async scan() {
    const library = this.view.library;
    if (!library || this.view.scanning || this.view.selecting) return;
    const version = ++this.version;
    this.update({ scanning: true, error: null });
    try {
      const status = await this.api.startScan(library.id);
      if (version !== this.version) return;
      this.update({ status });
      await this.poll(version, library.id, status.scanId);
    } catch (error) {
      if (version !== this.version) return;
      if (toAppError(error).code === "busy") {
        await this.poll(version, library.id, null);
      } else {
        this.failed(error);
      }
    }
  }

  private failed(error: unknown) {
    const failure = toAppError(error);
    this.update(failure.code === "notAuthorized"
      ? { library: null, assets: [], status: null, scanning: false, error: failure }
      : { scanning: false, error: failure });
  }

  private async poll(version: number, libraryId: string, scanId: string | null) {
    try {
      const status = await this.api.getStatus(libraryId);
      if (version !== this.version) return;
      if (scanId !== null && status.scanId !== scanId) {
        throw { code: "unavailable", message: "The scan changed. Refresh to load its latest results." };
      }
      this.update({ status });
      if (status.phase === "running") {
        this.timer = setTimeout(() => { void this.poll(version, libraryId, status.scanId); }, this.interval);
      } else if (status.phase === "failed") {
        this.failed(status.error ?? { code: "unavailable", message: "The scan failed. Try selecting the directory again." });
      } else if (status.phase === "idle") {
        this.update({ scanning: false });
      } else {
        const result = await this.api.listAssets(libraryId);
        if (version !== this.version) return;
        if (result.scanId !== status.scanId) {
          throw { code: "unavailable", message: "The results belong to a different scan. Refresh the library." };
        }
        this.update({ assets: result.assets, scanning: false, error: null });
      }
    } catch (error) {
      if (version === this.version) this.failed(error);
    }
  }
}
