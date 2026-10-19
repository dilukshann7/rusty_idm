# IDM Chromium integration: static protocol investigation

Date: 2026-10-18. Authority of the initial phase: installed, shipped artifacts. During that static phase no extension was installed or executed, no browser profile was inspected, and no cookie or browsing records were collected. Later user-assisted runtime tests are described in the follow-up below and in the main report; cookie-bearing endpoint requests were subsequently redacted. **Observed** in the static trace below means present in the identified source, not runtime verified.

## Artifact identity and coverage

The package inventory is `analysis/idm/browser/package-inventory.json`; it contains complete manifests and member names for the three installed CRX files. `extracted-inventory.json` records the size and SHA-256 of every selected extracted member and copied host declaration. CRX headers were parsed, then ZIP members were extracted with a containment check against the destination directory. This is extraction, not signature verification.

| Installed package | CRX format | Manifest | Extension version | Size | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| IDMGCExt.crx (selected) | 3 | 3 | 6.42.60 | 107461 | `8ac82b1abdfa5d1fac79fdf8511f1d86a7e7fb7a02d32e4a01e52b64d85fa016` |
| IDMEdgeExt.crx | 3 | 2 | 6.42.60 | 103237 | `9dc515fa15bab48bd67b33ddc8495a8024fd98eb80fbadb926f3c7aede9203ec` |
| IDMGCExt59.crx | 2 | 2 | 6.40.8.2 | 97524 | `3a5c70182a4a31c860295ab2931c34661a3c894dc02623ae6e2a70c9c378bac0` |

All originate from `C:\Program Files (x86)\Internet Download Manager`. The selected package identifies itself as IDM Integration Module and declares background service worker `background.js`, document-start content script `content.js`, minimum Chrome 109, `nativeMessaging`, cookies, downloads, webRequest, scripting and declarativeNetRequest permissions (`extension/manifest.json:13-25`). Declared capability does not prove current browser permission, installation or operation. The Edge package is not assumed modern merely from its name: its actual shipped manifest is version 2.

Selected source identities:

- `extension/background.js`: `c9e5aa6be16c74530be90aa635485f166936d850c375907b6378da07ab677fd7`.
- `extension/content.js`: `84a7881bca0ad8af23ae4538afcbeaaf1978e1c42ffa7fa65cd7d579d5c50627`.
- `extension/manifest.json`: `47eafaec22b44bffa7155ae497537b9d33de54886a5cd3adbb0171e2a676c2a6`.

References below are original physical line numbers, preserved in the extracted files. Minified names are used as source locations, not inferred original symbols.

REA target-free `analyze_javascript_application` analyzed `background.js` and `content.js` as inert text. Summary evidence `ev_5708179b68eba5330e262acdc6b86c567988c96e749c23f39bfca2ac19add676`, its parent reference `ev_d62eb95b7aaa3f0d3ede024e3895310672ba3dbad06fe6dc7fa6a60fde506203`, and limitations are preserved in `analysis/idm/browser/rea-javascript-summary.json`. Artifact-tree digest: `522bf2c64f92712f1167b657171a3fc8a1c634791944443e628bd95af7efbcb6`. Both sources parsed with no parse failures; background semantic analysis exhausted the worker heap. Application and semantic coverage are partial (26 application unknowns and 2517 semantic unknowns), and 25 inventoried files were not semantically analyzed. This report's detailed protocol trace comes from direct inspection of the preserved source, not a claim of complete automated REA recovery. The complete parent record was not exported from the worker session before that session ended; the retained local evidence is the summary plus identified source files, not the full automated semantic graph.

## Event-to-transport trace

```mermaid
sequenceDiagram
    participant U as User/browser
    participant C as Content script
    participant B as Background worker
    participant W as IDM WebSocket listener
    participant H as IDMMsgHost.exe
    participant I as IDM application
    C->>B: runtime port and numeric array messages
    U->>B: contextMenus.onClicked or downloads/webRequest events
    B->>B: collect URL, cookies, headers and request context
    alt earlier connection attempts: WebSocket
        B->>W: MSG#... payload (Blob)
    else later attempt: native messaging
        B->>H: connectNative(com.tonec.idm), postMessage(string)
        H-->>I: forwarding mechanism requires native evidence
    end
    H-->>B: native message converted to Blob and parsed
    W-->>B: WebSocket message parsed
    B->>B: response dispatch; continue/cancel browser download
```

The WebSocket listener's ownership and the host-to-application forwarding are **unknown from JavaScript**. The diagram does not establish that the socket listener is implemented by IDMan.exe itself.

