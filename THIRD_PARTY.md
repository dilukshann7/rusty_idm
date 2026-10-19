# Third-party software

Rusty Download Manager uses Rust dependencies pinned in Cargo.lock, including
Tokio, Reqwest, rustls, ring, serde, URL and the Microsoft windows bindings.
The release package contains an inventory of the resolved Windows dependency
versions and license expressions in `licenses/dependencies.json`, plus available
upstream license/notice files copied from those exact local Cargo packages.
Workspace source is covered by the included MIT LICENSE.

Inno Setup creates the installer. Its installer-engine notices are retained in
the generated setup executable. Inno Setup is not needed to run the app.
Windows, Chrome and their bundled system components retain their own licenses.
The application and extension branding is original artwork from this project.

Rebuild and source: Cargo manifests/lock and the scripts in this repository are
the source of truth. The package generator stops if a dependency has neither a
license expression nor a license file. Review applicable upstream license terms
before redistributing a modified release.
