#!/usr/bin/env python3
"""Create synthetic audio in a NEW temporary directory for manual GUI acceptance.

Requires gst-launch-1.0 with audiotestsrc, wavenc, lamemp3enc and vorbisenc.
No personal audio is read or changed. These fixtures contain generated tones.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    launcher = shutil.which("gst-launch-1.0")
    if launcher is None:
        raise SystemExit("gst-launch-1.0 is required to generate the optional GUI fixtures.")
    fixture = Path(tempfile.mkdtemp(prefix="soundshelf-playback-"))
    library = fixture / "audio"
    nested = library / "nested"
    nested.mkdir(parents=True)
    pipelines = [
        (library / "tone.wav", ["wavenc"]),
        (nested / "tone.MP3", ["lamemp3enc"]),
        (nested / "tone.ogg", ["vorbisenc", "!", "oggmux"]),
    ]
    for destination, encoder in pipelines:
        # About 20 seconds, quiet sine wave. Each command writes only a new file.
        subprocess.run([
            launcher, "-q", "audiotestsrc", "num-buffers=860", "wave=sine", "volume=0.05",
            "!", "audioconvert", "!", "audioresample", "!", *encoder,
            "!", "filesink", f"location={destination}",
        ], check=True, timeout=30)
    (library / "corrupt.wav").write_bytes(b"SoundShelf synthetic invalid audio")
    (library / "empty.mp3").touch()
    outside = fixture / "outside.wav"
    shutil.copyfile(library / "tone.wav", outside)
    (library / "outside-link.wav").symlink_to(outside)
    manifest = {}
    for path in library.rglob("*"):
        if path.is_file() and not path.is_symlink():
            stat = path.stat()
            manifest[str(path.relative_to(library))] = {
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                "mode": stat.st_mode, "mtimeNs": stat.st_mtime_ns,
            }
    (fixture / "integrity-before.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Select this directory through the native picker: {library}")
    print(f"Integrity baseline (outside the library): {fixture / 'integrity-before.json'}")
    print("The tone files are synthetic; keep speaker volume low for the manual test.")


if __name__ == "__main__":
    main()