| Stage | Static observation | Source |
| --- | --- | --- |
| Establish transport | `La` lists `127.0.0.1:1001`, then `0.1.0.1:1001`; attempts rotate through these and native messaging. Socket URL includes `cid` and random `rnd`, with subprotocol `plugin.v3.internetdownloadmanager.com`. | background.js:11,37-39 |
| Native fallback | `browser.runtime.connectNative("com.tonec.idm")`; disconnect and message listeners installed. | background.js:37-40 |
| Initial handshake | `Na` resets sequence to 1 and emits type 2, numeric slot 5, flags 1024 for the normal Chromium branch. Positional arguments include manifest version; fields 112/113 carry computed browser identity strings, 116 UI language, and 125 JSON-stringified installation-event data when `sb` has populated `nc`. | background.js:23,38,133-135 |
| Context-menu download | `contextMenus.onClicked` binds `Xa`; dispatch selects `Ea` through the menu table. `Ea` obtains `linkUrl` / `srcUrl`, page/frame URL and calls `pa(...,4)`, which retrieves cookies and emits type 14. | background.js:22,25,45,47-48,132 |
| Content-script route | Content opens a runtime port named top/sub; background `Ib` attaches `Jb`. Content arrays use a separate numeric operation namespace. Example: content operation 35 dispatches `Eb`, which calls `pa`. They are not native MSG types. | content.js:9,31; background.js:51,69,71-72 |
| Automatic capture | Listener setup registers downloads.onCreated/onDeterminingFilename and webRequest stages. `qb` can pause a browser download; `rb` matches policy and calls `kc` or `X`. | background.js:29-30,51-54 |
| HTTP observation | `cb` tracks GET/POST and request bodies, `Rb` records request headers, `Ub` records status/response headers and redirects, `X` emits type 13 with the context. | background.js:99-111 |
| Capture decision response | Incoming type 1 resolves a pending message by original sequence. Original type 13 response dispatch calls `jc` or `O`; branch value 3 requests cancellation, otherwise the browser can resume. Exact native meanings beyond this dispatch are unknown. | background.js:54,73,76-77,102 |

The context-menu dispatcher is minified `Xa` rather than `Ea` at registration; the routing table is necessary to establish their relationship. Configuration received from IDM gates capture behavior, so merely reaching onCreated does not prove every download will be intercepted.

## Inner message grammar and outer framing

Observed serializer `pb` and sender `S` (`background.js:15-16,41`):

```text
MSG#<sequence>#<type>#<numeric-slot-A>#<flags>
    [:<positional-number>...]
    [,<numeric-field>=<encoded-value>...];
```

`sequence` increments per connection. The semantic meaning of slot A is **unknown**; call sites assign 1, 2 or 5 and it must not be called a protocol version without native/runtime evidence. Flags are bitmasks with operation-specific uses. Flags bit 1 retains the outgoing message for response correlation. Numeric positional values use `m || 0` during serialization. Non-null fields with numeric keys are serialized in object enumeration order. Strings use their UTF-8 byte length followed by `:` and their value; numbers and booleans use numeric text. Arrays/byte buffers/Blob have additional length/binary handling. For newer native transport (`C == 2` and peer revision `R >= 99`), selected binary values are base64 with a `%` length marker; older branches and socket data can use `:` or `/`. These output branches are observed; their complete receiver acceptance rules are not established here.

Illustrative derived encoder output for a **synthetic**, field-limited message:

```text
MSG#1#14#1#0:1,6=30:http://127.0.0.1:8765/file.bin;
```

This is an explanation of the serializer, not a captured IDM message or a guaranteed accepted request. Parser `pb.prototype.B` recognizes MSG headers, numeric positional arguments, numeric fields and colon-delimited length-prefixed text; it validates delimiters, supports consecutive records and calls the disconnect callback on malformed input (`background.js:18-20`). No comprehensive protocol grammar is claimed for unobserved native-to-browser binary cases.

**Native outer framing:** source calls `port.postMessage` with `b.o.join("")`, a JavaScript string, and converts incoming `a` to `new Blob([a])` before parsing. Browser-provided native-message JSON framing and host input byte order are not implemented in the extension source and require host disassembly. Do not confuse this outer channel with MSG delimiters or assume the downstream host IPC uses either format.

**Socket framing:** `S` sends `new Blob(b.o)` as one WebSocket message; incoming event data is parsed through the same MSG object. This is a separate transport with no native JSON envelope at the extension call site.

## Selected types and fields

Everything in this section is **statically observed**, with semantic labels derived from the originating browser variables. Runtime verification: **none**. Table meanings are call-site specific rather than universal claims about every use of a field ID.

| Type | Observed use | Location |
| --- | --- | --- |
| 0 | Timer-triggered message, positional slot 1, flags 1, when peer revision is below 97. Keepalive is an inference from timer use. | background.js:38,40 |
| 1 | Response/acknowledgment; matches pending request by first positional value. Also sent as a generic reply for unhandled flagged messages. | background.js:73,77 |
| 2 | Initial outbound handshake. | background.js:38 |
| 3 / 4 | Incoming handshake/configuration branches; set peer revision and download-capture policy. | background.js:73-74 |
| 13 | Download/request interception report and correlated decision handling. | background.js:53,99-101,76-77 |
| 14 | Explicit URL/link download submission; single or batch item. | background.js:45-46 |
| 15 | Batch header for multiple links, with link count. | background.js:46 |
| 24 | Incoming cookie query uses field 6 and replies with field 51. | background.js:77-78 |

