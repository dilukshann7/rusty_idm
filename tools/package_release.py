"""Package already-built Rusty binaries; never launches the app or installer."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
import shutil
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
BROWSER_VERSION = VERSION.split("-", 1)[0]
PREFIX = f"RustyDownloadManager-{VERSION}"


def inspect_pe(path, allow_system_crt=False):
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 60)[0]
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0":
        raise SystemExit(f"Not a Windows executable: {path}")
    machine, sections = struct.unpack_from("<HH", data, pe + 4)
    optional = pe + 24
    if machine != 0x8664 or struct.unpack_from("<H", data, optional)[0] != 0x20B:
        raise SystemExit(f"Expected Windows x64 PE: {path}")
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    table = optional + optional_size

    def offset(rva):
        for index in range(sections):
            size, address, raw_size, raw = struct.unpack_from("<IIII", data, table + index * 40 + 8)
            if address <= rva < address + max(size, raw_size):
                return raw + rva - address
        raise SystemExit(f"Unmapped PE address in {path}")

    imports = []
    import_rva = struct.unpack_from("<I", data, optional + 120)[0]
    if import_rva:
        entry = offset(import_rva)
        while any(struct.unpack_from("<IIIII", data, entry)):
            name = offset(struct.unpack_from("<I", data, entry + 12)[0])
            imports.append(data[name:data.index(0, name)].decode("ascii"))
            entry += 20
    blocked = ("vcruntime", "msvcp") if allow_system_crt else ("vcruntime", "msvcp", "ucrtbase", "api-ms-win-crt")
    if any(name.lower().startswith(blocked) for name in imports):
        raise SystemExit(f"External Visual C++ runtime dependency: {path}")
    return {"machine": "x64", "imports": imports}


def copy_file(source, target):
    if not source.is_file():
        raise SystemExit(f"Required release file is missing: {source}")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def archive(source, target, prefix=""):
    # Cargo registry archives can preserve epoch timestamps before ZIP's 1980
    # minimum. Clamp metadata while preserving every file's exact contents.
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as output:
        for path in sorted(source.rglob("*")):
            if path.is_file():
                output.write(path, prefix + path.relative_to(source).as_posix())
    with zipfile.ZipFile(target) as result:
        names = result.namelist()
        if not names or len(names) != len(set(names)) or any(".." in Path(n).parts for n in names):
            raise SystemExit("Unsafe or empty package paths")


def write_package_readme(payload):
    # Keep distribution links relative to files that are actually shipped.
    channel = "Beta prerelease" if "-" in VERSION else "Release"
    (payload / "README.md").write_text(
        f"# Rusty Download Manager {VERSION}\n\n"
        f"**{channel} · Windows 10/11 x64 · Unsigned**\n\n"
        "![Rusty](branding/rusty-128.png)\n\n"
        "Run `bin/rusty-win32.exe` to open the manager. Close or minimize it to "
        "keep downloads running in the notification area. Use tray Exit to stop it.\n\n"
        "Read the [user guide](docs/USER_GUIDE.md) for browser capture, background "
        "startup and portable execution. See [release notes](docs/RELEASE_NOTES.md), "
        "[validation](docs/VALIDATION.md), "
        "[third-party notices](THIRD_PARTY.md) and [license](LICENSE).\n\n"
        "Windows 10/11 x64. Unsigned release. Unfinished jobs are held after restart. "
        "Browser capture needs each extension and website-access approval. Portable "
        "execution does not enable startup or register browser hosts automatically.\n",
        encoding="utf-8",
    )


def stage_documents(payload):
    """Keep distributed guides and their local document links together."""
    for source in sorted((ROOT / "docs").iterdir()):
        if source.is_file() and (source.suffix == ".md" or source.name == "SETUP.txt"):
            copy_file(source, payload / "docs" / source.name)


def stage(binary_dir):
    dist = ROOT / "dist"
    payload = dist / "payload"
    if payload.exists():
        raise SystemExit("Release payload already exists; preserve it and choose a fresh dist directory.")
    payload.mkdir(parents=True)
    for name in ["rusty-win32.exe", "rusty-native-host.exe"]:
        inspect_pe(binary_dir / name)
        copy_file(binary_dir / name, payload / "bin" / name)
    lock = json.loads((ROOT / "tools/media_tools_lock.json").read_text(encoding="utf-8"))
    source_tools = ROOT / "assets/media-tools/windows-x64"
    tool_manifest = json.loads((source_tools / "manifest.json").read_text(encoding="utf-8"))
    if tool_manifest.get("archive_sha256") != {entry["name"]: entry["archive_sha256"] for entry in lock["tools"]}:
        raise SystemExit("Media tool archive hashes do not match the pinned lock")
    for name in ["yt-dlp.exe", "ffmpeg.exe", "ffprobe.exe", "deno.exe"]:
        source = source_tools / name
        expected = next(entry["outputs"][name] for entry in lock["tools"] if name in entry["outputs"])
        if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest() != expected or tool_manifest["files"].get(name) != expected:
            raise SystemExit(f"Missing or changed media tool: {name}")
        inspect_pe(source, allow_system_crt=True)
        copy_file(source, payload / "tools" / name)
    copy_file(ROOT / "tools/media_tools_lock.json", payload / "tools" / "media_tools_lock.json")
    copy_file(source_tools / "manifest.json", payload / "tools" / "manifest.json")
    for notice in lock["notices"]:
        source = ROOT / "assets/media-tool-notices" / notice
        if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest() != lock["notices"][notice]:
            raise SystemExit(f"Missing or changed media notice: {notice}")
        copy_file(source, payload / "licenses" / "media-tools" / notice)
    for name in ["LICENSE", "README.md", "THIRD_PARTY.md"]:
        copy_file(ROOT / name, payload / name)
    write_package_readme(payload)
    stage_documents(payload)
    for name in ["register-chrome-host.ps1", "unregister-chrome-host.ps1", "chrome-integration.ps1", "release-integration.ps1", "browser-host.ps1"]:
        copy_file(ROOT / "scripts" / name, payload / "scripts" / name)
    for path in (ROOT / "assets/branding").glob("*"):
        copy_file(path, payload / "branding" / path.name)
    from package_browser_extensions import stage as stage_browsers
    stage_browsers(payload)
    cargo = Path.home() / ".cargo/bin/cargo.exe"
    metadata = json.loads(subprocess.check_output([str(cargo), "metadata", "--locked", "--offline", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc"], cwd=ROOT))
    inventory = []
    for package in metadata["packages"]:
        if package["id"] in metadata["workspace_members"]:
            continue
        if not package.get("license") and not package.get("license_file"):
            raise SystemExit(f"Missing license metadata for {package['name']}")
        source = Path(package["manifest_path"]).parent
        dest = payload / "licenses" / f"{package['name']}-{package['version']}"
        files = [p for p in source.iterdir() if p.is_file() and p.name.lower().startswith(("license", "licence", "copying", "copyright", "notice"))]
        if package.get("license_file"):
            files.append(source / package["license_file"])
        for file in set(files):
            copy_file(file, dest / file.name)
        inventory.append({"name": package["name"], "version": package["version"], "license": package.get("license"), "files": sorted({f.name for f in files})})
    (payload / "licenses/dependencies.json").write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")
    archive(payload, dist / f"{PREFIX}-windows-x64-portable.zip", f"{PREFIX}/")
    for browser in ("chrome", "brave", "edge", "firefox"):
        archive(payload / f"{browser}-extension", dist / f"{PREFIX}-{browser}-extension.zip")
    print(f"Staged {sum(p.is_file() for p in payload.rglob('*'))} files and five archives")


def finish(test_suites="not run"):
    dist = ROOT / "dist"
    names = [f"{PREFIX}-windows-x64-setup.exe", f"{PREFIX}-windows-x64-portable.zip"] + [
        f"{PREFIX}-{browser}-extension.zip"
        for browser in ("chrome", "brave", "edge", "firefox")
    ]
    artifacts = []
    for name in names:
        file = dist / name
        if not file.is_file():
            raise SystemExit(f"Missing artifact: {file}")
        artifacts.append({"name": name, "bytes": file.stat().st_size, "sha256": hashlib.sha256(file.read_bytes()).hexdigest()})
    manifest = {"product": "Rusty Download Manager", "version": VERSION, "platform": "windows-x64", "signed": False, "artifacts": artifacts, "verification": {"optimized_windows_build": "passed", "source_checks": "passed", "installer_compilation": "passed", "package_paths_versions_hashes": "passed", "test_suites": test_suites, "installer_install_uninstall": "not run", "live_tray_login_chrome": "not run"}}
    path = dist / "release-manifest.json"
    path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    checksums = artifacts + [{"name": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}]
    (dist / "SHA256SUMS.txt").write_text("".join(f"{item['sha256']}  {item['name']}\n" for item in checksums), encoding="ascii")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["stage", "finish"])
    parser.add_argument("--binary-dir", type=Path)
    parser.add_argument("--test-suites-status", choices=["not run", "passed"], default="not run")
    args = parser.parse_args()
    if args.mode == "stage":
        if not args.binary_dir:
            parser.error("stage requires --binary-dir")
        stage(args.binary_dir.resolve())
    else:
        finish(args.test_suites_status)
