//! Compact four-page Options window. Native callbacks never borrow application state.
use crate::{
    commands::*,
    native_form::{self as ui, Action, Form},
    options_editors as editors,
    platform::{self, item, message, send, wide},
    settings::Settings,
    AppError,
};
use rusty_browser_bridge::policy::{self, BrowserKind};
use rusty_engine::categories::{self, CategoryRule};
use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{HWND, RECT},
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::{Controls::*, Shell::*, WindowsAndMessaging::*},
    },
};
thread_local! {static OPEN:Cell<HWND>=const{Cell::new(HWND(std::ptr::null_mut()))};}
struct OpenGuard;
impl Drop for OpenGuard {
    fn drop(&mut self) {
        OPEN.with(|v| v.set(HWND::default()));
    }
}
const TITLES: [&str; 4] = ["General", "File types", "Save to", "Downloads"];
struct OwnedIcon(usize);
impl Drop for OwnedIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(HICON(self.0 as *mut _));
        }
    }
}
struct HeaderIcons {
    pending: Arc<Mutex<Vec<(usize, OwnedIcon)>>>,
    attached: Vec<OwnedIcon>,
    pages: [HWND; 4],
}
impl HeaderIcons {
    fn new(pages: [HWND; 4]) -> Self {
        let pending = Arc::new(Mutex::new(vec![]));
        let result = pending.clone();
        let _ = std::thread::Builder::new()
            .name("rusty-options-icons".into())
            .spawn(move || {
                if unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_err() {
                    return;
                }
                for (n, kind) in [SIID_DESKTOPPC, SIID_DOCASSOC, SIID_FOLDEROPEN, SIID_WORLD]
                    .into_iter()
                    .enumerate()
                {
                    let mut info = SHSTOCKICONINFO {
                        cbSize: std::mem::size_of::<SHSTOCKICONINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe { SHGetStockIconInfo(kind, SHGSI_ICON | SHGSI_LARGEICON, &mut info) }
                        .is_ok()
                    {
                        result
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .push((n, OwnedIcon(info.hIcon.0 as usize)));
                    }
                }
                unsafe {
                    CoUninitialize();
                }
            });
        Self {
            pending,
            attached: vec![],
            pages,
        }
    }
    fn drain(&mut self) {
        for (n, icon) in
            std::mem::take(&mut *self.pending.lock().unwrap_or_else(|p| p.into_inner()))
        {
            if let Ok(control) = item(self.pages[n], CFG_ICON) {
                send(control, STM_SETICON, icon.0, 0);
                self.attached.push(icon);
            }
        }
    }
}
impl Drop for HeaderIcons {
    fn drop(&mut self) {
        for page in self.pages {
            if let Ok(control) = item(page, CFG_ICON) {
                send(control, STM_SETICON, 0, 0);
            }
        }
    }
}
struct Pages {
    handles: [HWND; 4],
    category: usize,
    startup_readable: bool,
}
impl Pages {
    fn new(form: &Form, settings: &Settings) -> Result<Self, AppError> {
        let h = form.hwnd();
        let tabs = item(h, CFG_TABS)?;
        for (n, title) in TITLES.into_iter().enumerate() {
            let mut title = wide(title);
            let tab = TCITEMW {
                mask: TCIF_TEXT,
                pszText: PWSTR(title.as_mut_ptr()),
                ..Default::default()
            };
            send(tabs, TCM_INSERTITEMW, n, &tab as *const _ as isize);
        }
        let handles = [
            form.child(CONFIG_GENERAL)?,
            form.child(CONFIG_TYPES)?,
            form.child(CONFIG_SAVE)?,
            form.child(CONFIG_DOWNLOADS)?,
        ];
        let startup = crate::configuration_system::startup_enabled();
        let pages = Self {
            handles,
            category: 0,
            startup_readable: startup.is_ok(),
        };
        pages.position(h)?;
        let [general, types, save, downloads] = handles;
        let c = &settings.configuration;
        ui::check(general, CFG_STARTUP, startup.unwrap_or(false));
        ui::enabled(general, CFG_STARTUP, pages.startup_readable);
        ui::check(general, CFG_CLIPBOARD, c.clipboard);
        let list = item(general, CFG_BROWSERS)?;
        send(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_CHECKBOXES | LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as isize,
        );
        for (n, width) in [170, 140].into_iter().enumerate() {
            let column = LVCOLUMNW {
                mask: LVCF_WIDTH,
                cx: crate::controls::scale(
                    width,
                    unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(h) }.max(96),
                ),
                ..Default::default()
            };
            send(list, LVM_INSERTCOLUMNW, n, &column as *const _ as isize);
        }
        for (n, browser) in BrowserKind::ALL.into_iter().enumerate() {
            let mut label = wide(browser.label());
            let row = LVITEMW {
                mask: LVIF_TEXT,
                iItem: n as i32,
                pszText: PWSTR(label.as_mut_ptr()),
                ..Default::default()
            };
            send(list, LVM_INSERTITEMW, 0, &row as *const _ as isize);
            let row = LVITEMW {
                state: LIST_VIEW_ITEM_STATE_FLAGS(if c.capture.browsers.contains(&browser) {
                    2 << 12
                } else {
                    1 << 12
                }),
                stateMask: LVIS_STATEIMAGEMASK,
                ..Default::default()
            };
            send(list, LVM_SETITEMSTATE, n, &row as *const _ as isize);
        }
        pages.browser_status(settings)?;
        ui::put(
            types,
            CFG_EXTENSIONS,
            &c.capture.extensions.join(" ").to_ascii_uppercase(),
        );
        ui::put(types, CFG_SITES, &c.capture.excluded_sites.join(" "));
        ui::limit(types, CFG_EXTENSIONS, 16000);
        ui::limit(types, CFG_SITES, 16000);
        pages.category_choices(settings);
        pages.write_category(settings);
        ui::check(save, CFG_CREATION, c.server_creation_time);
        ui::put(
            save,
            CFG_TEMP,
            &c.temporary_folder()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        ui::limit(save, CFG_FOLDER, 32767);
        ui::limit(save, CFG_TEMP, 32767);
        for (id, on) in [
            (CFG_START_DIALOG, c.show_start_dialog),
            (CFG_QUEUE_ONLY, c.queue_only),
            (CFG_COMPLETE, settings.show_completion_dialog),
            (CFG_IMMEDIATE, c.immediate_info),
            (CFG_QUEUE_LATER, c.choose_queue_later),
            (CFG_QUEUE_BATCH, c.choose_queue_batch),
            (CFG_MTIME, c.ignore_modified_time),
        ] {
            ui::check(downloads, id, on);
        }
        ui::choices(
            downloads,
            CFG_DUPLICATES,
            &[
                "Show a dialog and ask what to do.",
                "Add a numbered copy.",
                "Replace existing, with overwrite approval.",
                "Show or resume the existing download.",
            ],
            match settings.duplicate_choice {
                None => 0,
                Some(crate::duplicates::DuplicateChoice::Numbered) => 1,
                Some(crate::duplicates::DuplicateChoice::Overwrite) => 2,
                Some(crate::duplicates::DuplicateChoice::Existing) => 3,
            },
        );
        ui::put(downloads, CFG_UA, &c.user_agent);
        ui::limit(downloads, CFG_UA, 1024);
        pages.dependencies();
        pages.show(h, 0)?;
        Ok(pages)
    }
    fn position(&self, root: HWND) -> Result<(), AppError> {
        let mut rect = RECT {
            left: 12,
            top: 24,
            right: 344,
            bottom: 284,
        };
        unsafe {
            MapDialogRect(root, &mut rect)?;
            for page in self.handles {
                SetWindowPos(
                    page,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )?;
            }
        }
        Ok(())
    }
    fn show(&self, root: HWND, n: usize) -> Result<(), AppError> {
        for (i, page) in self.handles.iter().enumerate() {
            unsafe {
                let visible = GetWindowLongW(*page, GWL_STYLE) as u32 & WS_VISIBLE.0 != 0;
                if visible != (i == n) {
                    let _ = ShowWindow(*page, if i == n { SW_SHOW } else { SW_HIDE });
                }
            }
        }
        send(item(root, CFG_TABS)?, TCM_SETCURSEL, n, 0);
        Ok(())
    }
    fn dependencies(&self) {
        ui::enabled(
            self.handles[3],
            CFG_QUEUE_ONLY,
            !ui::checked(self.handles[3], CFG_START_DIALOG),
        );
    }
    fn category_choices(&self, settings: &Settings) {
        let labels: Vec<_> = settings
            .configuration
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        ui::choices(self.handles[2], CFG_CATEGORIES, &labels, self.category);
    }
    fn read_category(&self, settings: &mut Settings) {
        if let Some(rule) = settings.configuration.categories.get_mut(self.category) {
            let folder = ui::value(self.handles[2], CFG_FOLDER);
            rule.folder = (!folder.trim().is_empty()).then(|| PathBuf::from(folder.trim()));
            rule.remember = ui::checked(self.handles[2], CFG_REMEMBER);
        }
    }
    fn write_category(&self, settings: &Settings) {
        if let Some(rule) = settings.configuration.categories.get(self.category) {
            ui::put(
                self.handles[2],
                CFG_CATEGORY_TYPES,
                &if rule.name == "General" {
                    "The file types not listed in any other category".into()
                } else {
                    rule.extensions.join(" ").to_ascii_uppercase()
                },
            );
            ui::put(
                self.handles[2],
                CFG_FOLDER,
                &rule
                    .folder
                    .as_deref()
                    .unwrap_or(&settings.default_destination)
                    .to_string_lossy(),
            );
            ui::check(self.handles[2], CFG_REMEMBER, rule.remember);
        }
    }
    fn browser_status(&self, settings: &Settings) -> Result<(), AppError> {
        let list = item(self.handles[0], CFG_BROWSERS)?;
        let mut count = 0;
        for (n, browser) in BrowserKind::ALL.into_iter().enumerate() {
            let registered = crate::configuration_system::registered_id(browser).is_some();
            if registered {
                count += 1;
            }
            let status = if settings
                .configuration
                .registrations
                .iter()
                .any(|r| r.browser == browser)
            {
                "Setup on OK"
            } else if registered {
                "Host registered"
            } else if crate::configuration_system::browser_executable(browser, settings).is_some() {
                "Extension setup needed"
            } else {
                "Not detected"
            };
            let mut text = wide(status);
            let item = LVITEMW {
                iSubItem: 1,
                pszText: PWSTR(text.as_mut_ptr()),
                ..Default::default()
            };
            send(list, LVM_SETITEMTEXTW, n, &item as *const _ as isize);
        }
        ui::put(
            self.handles[0],
            CFG_STATUS,
            &format!("{count} browser host(s) registered."),
        );
        Ok(())
    }
    fn gather(&self, draft: &Settings) -> Result<Settings, (usize, String)> {
        let mut settings = draft.clone();
        self.read_category(&mut settings);
        let c = &mut settings.configuration;
        let [general, types, save, downloads] = self.handles;
        if self.startup_readable {
            c.startup = Some(ui::checked(general, CFG_STARTUP));
        }
        c.clipboard = ui::checked(general, CFG_CLIPBOARD);
        let list = item(general, CFG_BROWSERS).map_err(|e| (0, e.to_string()))?;
        c.capture.browsers = BrowserKind::ALL
            .into_iter()
            .enumerate()
            .filter_map(|(n, b)| {
                ((send(list, LVM_GETITEMSTATE, n, LVIS_STATEIMAGEMASK.0 as isize).0 >> 12) == 2)
                    .then_some(b)
            })
            .collect();
        c.capture.extensions =
            policy::extensions(&ui::value(types, CFG_EXTENSIONS)).map_err(|e| (1, e))?;
        c.capture.excluded_sites =
            policy::site_patterns(&ui::value(types, CFG_SITES)).map_err(|e| (1, e))?;
        c.server_creation_time = ui::checked(save, CFG_CREATION);
        let temp = ui::value(save, CFG_TEMP);
        let system_default =
            std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("RustyIDM/partials"));
        let chosen = (!temp.trim().is_empty()).then(|| PathBuf::from(temp.trim()));
        c.temporary_directory = chosen.filter(|p| Some(p) != system_default.as_ref());
        c.show_start_dialog = ui::checked(downloads, CFG_START_DIALOG);
        c.queue_only = ui::checked(downloads, CFG_QUEUE_ONLY);
        c.immediate_info = ui::checked(downloads, CFG_IMMEDIATE);
        c.choose_queue_later = ui::checked(downloads, CFG_QUEUE_LATER);
        c.choose_queue_batch = ui::checked(downloads, CFG_QUEUE_BATCH);
        c.ignore_modified_time = ui::checked(downloads, CFG_MTIME);
        c.user_agent = ui::value(downloads, CFG_UA);
        settings.show_completion_dialog = ui::checked(downloads, CFG_COMPLETE);
        settings.duplicate_choice = match ui::selection(downloads, CFG_DUPLICATES) {
            Some(1) => Some(crate::duplicates::DuplicateChoice::Numbered),
            Some(2) => Some(crate::duplicates::DuplicateChoice::Overwrite),
            Some(3) => Some(crate::duplicates::DuplicateChoice::Existing),
            _ => None,
        };
        if let Some(rule) = c.categories.iter().find(|r| r.name == "General") {
            if let Some(path) = &rule.folder {
                settings.default_destination = path.clone();
            }
        }
        c.download_options(true).validate().map_err(|e| (3, e))?;
        c.scanner.validate().map_err(|e| (3, e))?;
        c.validate().map_err(|e| {
            (
                if e.contains("User-Agent") || e.contains("Scanner") {
                    3
                } else {
                    2
                },
                e,
            )
        })?;
        Ok(settings)
    }
}
pub fn show(owner: HWND, settings: &Settings) -> Result<Option<Settings>, AppError> {
    let existing = OPEN.with(Cell::get);
    if !existing.is_invalid() {
        platform::bring_to_front(existing);
        return Ok(None);
    }
    let mut form = Form::new(
        owner,
        OPTIONS_DIALOG,
        "Rusty Download Manager Configuration",
    )?;
    OPEN.with(|v| v.set(form.hwnd()));
    let _guard = OpenGuard;
    let h = form.hwnd();
    let mut draft = settings.clone();
    let mut pages = Pages::new(&form, &draft)?;
    let mut icons = HeaderIcons::new(pages.handles);
    unsafe {
        SetTimer(Some(h), 1, 100, None);
    }
    form.show();
    while let Some(actions) = form.next()? {
        icons.drain();
        for action in actions {
            match action {
                Action::Dpi => pages.position(h)?,
                Action::Timer => {}
                Action::Tab => {
                    let n = send(item(h, CFG_TABS)?, TCM_GETCURSEL, 0, 0).0.max(0) as usize;
                    pages.show(h, n.min(3))?;
                }
                Action::Command(page, id, notification) => match id {
                    2 => return Ok(None),
                    1 => match pages.gather(&draft) {
                        Ok(settings) => return Ok(Some(settings)),
                        Err((page, e)) => {
                            pages.show(h, page)?;
                            message(Some(h), "Configuration", &e, false);
                        }
                    },
                    9 => {
                        let n = send(item(h, CFG_TABS)?, TCM_GETCURSEL, 0, 0).0.max(0) as usize;
                        message(Some(h), TITLES[n.min(3)], HELP[n.min(3)], false);
                    }
                    CFG_START_DIALOG => pages.dependencies(),
                    CFG_EXT_DEFAULT => {
                        ui::put(pages.handles[1], CFG_EXTENSIONS, policy::DEFAULT_EXTENSIONS)
                    }
                    CFG_SITE_DEFAULT => ui::put(
                        pages.handles[1],
                        CFG_SITES,
                        &policy::CapturePolicy::default().excluded_sites.join(" "),
                    ),
                    CFG_ADDRESSES_EDIT => {
                        if let Some(values) =
                            editors::addresses(h, &draft.configuration.capture.excluded_addresses)?
                        {
                            draft.configuration.capture.excluded_addresses = values;
                        }
                    }
                    CFG_FOLDER_BROWSE | CFG_TEMP_BROWSE => {
                        let control = if id == CFG_FOLDER_BROWSE {
                            CFG_FOLDER
                        } else {
                            CFG_TEMP
                        };
                        if let Some(path) = crate::dialogs::choose_folder(
                            h,
                            &PathBuf::from(ui::value(page, control)),
                        )? {
                            ui::put(page, control, &path.to_string_lossy());
                        }
                    }
                    CFG_CATEGORIES if notification == CBN_SELCHANGE => {
                        pages.read_category(&mut draft);
                        pages.category = ui::selection(page, CFG_CATEGORIES).unwrap_or(0);
                        pages.write_category(&draft);
                    }
                    CFG_CATEGORY_NEW | CFG_CATEGORY_EDIT => {
                        pages.read_category(&mut draft);
                        let new = id == CFG_CATEGORY_NEW;
                        let old = if new {
                            CategoryRule {
                                name: String::new(),
                                extensions: vec![],
                                folder: Some(draft.default_destination.clone()),
                                remember: true,
                            }
                        } else {
                            draft.configuration.categories[pages.category].clone()
                        };
                        if let Some(rule) =
                            editors::category(h, &old, &draft.default_destination, new)?
                        {
                            let mut rules = draft.configuration.categories.clone();
                            let conflicts: Vec<_> = rules
                                .iter()
                                .filter(|r| new || r.name != old.name)
                                .filter(|r| {
                                    r.extensions.iter().any(|e| rule.extensions.contains(e))
                                })
                                .map(|r| r.name.clone())
                                .collect();
                            if !conflicts.is_empty()
                                && !message(
                                    Some(h),
                                    "Move file types?",
                                    &format!(
                                        "Move overlapping file types from {} into {}?",
                                        conflicts.join(", "),
                                        rule.name
                                    ),
                                    true,
                                )
                            {
                                continue;
                            }
                            for r in &mut rules {
                                if new || r.name != old.name {
                                    r.extensions.retain(|e| !rule.extensions.contains(e));
                                }
                            }
                            if new {
                                rules.push(rule.clone());
                            } else {
                                rules[pages.category] = rule.clone();
                            }
                            match categories::validate_rules(&rules) {
                                Err(e) => {
                                    message(Some(h), "Category", &e, false);
                                }
                                Ok(()) => {
                                    if !new && old.name != rule.name {
                                        let mut chained = false;
                                        for (_, to) in &mut draft.configuration.renamed_categories {
                                            if *to == old.name {
                                                *to = rule.name.clone();
                                                chained = true;
                                            }
                                        }
                                        if !chained {
                                            draft
                                                .configuration
                                                .renamed_categories
                                                .push((old.name, rule.name.clone()));
                                        }
                                    }
                                    draft.configuration.categories = rules;
                                    if new {
                                        pages.category = draft.configuration.categories.len() - 1;
                                    }
                                    pages.category_choices(&draft);
                                    pages.write_category(&draft);
                                }
                            }
                        }
                    }
                    CFG_ADD_BROWSER => {
                        if let Some((registration, path)) = editors::browser(h, &draft)? {
                            if let Some(path) = path {
                                draft
                                    .configuration
                                    .browser_executables
                                    .insert(registration.browser, path);
                            }
                            draft.configuration.registrations.retain(|r| {
                                r.browser != registration.browser
                                    || r.extension_id != registration.extension_id
                            });
                            draft.configuration.registrations.push(registration);
                            pages.browser_status(&draft)?;
                        }
                    }
                    CFG_RESTART => {
                        draft.configuration.capture.restart_generation = draft
                            .configuration
                            .capture
                            .restart_generation
                            .saturating_add(1);
                        ui::put(page, CFG_STATUS, "Integration reconnects on OK.");
                    }
                    CFG_KEYS => {
                        let keys = &draft.configuration.capture.keys;
                        let initial: [bool; 6] = std::array::from_fn(|n| {
                            if n < 3 {
                                keys.prevent & (1 << n) != 0
                            } else {
                                keys.force & (1 << (n - 3)) != 0
                            }
                        });
                        if let Some(flags)=editors::flags(h,"Browser capture keys","Hold all checked keys for a rule. Prevent wins; force overrides file types only.",&["Prevent: Ctrl","Prevent: Alt","Prevent: Shift","Force: Ctrl","Force: Alt","Force: Shift"],&initial)?{draft.configuration.capture.keys.prevent=(0..3).fold(0,|mask,n|mask|if flags[n]{1<<n}else{0});draft.configuration.capture.keys.force=(0..3).fold(0,|mask,n|mask|if flags[n+3]{1<<n}else{0});}
                    }
                    CFG_MENUS => {
                        let m = &draft.configuration.capture.menus;
                        if let Some(v) = editors::flags(
                            h,
                            "Browser context menus",
                            "Choose explicit Download with Rusty actions.",
                            &["Links", "Selected URL text", "Direct audio/video"],
                            &[m.links, m.selection, m.media],
                        )? {
                            let m = &mut draft.configuration.capture.menus;
                            m.links = v[0];
                            m.selection = v[1];
                            m.media = v[2];
                        }
                    }
                    CFG_PANELS => {
                        let p = &draft.configuration.capture.panels;
                        if let Some(v)=editors::flags(h,"Browser download panels","Direct HTTP(S) media files only; streams and protected media are not extracted.",&["Show download panels","Video files","Audio files","Show only on hover"],&[p.enabled,p.video,p.audio,p.hover_only])?{let p=&mut draft.configuration.capture.panels;p.enabled=v[0];p.video=v[1];p.audio=v[2];p.hover_only=v[3];}
                    }
                    CFG_PROGRESS => {
                        let p = &draft.configuration.progress;
                        if let Some(v) = editors::flags(
                            h,
                            "Download progress dialog",
                            "The completed download dialog is controlled separately.",
                            &[
                                "Show progress dialog",
                                "Bring progress to the foreground",
                                "Show details initially",
                                "Close progress when download finishes",
                            ],
                            &[p.show, p.foreground, p.details, p.close_when_finished],
                        )? {
                            let p = &mut draft.configuration.progress;
                            p.show = v[0];
                            p.foreground = v[1];
                            p.details = v[2];
                            p.close_when_finished = v[3];
                        }
                    }
                    CFG_SCANNER_EDIT => {
                        if let Some(scanner) = editors::scanner(h, &draft.configuration.scanner)? {
                            draft.configuration.scanner = scanner;
                        }
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(None)
}
const HELP:[&str;4]=[
    "Each browser needs its Rusty extension and one-time website-access approval. A browser checkbox permits automatic capture; it does not grant that approval. Add browser stages its exact extension ID until OK. Clipboard monitoring starts with the next copied public HTTP(S) URL. Keys affect automatic capture; unsafe/credential-bearing requests remain in the browser.",
    "File types are extensions separated by spaces. Excluded sites are exact hosts or *.subdomain patterns. Excluded addresses are exact public HTTP(S) URLs, optionally ending with * for a prefix. These filters affect automatic capture and clipboard URLs; explicit Add URL/context-menu actions remain available.",
    "General catches otherwise unmatched types. Category folders affect future suggestions; manually edited paths are preserved. Remember folder updates only enabled categories. Changing the temporary directory leaves existing recovery files in their recorded locations. Copies into another final directory are journaled and verified before replacement.",
    "Turning off the start dialog uses the category folder and duplicate preference, but required overwrite/credential/recovery decisions still prompt. Immediate File Info downloads receive into a provisional partial until confirmed. Download Later holds in the selected queue. Ignoring modification time never bypasses identity, byte hashes or strict server resume validators. Virus checking runs only the configured executable with a standalone {file} argument."
];
