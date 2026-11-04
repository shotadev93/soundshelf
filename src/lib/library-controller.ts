import { toAppError } from "./contracts.ts";
import type { LibraryApi } from "./ipc.ts";
import type { AppError, AudioAsset, LibraryInfo, SavedLibrary, ScanStatus, Tag } from "./types.ts";

const PAGE_SIZE = 50;

export interface LibraryView {
  library: LibraryInfo | null;
  libraries: SavedLibrary[];
  active: boolean;
  assets: AudioAsset[];
  status: ScanStatus | null;
  selecting: boolean;
  scanning: boolean;
  loading: boolean;
  search: string;
  tagIds: string[];
  favoritesOnly: boolean;
  tags: Tag[];
  tagsLoading: boolean;
  metadataBusy: boolean;
  metadataError: AppError | null;
  metadataStatus: string;
  offset: number;
  total: number;
  pageSize: number;
  error: AppError | null;
}

// Navigation, query, and picker generations independently reject stale responses.
export class LibraryController {
  private api: LibraryApi;
  private interval: number;
  private version = 0;
  private queryVersion = 0;
  private tagsVersion = 0;
  private metadataVersion = 0;
  private selectionVersion = 0;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private listeners = new Set<() => void>();
  private view: LibraryView = {
    library: null, libraries: [], active: false, assets: [], status: null,
    selecting: false, scanning: false, loading: false, search: "", offset: 0,
    tagIds: [], favoritesOnly: false, tags: [], tagsLoading: false, metadataBusy: false, metadataError: null, metadataStatus: "",
    total: 0, pageSize: PAGE_SIZE, error: null,
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
    this.queryVersion += 1;
    this.selectionVersion += 1;
    this.tagsVersion += 1;
    this.metadataVersion += 1;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  };

  async initialize() {
    void this.loadTags();
    const version = this.version;
    this.update({ loading: true, error: null });
    try {
      const libraries = await this.api.listLibraries();
      if (version !== this.version) return;
      this.update({ libraries, loading: false });
      if (libraries[0]) await this.openLibrary(libraries[0]);
    } catch (error) {
      if (version === this.version) this.update({ loading: false, error: toAppError(error) });
    }
  }

  async openLibrary(saved: SavedLibrary) {
    this.version += 1;
    this.queryVersion += 1;
    if (this.timer !== null) clearTimeout(this.timer);
    this.update({ library: saved.library, active: saved.active, assets: [], status: null,
      search: "", tagIds: [], favoritesOnly: false, metadataStatus: "", offset: 0, total: 0, scanning: saved.active, error: null });
    if (saved.active) await this.poll(this.version, saved.library.id, null);
    else await this.load();
  }

  async selectDirectory() {
    if (this.view.selecting) return;
    const selection = ++this.selectionVersion;
    this.update({ selecting: true });
    try {
      const library = await this.api.selectDirectory();
      if (selection !== this.selectionVersion) return;
      this.update({ selecting: false });
      if (library === null) return;
      const libraries = this.view.libraries.filter((entry) => entry.library.id !== library.id);
      const saved = { library, active: true };
      this.update({ libraries: [...libraries, saved] });
      if (library.id === this.view.library?.id && this.view.active) return;
      await this.openLibrary(saved);
    } catch (error) {
      if (selection === this.selectionVersion) this.update({ selecting: false, error: toAppError(error) });
    }
  }

  async search(text: string) {
    this.update({ search: text, offset: 0, assets: [], total: 0 });
    await this.load();
  }

  async filter(tagIds: string[], favoritesOnly: boolean) {
    this.update({ tagIds, favoritesOnly, offset: 0, assets: [], total: 0 });
    await this.load();
  }

  async loadTags() {
    const generation = ++this.tagsVersion;
    this.update({ tagsLoading: true, metadataError: null });
    try {
      const tags = await this.api.listTags();
      if (generation !== this.tagsVersion) return;
      this.update({ tags, tagsLoading: false });
    } catch (error) {
      if (generation === this.tagsVersion) this.update({ tagsLoading: false, metadataError: toAppError(error) });
    }
  }

