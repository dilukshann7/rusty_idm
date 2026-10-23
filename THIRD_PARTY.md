# Third-party software

Rusty Download Manager uses Rust dependencies pinned in Cargo.lock, including
Tokio, Reqwest, rustls, ring, serde, URL and the Microsoft windows bindings.
The release package contains an inventory of the resolved Windows dependency
versions and license expressions in `licenses/dependencies.json`, plus available
upstream license/notice files copied from those exact local Cargo packages.
Workspace source is covered by the included MIT LICENSE.

Site Grabber also uses pinned lol_html 3.0.1, cssparser 0.38.0, encoding_rs 0.8.42
and robotstxt 0.3.0 for HTML/CSS parsing, web encodings and robots-rule matching.
Their transitive dependencies and upstream notices use the same Cargo package
inventory mechanism.

Inno Setup creates the installer. Its installer-engine notices are retained in
the generated setup executable. Inno Setup is not needed to run the app.
Windows, Chrome and their bundled system components retain their own licenses.
The installer and portable archive also include pinned yt-dlp, Deno, FFmpeg and
ffprobe executables. Their exact release URLs and archive checksums are in
`tools/media_tools_lock.json`; extracted tool hashes are in the packaged
`tools/manifest.json` (generated from `assets/media-tools/windows-x64/manifest.json`).
Their upstream license and build notices are under `licenses/media-tools/`.
The official Windows yt-dlp executable includes components covered by its
`yt-dlp-THIRD_PARTY_LICENSES.txt`; its combined work and the selected FFmpeg build
are GPL-enabled. Source and redistribution terms are described in those upstream
notices and the pinned upstream release repositories.
The application and extension branding is original artwork from this project.

Rebuild and source: Cargo manifests/lock and the scripts in this repository are
the source of truth. The package generator stops if a dependency has neither a
license expression nor a license file. Review applicable upstream license terms
before redistributing a modified release.
