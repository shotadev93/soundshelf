#!/usr/bin/env python3
"""Collect cached, version-specific upstream notices; no network or media access.

Requires locked Cargo dependencies, installed npm dependencies and Rust toolchain
documentation. Reports missing texts rather than inventing notices or relicensing.
"""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def output(args, cwd=ROOT):
    return subprocess.check_output(args, cwd=cwd, text=True)


def notice_files(root):
    return sorted(p for p in root.rglob("*") if p.is_file()
                  and re.match(r"(licen[cs]e|copying|copyright|notice|unlicense)", p.name, re.I))


def normalized(text):
    # Preserve legal words/attribution; normalize only line endings/trailing space.
    return "\n".join(line.rstrip() for line in text.splitlines()) + "\n"


def main():
    metadata = json.loads(output([
        "cargo", "metadata", "--offline", "--locked", "--format-version", "1",
        "--filter-platform", "x86_64-unknown-linux-gnu",
    ], ROOT / "src-tauri"))
    tree = output(["cargo", "tree", "--offline", "--locked", "--target",
                   "x86_64-unknown-linux-gnu", "-e", "normal,build", "--prefix", "none"], ROOT / "src-tauri")
    included = set(re.findall(r"^([\w-]+) v([^\s]+)", tree, re.M))
    packages = sorted(metadata["packages"], key=lambda p: (p["name"], p["version"]))
    by_name = {p["name"]: Path(p["manifest_path"]).parent for p in packages}
    chunks = ["SoundShelf third-party notices\n=============================\n\n"
              "These upstream works retain their own licenses, not SoundShelf's MIT license.\n"
              "This conservative inventory covers Linux normal/build Cargo dependencies and\n"
              "installed npm build/runtime dependencies; inclusion does not mean all are linked.\n"
              "MPL-covered source is available in the exact upstream source archives listed\n"
              "below. No modifications to those upstream sources were made by SoundShelf.\n"
              "See docs/THIRD_PARTY_LICENSE_REVIEW.md for scope, runtime libraries and assets.\n"]
    records = []
    missing = []
    lock_crates = {(p["name"], p["version"]): p.get("checksum") for p in
                   tomllib.loads((ROOT / "src-tauri/Cargo.lock").read_text())["package"]}
    mpl_sources = []

    def record(ecosystem, name, version, license_name, source, root, files, note=""):
        chunks.append(f"\n{'=' * 72}\n{ecosystem}: {name} {version}\n"
                      f"Declared license: {license_name}\nSource: {source}\n{note}\n")
        entries = []
        if not files:
            missing.append(f"{ecosystem}:{name}@{version}")
        for label, path in files:
            data = path.read_bytes()
            chunks.append(f"\n--- {label} ---\n{data.decode('utf-8', errors='replace')}\n")
            entries.append({"file": label, "sha256": hashlib.sha256(data).hexdigest()})
        records.append({"ecosystem": ecosystem, "name": name, "version": version,
                        "license": license_name, "source": source, "notices": entries, "note": note})

    for package in packages:
        name, version = package["name"], package["version"]
        if name == "soundshelf" or (name, version) not in included:
            continue
        root = Path(package["manifest_path"]).parent
        files = [(str(p.relative_to(root)), p) for p in notice_files(root)]
        note = ""
        if name == "alloc-stdlib":
            files = [("repository-root BSD notice (see supplemental provenance)", by_name["alloc-no-stdlib"] / "LICENSE")]
        elif name in {"dlopen2", "dlopen2_derive"}:
            files = [("repository-root LICENSE at published commit", ROOT / "licenses/supplemental/dlopen2-LICENSE")]
        elif name == "selectors":
            files = [("MPL-2.0 text (same standard text as cssparser LICENSE)", by_name["cssparser"] / "LICENSE")]
            note = "Source header: " + (root / "lib.rs").read_text().split("*/", 1)[0] + "*/"
        if package["license"] == "MPL-2.0":
            archive = root.parents[2] / "cache" / root.parent.name / f"{name}-{version}.crate"
            checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
            assert checksum == lock_crates[(name, version)], f"MPL source checksum mismatch: {name}"
            destination = ROOT / "licenses/sources" / archive.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(archive, destination)
            mpl_sources.append({"file": str(destination.relative_to(ROOT)), "sha256": checksum})
            note += f"\nUnmodified source is also supplied locally in licenses/sources/{archive.name}.\n"
        record("Cargo Linux normal/build", name, version, package["license"],
               f"https://crates.io/api/v1/crates/{name}/{version}/download", root, files, note)

    lock = json.loads((ROOT / "package-lock.json").read_text())
    for path, locked in sorted(lock["packages"].items()):
        if not path or not (ROOT / path / "package.json").is_file():
            continue
        root = ROOT / path
        package = json.loads((root / "package.json").read_text())
        assert package["version"] == locked["version"], f"npm lock mismatch: {path}"
        files = [(str(p.relative_to(root)), p) for p in notice_files(root)]
        if package["name"] == "@tauri-apps/cli-linux-x64-gnu" and not files:
            parent = ROOT / "node_modules/@tauri-apps/cli"
            files = [("parent CLI/" + str(p.relative_to(parent)), p) for p in notice_files(parent)]
        elif package["name"] == "@rolldown/binding-linux-x64-gnu" and not files:
            parent = ROOT / "node_modules/rolldown"
            files = [("parent Rolldown/" + str(p.relative_to(parent)), p) for p in notice_files(parent)]
        record("npm installed (runtime or build tool)", package["name"], package["version"],
               package.get("license"), locked.get("resolved", package.get("homepage", "")), root, files)

    sqlite = (by_name["libsqlite3-sys"] / "sqlite3/sqlite3.c").read_text()
    start = sqlite.index("** 2001 September 15")
    chunks.append("\nSQLite amalgamation 3.54.1 (bundled through libsqlite3-sys)\n"
                  "Source: https://sqlite.org/copyright.html\n/*\n" + sqlite[start:sqlite.index("*/", start) + 2] + "\n")
    sysroot = Path(output(["rustc", "--print", "sysroot"]).strip())
    rust_docs = sysroot / "share/doc/rust"
    destination = ROOT / "licenses/rust-standard-library"
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(rust_docs / "COPYRIGHT-library.html", destination / "COPYRIGHT-library.html")
    shutil.copytree(rust_docs / "licenses", destination / "licenses", dirs_exist_ok=True)
    for path in destination.rglob("*"):
        if path.is_file():
            path.write_text(normalized(path.read_text()))
    chunks.append("\nRust standard library and compiler-builtins notices: see\n"
                  "licenses/rust-standard-library/COPYRIGHT-library.html and its licenses directory.\n"
                  "Toolchain: " + output(["rustc", "--version"]).strip() + "\n")
    (ROOT / "THIRD_PARTY_NOTICES.txt").write_text(normalized("".join(chunks)))
    inventory = {"scope": "Linux normal/build Cargo closure and installed npm packages; conservative, not a linker map",
                 "cargoLockSha256": hashlib.sha256((ROOT / "src-tauri/Cargo.lock").read_bytes()).hexdigest(),
                 "npmLockSha256": hashlib.sha256((ROOT / "package-lock.json").read_bytes()).hexdigest(),
                 "packages": records, "mplSourceArchives": mpl_sources, "missingNoticeTexts": missing}
    all_metadata = json.loads(output(["cargo", "metadata", "--offline", "--locked", "--format-version", "1"], ROOT / "src-tauri"))
    inventory["cargoOutsideLinuxNormalBuildScope"] = [
        {"name": p["name"], "version": p["version"], "license": p["license"]}
        for p in sorted(all_metadata["packages"], key=lambda p: (p["name"], p["version"]))
        if p["name"] != "soundshelf" and (p["name"], p["version"]) not in included
    ]
    inventory["npmNotInstalledOnThisHost"] = [
        {"packagePath": p, "version": entry.get("version"), "license": entry.get("license")}
        for p, entry in sorted(lock["packages"].items()) if p and not (ROOT / p / "package.json").exists()
    ]
    (ROOT / "licenses/inventory.json").write_text(json.dumps(inventory, indent=2, ensure_ascii=False) + "\n")
    print(f"Collected {len(records)} package records; missing notice texts: {missing}")
    if missing:
        raise SystemExit("Missing upstream notice text requires review")


if __name__ == "__main__":
    main()