  private async mutate(operation: () => Promise<unknown>, message: string, tagsChanged = false, libraryId?: string, deletedTagId?: string) {
    if (this.view.metadataBusy) return;
    const generation = ++this.metadataVersion;
    this.queryVersion += 1;
    this.update({ metadataBusy: true, metadataError: null, metadataStatus: "Saving metadata…", loading: false });
    try {
      await operation();
      if (generation !== this.metadataVersion) return;
      if (deletedTagId) this.update({ tagIds: this.view.tagIds.filter((id) => id !== deletedTagId), offset: 0 });
      if (tagsChanged) await this.loadTags();
      if (generation !== this.metadataVersion) return;
      this.update({ metadataStatus: !libraryId || libraryId === this.view.library?.id ? message : "" });
      await this.load();
    } catch (error) {
      if (generation === this.metadataVersion && (!libraryId || libraryId === this.view.library?.id)) {
        this.update({ metadataError: toAppError(error), metadataStatus: "" });
      }
    } finally {
      if (generation === this.metadataVersion) this.update({ metadataBusy: false });
    }
  }

  createTag(name: string) {
    return this.mutate(() => this.api.createTag({ name }), "Tag created.", true);
  }
  renameTag(tagId: string, name: string) {
    return this.mutate(() => this.api.renameTag({ tagId, name }), "Tag renamed.", true);
  }
  deleteTag(tagId: string) {
    return this.mutate(() => this.api.deleteTag({ tagId }), "Tag deleted from all assets.", true, undefined, tagId);
  }
  setTags(assetId: string, tagIds: string[]) {
    const libraryId = this.view.library?.id;
    if (!libraryId) return Promise.resolve();
    return this.mutate(() => this.api.setAssetTags({ libraryId, assetId, tagIds }), "Asset tags saved.", false, libraryId);
  }
  setFavorite(assetId: string, favorite: boolean) {
    const libraryId = this.view.library?.id;
    if (!libraryId) return Promise.resolve();
    return this.mutate(() => this.api.setAssetFavorite({ libraryId, assetId, favorite }), favorite ? "Added to favorites." : "Removed from favorites.", false, libraryId);
  }

  async page(offset: number) {
    if (offset < 0 || offset > 1_000_000 || !Number.isSafeInteger(offset)) return;
    this.update({ offset, assets: [] });
    await this.load();
  }

  async reload() { await this.load(); }

  private async load(expectedScanId?: string | null) {
    const library = this.view.library;
    if (!library) return;
    const query = ++this.queryVersion;
    const version = this.version;
    const request = { libraryId: library.id, search: this.view.search, tagIds: this.view.tagIds, favoritesOnly: this.view.favoritesOnly, offset: this.view.offset, limit: PAGE_SIZE };
    this.update({ loading: true, error: null });
    try {
      const result = await this.api.listAssets(request);
      if (version !== this.version || query !== this.queryVersion) return;
      if (result.offset !== request.offset || result.limit !== request.limit) {
        throw { code: "unavailable", message: "The results belong to a different page. Retry loading." };
      }
      if (expectedScanId !== undefined && result.scanId !== expectedScanId) {
        throw { code: "unavailable", message: "The results belong to a different scan. Refresh the library." };
      }
      // A rescan may shrink the final page; return to a valid page once.
      if (result.assets.length === 0 && request.offset > 0 && result.total <= request.offset) {
        this.update({ offset: result.total === 0 ? 0 : Math.floor((result.total - 1) / PAGE_SIZE) * PAGE_SIZE });
        await this.load();
        return;
      }
      this.update({ assets: result.assets, total: result.total, loading: false });
    } catch (error) {
      if (version === this.version && query === this.queryVersion) this.update({ loading: false, error: toAppError(error) });
    }
  }

  async scan() {
    const library = this.view.library;
    if (!library || !this.view.active || this.view.scanning || this.view.selecting) return;
    const version = ++this.version;
    this.queryVersion += 1;
    this.update({ scanning: true, loading: false, error: null });
    try {
      const status = await this.api.startScan(library.id);
      if (version !== this.version) return;
      this.update({ status });
      await this.poll(version, library.id, status.scanId);
    } catch (error) {
      if (version !== this.version) return;
      if (toAppError(error).code === "busy") await this.poll(version, library.id, null);
      else this.failed(error);
    }
  }

  private failed(error: unknown) {
    const failure = toAppError(error);
    if (failure.code === "notAuthorized") {
      const id = this.view.library?.id;
      this.update({ active: false, scanning: false, error: failure,
        libraries: this.view.libraries.map((entry) => entry.library.id === id ? { ...entry, active: false } : entry) });
    } else this.update({ scanning: false, error: failure });
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
      } else {
        this.update({ scanning: false });
        await this.load(status.phase === "idle" ? undefined : status.scanId);
      }
    } catch (error) {
      if (version === this.version) this.failed(error);
    }
  }
}
