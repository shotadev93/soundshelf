import { useState } from "react";
import type { LibraryController, LibraryView } from "../lib/library-controller";

export function TagEditor({ controller, view }: { controller: LibraryController; view: LibraryView }) {
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const busy = view.metadataBusy || view.tagsLoading;
  return <section className="tag-editor" aria-labelledby="tags-heading" aria-busy={busy}>
    <h2 id="tags-heading">Tags</h2>
    <p id="tag-help">Names use 1–64 characters, trimmed, without control characters. Names differing only by Unicode lowercase mapping are duplicates. Composed and decomposed characters remain distinct. Deleting a tag removes it from every asset.</p>
    <form onSubmit={(event) => {
      event.preventDefault();
      void (editing ? controller.renameTag(editing, name) : controller.createTag(name));
    }}>
      <label htmlFor="tag-name">{editing ? "Rename tag" : "New tag name"}</label>
      <input id="tag-name" required value={name} disabled={busy} aria-describedby="tag-help" onChange={(event) => setName(event.target.value)} />
      <button disabled={busy} type="submit">{editing ? "Save tag name" : "Create tag"}</button>
      {editing && <button type="button" disabled={busy} onClick={() => { setEditing(null); setName(""); }}>Cancel rename</button>}
    </form>
    {view.tagsLoading && <p role="status">Loading tags…</p>}
    {!view.tagsLoading && view.tags.length === 0 && <p>No tags yet. Create a tag to annotate assets.</p>}
    <ul>{view.tags.map((tag) => <li key={tag.id}>
      <span>{tag.name}</span>{" "}
      <button type="button" disabled={busy} aria-label={`Rename tag ${tag.name}`} onClick={() => { setEditing(tag.id); setName(tag.name); }}>Rename</button>{" "}
      <button type="button" disabled={busy} aria-label={`Delete tag ${tag.name} from all assets`} onClick={() => { setEditing(null); void controller.deleteTag(tag.id); }}>Delete</button>
    </li>)}</ul>
    <p role="status" aria-live="polite" aria-atomic="true">{view.metadataStatus}</p>
    {view.metadataError && <div className="error" role="alert"><p>{view.metadataError.message}</p>
      <button type="button" disabled={busy} onClick={() => { void controller.loadTags(); }}>Reload tags</button>
      <p>For a failed edit, correct the input if needed and use the same edit control to retry.</p>
    </div>}
  </section>;
}
