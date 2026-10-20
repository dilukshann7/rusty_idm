//! Working compact editors opened outside native callbacks.
use crate::{
    commands::*,
    native_form::{self as ui, Action, Form},
    platform::{self, item, message, send, wide},
    preferences::{BrowserRegistration, ScannerPreferences},
    settings::Settings,
    AppError,
};
use rusty_browser_bridge::policy::{self, BrowserKind};
use rusty_engine::categories::CategoryRule;
use std::path::PathBuf;
use windows::Win32::{
    Foundation::HWND,
    UI::{Controls::*, WindowsAndMessaging::*},
};
pub fn text(
    owner: HWND,
    title: &str,
    notice: &str,
    initial: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    let mut form = Form::new(owner, CONFIG_TEXT, title)?;
    let h = form.hwnd();
    ui::put(h, CFG_EDITOR_LABEL, notice);
    ui::put(h, CFG_TEXT_VALUE, initial);
    ui::limit(h, CFG_TEXT_VALUE, max);
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    1 => return Ok(Some(ui::value(h, CFG_TEXT_VALUE))),
                    2 => return Ok(None),
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
pub fn flags(
    owner: HWND,
    title: &str,
    notice: &str,
    labels: &[&str],
    values: &[bool],
) -> Result<Option<Vec<bool>>, AppError> {
    let mut form = Form::new(owner, CONFIG_FLAGS, title)?;
    let h = form.hwnd();
    ui::put(h, CFG_EDITOR_LABEL, notice);
    for n in 0..8 {
        if let Ok(control) = item(h, CFG_FLAG_BASE + n) {
            if let Some(label) = labels.get(n as usize) {
                platform::set_text(control, label);
                ui::check(
                    h,
                    CFG_FLAG_BASE + n,
                    *values.get(n as usize).unwrap_or(&false),
                );
            } else {
                unsafe {
                    let _ = ShowWindow(control, SW_HIDE);
                }
            }
        }
    }
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    1 => {
                        return Ok(Some(
                            (0..labels.len())
                                .map(|n| ui::checked(h, CFG_FLAG_BASE + n as i32))
                                .collect(),
                        ))
                    }
                    2 => return Ok(None),
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
pub fn addresses(owner: HWND, initial: &[String]) -> Result<Option<Vec<String>>, AppError> {
    let mut form = Form::new(owner, CONFIG_ADDRESSES, "Excluded download addresses")?;
    let h = form.hwnd();
    let list = item(h, CFG_ADDRESS_LIST)?;
    let mut rows = initial.to_vec();
    let draw = |rows: &[String]| {
        send(list, LB_RESETCONTENT, 0, 0);
        for row in rows {
            let value = wide(row);
            send(list, LB_ADDSTRING, 0, value.as_ptr() as isize);
        }
    };
    draw(&rows);
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                let selected = send(list, LB_GETCURSEL, 0, 0).0;
                match id {
                    1 => return Ok(Some(rows)),
                    2 => return Ok(None),
                    CFG_ADDRESS_ADD | CFG_ADDRESS_CHANGE => {
                        if id == CFG_ADDRESS_ADD && rows.len() >= 256 {
                            message(
                                Some(h),
                                "Address limit",
                                "Maximum 256 excluded addresses.",
                                false,
                            );
                            continue;
                        }
                        let initial = if id == CFG_ADDRESS_CHANGE {
                            if selected < 0 {
                                continue;
                            }
                            rows[selected as usize].clone()
                        } else {
                            String::new()
                        };
                        if let Some(value) = text(
                            h,
                            "Excluded address",
                            "HTTP(S) address; append * for a URL prefix.",
                            &initial,
                            8192,
                        )? {
                            match policy::address_pattern(&value) {
                                Ok(value) => {
                                    if id == CFG_ADDRESS_CHANGE {
                                        rows[selected as usize] = value;
                                    } else {
                                        rows.push(value);
                                    }
                                    rows.sort();
                                    rows.dedup();
                                    draw(&rows);
                                }
                                Err(e) => {
                                    message(Some(h), "Excluded address", &e, false);
                                }
                            }
                        }
                    }
                    CFG_ADDRESS_REMOVE if selected >= 0 => {
                        rows.remove(selected as usize);
                        draw(&rows);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
pub fn category(
    owner: HWND,
    initial: &CategoryRule,
    default_folder: &std::path::Path,
    new: bool,
) -> Result<Option<CategoryRule>, AppError> {
    let mut form = Form::new(
        owner,
        CONFIG_CATEGORY,
        if new { "New category" } else { "Edit category" },
    )?;
    let h = form.hwnd();
    let fixed = initial.name == "General"
        || rusty_engine::categories::BUILTIN.contains(&initial.name.as_str());
    ui::put(h, CFG_CAT_NAME, &initial.name);
    ui::put(h, CFG_CAT_EXT, &initial.extensions.join(" "));
    ui::put(
        h,
        CFG_CAT_FOLDER,
        &initial
            .folder
            .as_deref()
            .unwrap_or(default_folder)
            .to_string_lossy(),
    );
    ui::check(h, CFG_CAT_REMEMBER, initial.remember);
    ui::enabled(h, CFG_CAT_NAME, !fixed);
    ui::enabled(h, CFG_CAT_EXT, initial.name != "General");
    ui::limit(h, CFG_CAT_NAME, 80);
    ui::limit(h, CFG_CAT_EXT, 16000);
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    2 => return Ok(None),
                    CFG_CAT_BROWSE => {
                        if let Some(path) = crate::dialogs::choose_folder(
                            h,
                            &PathBuf::from(ui::value(h, CFG_CAT_FOLDER)),
                        )? {
                            ui::put(h, CFG_CAT_FOLDER, &path.to_string_lossy());
                        }
                    }
                    1 => {
                        let name = ui::value(h, CFG_CAT_NAME).trim().to_owned();
                        let folder = PathBuf::from(ui::value(h, CFG_CAT_FOLDER).trim());
                        let extensions = match policy::extensions(&ui::value(h, CFG_CAT_EXT)) {
                            Ok(e) => e,
                            Err(e) => {
                                message(Some(h), "Category", &e, false);
                                continue;
                            }
                        };
                        if name.is_empty()
                            || name.chars().any(char::is_control)
                            || name.eq_ignore_ascii_case("Automatic")
                            || !folder.is_absolute()
                            || !folder.is_dir()
                        {
                            message(
                                Some(h),
                                "Category",
                                "Enter a category name and an existing absolute folder.",
                                false,
                            );
                            continue;
                        }
                        return Ok(Some(CategoryRule {
                            name,
                            extensions,
                            folder: Some(folder),
                            remember: ui::checked(h, CFG_CAT_REMEMBER),
                        }));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
pub fn scanner(
    owner: HWND,
    initial: &ScannerPreferences,
) -> Result<Option<ScannerPreferences>, AppError> {
    let mut form = Form::new(owner, CONFIG_SCANNER, "Virus checking settings")?;
    let h = form.hwnd();
    ui::check(h, CFG_SCAN_ENABLED, initial.enabled);
    ui::put(h, CFG_SCAN_EXE, &initial.executable.to_string_lossy());
    ui::put(
        h,
        CFG_SCAN_ARGS,
        &if initial.arguments.is_empty() {
            "{file}".into()
        } else {
            initial.arguments.join("\r\n")
        },
    );
    ui::limit(h, CFG_SCAN_EXE, 32767);
    ui::limit(h, CFG_SCAN_ARGS, 16000);
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, _) = action {
                match id {
                    2 => return Ok(None),
                    CFG_SCAN_BROWSE => {
                        if let Some(path) = crate::dialogs::choose_program(h)? {
                            ui::put(h, CFG_SCAN_EXE, &path.to_string_lossy());
                        }
                    }
                    1 => {
                        let options = ScannerPreferences {
                            enabled: ui::checked(h, CFG_SCAN_ENABLED),
                            executable: ui::value(h, CFG_SCAN_EXE).trim().into(),
                            arguments: ui::value(h, CFG_SCAN_ARGS)
                                .lines()
                                .map(str::to_owned)
                                .collect(),
                        };
                        match options.validate() {
                            Ok(()) => return Ok(Some(options)),
                            Err(e) => {
                                message(Some(h), "Virus checking", &e, false);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
pub fn browser(
    owner: HWND,
    settings: &Settings,
) -> Result<Option<(BrowserRegistration, Option<PathBuf>)>, AppError> {
    let mut form = Form::new(owner, CONFIG_BROWSER, "Add browser integration")?;
    let h = form.hwnd();
    ui::choices(
        h,
        CFG_BROWSER_KIND,
        &BrowserKind::ALL.map(BrowserKind::label),
        0,
    );
    let mut selected = BrowserKind::Chrome;
    let load = |b: BrowserKind| {
        ui::put(
            h,
            CFG_BROWSER_ID,
            &crate::configuration_system::registered_id(b).unwrap_or_else(|| {
                if b == BrowserKind::Firefox {
                    "rusty-download-manager@rusty.local".into()
                } else {
                    String::new()
                }
            }),
        );
        ui::put(
            h,
            CFG_BROWSER_EXE,
            &crate::configuration_system::browser_executable(b, settings)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
    };
    load(selected);
    ui::limit(h, CFG_BROWSER_ID, 200);
    ui::limit(h, CFG_BROWSER_EXE, 32767);
    form.show();
    while let Some(actions) = form.next()? {
        for action in actions {
            if let Action::Command(_, id, notification) = action {
                match id {
                    2 => return Ok(None),
                    CFG_BROWSER_KIND if notification == CBN_SELCHANGE => {
                        selected = BrowserKind::ALL
                            [ui::selection(h, CFG_BROWSER_KIND).unwrap_or(0).min(3)];
                        load(selected);
                    }
                    CFG_BROWSER_BROWSE => {
                        if let Some(path) = crate::dialogs::choose_program(h)? {
                            if let Some(browser) =
                                crate::configuration_system::executable_kind(&path)
                            {
                                selected = browser;
                                send(
                                    item(h, CFG_BROWSER_KIND)?,
                                    CB_SETCURSEL,
                                    BrowserKind::ALL
                                        .iter()
                                        .position(|b| *b == browser)
                                        .unwrap_or(0),
                                    0,
                                );
                                load(selected);
                                ui::put(h, CFG_BROWSER_EXE, &path.to_string_lossy());
                            } else {
                                message(
                                    Some(h),
                                    "Supported browsers",
                                    "Select chrome.exe, brave.exe, msedge.exe or firefox.exe.",
                                    false,
                                );
                            }
                        }
                    }
                    1 => {
                        let id = ui::value(h, CFG_BROWSER_ID).trim().to_owned();
                        let valid = if selected == BrowserKind::Firefox {
                            id.len() >= 3
                                && id.len() <= 200
                                && id
                                    .bytes()
                                    .all(|c| c.is_ascii_alphanumeric() || b"@._-{}".contains(&c))
                        } else {
                            id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c))
                        };
                        let value = ui::value(h, CFG_BROWSER_EXE);
                        let path = (!value.trim().is_empty()).then(|| PathBuf::from(value.trim()));
                        if !valid
                            || path.as_ref().is_some_and(|p| {
                                !p.is_absolute()
                                    || !p.is_file()
                                    || crate::configuration_system::executable_kind(p)
                                        != Some(selected)
                            })
                        {
                            message(Some(h),"Browser setup","Enter the exact extension ID and a matching installed browser executable, or leave the executable blank for a standard install.",false);
                            continue;
                        }
                        return Ok(Some((
                            BrowserRegistration {
                                browser: selected,
                                extension_id: id,
                            },
                            path,
                        )));
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(None)
}