| Field | Observed content | Producer/source |
| --- | --- | --- |
| 6 | Download URL, sometimes transformed by `cc` (including `idmdwnlmfv4://` wrapper under a negotiated flag); not always a raw HTTP URL. | background.js:45,53,100,119 |
| 7 | Page context URL. | background.js:45,47,100 |
| 17 | Earlier/original URL when request/download was redirected. | background.js:53,100,105 |
| 50 | Referrer or supplied referring-page context. | background.js:45,47,53; request fallback `Kc` at 117 |
| 51 | Cookies formatted `name=value; name=value`, retrieved for target URL when needed. | background.js:45,53,78,99-100,120 |
| 54 | `navigator.userAgent`. | background.js:45,53,100 |
| 56 | Cookies for original URL when final browser download URL differs. | background.js:53 |
| 100 | Suggested/observed filename; derived from browser `filename` or selected response filename. | background.js:45,53,94-100 |
| 102 | Link display metadata passed by explicit-link/batch route; exact general meaning remains unknown. | background.js:45-46; content.js:31-35 |
| 103 | Browser download `totalBytes`, omitted when falsy. | background.js:53 |
| 4 | Extension/type hint from filename or pathname. | background.js:52-53,100 |
| 11 | Request header name/value pairs serialized with CRLF. Cookie/authorization-bearing headers can be included if supplied by the browser. | background.js:100,121 |
| 12 / 13 | HTTP status line / response headers. | background.js:100,109,121 |
| 14 | Request body reconstructed from raw/form data. | background.js:100,124-125 |
| 18 / 19 | Earlier request headers/body retained across redirects. | background.js:100,105 |
| 127 | Browser MIME type in `kc`; a separate request metadata variable in `X`, so global semantic identity is not proven. | background.js:53,100 |
| 121 / 122 | Proxy-related state, with different branches and usages. Exact wire semantics require native tracing. | background.js:32-34,45,53,77,100 |

Type 13's `X` positional values are `[request-or-download-id, content-port-id, floor(timestamp), status-code, protocol-kind, action-code, tab-id, frame-id]` (`background.js:99-100`). `kc` uses a shorter six-value array for browser downloads (`background.js:53`). Numeric flags and absent values therefore require route-specific handling. Neither this table nor the message IDs form a proposed Rust API.

## Host declarations and forwarding boundary

Copied Chromium declaration `analysis/idm/browser/IDMMsgHost.json:2-6` declares name `com.tonec.idm`, type `stdio`, path `IDMMsgHost.exe`, and allowed origins `chrome-extension://ngpampappnmepgilojfohadhhmbhlaek/` and `chrome-extension://llbjbkhnmlidjebalopleeepgdfgcpec/`. The selected manifest does not itself establish which origin Chrome will assign to an unpacked installation; preserve this as a runtime compatibility condition.

Firefox declaration `IDMMsgHostMoz.json:2-6` uses the same name, executable and stdio type, with `allowed_extensions: ["mozilla_cc3@internetdownloadmanager.com"]`. This compares declarations only. It does not prove Firefox source equivalence, registration in either browser, or the same transport preference in a separately shipped Firefox package.

**Unknown in this JavaScript-only phase:** the native host's reading/writing of browser envelopes, origin validation, forwarding transport, downstream record format and error behavior. The subsequent [native investigation](idm-investigation.md) establishes a session-specific named-pipe bridge, escape/unescape routines, length-prefix handling and selected errors. Origin validation and manager-side dispatch remain unresolved. No speculative private messages were sent to the host.

## Controlled runtime follow-up and independent Rust implications

1. Use an explicitly disposable Chromium user-data directory and the loopback fixture. Record Chromium version and whether it accepts this extension/package. Avoid changing the user's normal profile or installing copied binaries.
2. Verify effective extension ID and native host registration before assuming unpacked load can contact this declaration. Prefer a supported existing installation in the disposable profile; if unavailable, mark integration unresolved.
3. Use URLs, filenames, referrers, a synthetic cookie and request header that identify only the fixture. Trigger context-menu download and one ordinary browser download. Correlate browser events, fixture logs, chosen transport and IDM output hash.
4. Observe whether a socket is active before expecting native-host traffic. If only native evidence is required, do not disable the socket globally or alter existing IDM settings without isolating/restoring them.
5. Capture the outer message separately from the MSG payload. Confirm sequence/response and cancel/resume outcomes; do not infer success from a paused browser download alone.

For independent Rust implementation, expose an original, versioned native-messaging API with explicit URL, destination suggestion, referrer and allowed request headers. Cookies are sensitive request context and should stay local to the submitted job. Browser source demonstrates the need for per-route state, transfer-acceptance acknowledgment and fallback/resume of the browser download on handoff failure. These are new design recommendations, not claims that Rust must clone IDM's private numeric protocol or socket listener.

Follow-up runtime evidence: the user confirmed IDM takeover in the disposable Edge profile and browser.bin passed its fixture hash check. Loopback handoff therefore worked in that test. The selected transport and raw messages were not captured. Remaining unknowns include many flags/metadata IDs, dynamic configuration, permission-dependent header visibility, host/socket authentication and manager-side dispatch. The native envelope and forwarding findings are in the main report.
