# Site Grabber

The current source adds **Tasks → Site Grabber**, a modeless native Windows
manager. The engine compiled during focused transfer tests. Native UI compilation
and live acceptance have not been run for this feature.

## Workflow

1. Choose **New**, enter a project name, HTTP/HTTPS starting URL and absolute save
   folder. Choose **Collect files** or **Offline website**, then **Save settings**.
2. Configure page/asset host lists and filters, then **Scan**. Empty host lists
   default to the starting hostname. Scanning reads HTML/CSS and inspects other
   response headers; binary bodies are downloaded only after review.
3. Check results to save. **Filter** searches URLs/MIME types. Results are paged
   in batches of 200; **Select page** and **Clear page** affect the displayed page.
4. Choose **Download selected**. Transfers share the ordinary engine's recovery,
   concurrency and guarded output handling. Use **Open folder**, or **Open
   offline** after publication.

New project transfers capture the current download options, including connection
count and retry policy. Mirror temporary files remain inside the project's cache.
Project Resume preserves frozen automatic-retry waits; polling does not bypass
them or undo a file paused individually in the main download list.

**Stop** retains results and progress; **Resume** continues manually. Closing the
manager leaves work running. Restarted projects remain held until manually
resumed. **Retry failed** retries owned transfers or failed discovery requests.
Missing or changed frozen originals require **Rescan**. Configuration changes
after scanning also require Rescan, which stops old work, waits for its workers,
and creates a fresh scan generation. Interrupted rescans/deletions retain their
intent and remain held until Resume completes them. Previous exported files
remain intact.

## Scope and filters

Only public server-delivered HTTP/HTTPS content is supported. Login sessions,
browser cookies, JavaScript rendering, form submission, script-source discovery
and scheduled rescans are excluded. Client-rendered applications and browser
features requiring a web server may not work from local files. Discovery servers
must honor `Accept-Encoding: identity`.

Lists are comma separated. Hosts match exactly, with optional `*.example.com`
rules for descendants; that wildcard does not include `example.com` itself.
Separate page/asset scopes and exclusion patterns apply to redirects during
scanning and transfers. URL patterns support `*` and `?` and are case sensitive.
Exclusions prevent traversal. Includes control result eligibility, allowing
`*.pdf` while still traversing HTML. Extensions are case insensitive and examine
the URL path without its query. MIME types support patterns such as `image/*`.
Each list uses OR; populated extension/MIME lists combine with AND. Unknown MIME
cannot satisfy an explicit MIME filter. File filters never prevent HTML traversal.

Navigation starts at depth 0. Page links increase depth; embedded assets and CSS
dependencies retain their parent's depth. Linked-file headers may be inspected
at the next level to distinguish files from pages; out-of-depth HTML bodies are
not fetched. Deduplication removes fragments while
preserving query values/order. Parsers handle document bases, final response
URLs, embedded assets, `srcset`, inline CSS, `url()` and stylesheet imports.
Robots rules are enabled by default and matched per origin using
`RustySiteGrabber`; missing rules permit access, temporary failures hold that
origin until Resume retries it.

| Limit | Default |
| --- | --- |
| Navigation depth | 2 |
| HTML pages / unique results | 1,000 / 10,000 |
| Scan workers | 2 |
| Request start spacing per hostname | 500 ms |
| Redirects per request | 10 |
| HTML/CSS document | 8 MiB |
| Scan-original cache per generation | 256 MiB |
| Encrypted project manifest | 16 MiB |

Limits pause discovery visibly. Existing results can be reviewed/downloaded;
continuing discovery requires Rescan with higher limits or narrower filters.
Downloaded binary resources are separate from the scan-original cache. Shared
history capacity still applies to admissions.

## Persistence and offline output

Manifests live under
`%LOCALAPPDATA%\RustyIDM\grabber\<project-id>\project.v1.dpapi`, encrypted for the
current Windows user and replaced atomically. They retain settings, frontier,
results, selections, original hashes and durable download associations. Public
originals and mirror resources use adjacent cache directories. Corrupt projects
are preserved and reported.

Queue schema 8 reads schemas 1–7 and adds optional project membership. Existing
entry IDs, history, queues, checkpoints and journals remain intact. Older
binaries that only understand schema 7 cannot read newly saved state. Admissions
use `(project, scan generation, item)` identities to reconcile lost replies.

Collect mode saves below
`<save-folder>\rusty-files-<project-id>\scan-<generation>`. Offline mode publishes
fresh directories below
`<save-folder>\rusty-site-<project-id>\snapshot-<generation>-<revision>`, with an
`index.html` entry point. Filenames combine a readable stem and URL hash.

Mirror HTML/CSS comes from immutable scan originals. Successful selected targets
receive escaped relative links, with fragments retained. Other HTTP references
become absolute online URLs. Rewritten documents use UTF-8. The seed is mandatory;
selecting pages includes eligible dependencies unless explicitly deselected.
Partial snapshots report missing resources. Only completed publication changes
the current offline entry point; older snapshots remain available. Mirror
resources do not automatically open individual progress/completion dialogs.

Paths reject traversal and reparse points and use fresh files/directories,
preserving unrelated files. Frozen original hashes and completed resource
identities are checked before export. Project deletion stops owned work and
removes internal records/cache, preserving snapshots, collected files and
ordinary download history.

## Verification boundary

Source checks cover Rust parsing/formatting, locked workspace metadata, native
resource IDs/assets, documentation links and whitespace. Focused transfer tests
also compiled the engine, as recorded in [Validation](VALIDATION.md). These checks
do not establish Site Grabber runtime acceptance. Native UI compilation, live
feature checks, app launch and installer packaging remain deferred.

Deferred acceptance: depth/domain/filter/MIME combinations; redirect limits and
loops; robots failures; CSS cycles, `srcset`, bases, queries and fragments;
legacy encodings; missing/modified assets; Windows path collisions/reparse paths;
history capacity and lost admissions; restart/resume/retry; rescan generations;
retained snapshots; disk/storage failures; keyboard navigation, DPI and shutdown.
