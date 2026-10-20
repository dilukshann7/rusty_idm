use crate::{
    commands::*,
    download_form::{self, CATEGORIES},
    duplicates::DuplicateChoice,
    platform::{self, item, message, resource, set_text, text, wide},
    settings::Settings,
    AppError,
};
use rusty_browser_bridge::{
    lease::{CaptureLease, LeaseState},
    protocol::CaptureRequest,
};
use rusty_engine::{validate_request, DownloadRequest, ReplacementPolicy};
use std::os::windows::ffi::OsStringExt;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND, LPARAM, WPARAM},
        System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER},
        UI::{
            Controls::{BST_CHECKED, BST_UNCHECKED, EM_SETLIMITTEXT, EM_SETSEL},
            Input::KeyboardAndMouse::{EnableWindow, SetFocus},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};

#[derive(Clone, Copy)]
enum Mode {
    Address,
    Info,
    Options,
}
#[derive(Clone, Default)]
struct Draft {
    url: String,
    path: String,
    login: String,
    password: String,
    authorization: bool,
    category: String,
    description: String,
    remember: bool,
    deferred: bool,
    duplicate_choice: Option<DuplicateChoice>,
    show_completion_dialog: bool,
}
struct Context {
    mode: Mode,
    draft: Draft,
    settings: Settings,
    result: RefCell<Option<Draft>>,
    lease: Option<CaptureLease>,
}
pub struct AddedDownload {
    pub request: DownloadRequest,
    pub remember_path: bool,
}

pub fn show_add_download(
    owner: HWND,
    settings: &Settings,
) -> Result<Option<AddedDownload>, AppError> {
    let mut draft = Draft {
        category: "Automatic".into(),
        remember: true,
        ..Default::default()
    };
    loop {
        let Some(address) = show(owner, Mode::Address, &draft, settings, None)? else {
            return Ok(None);
        };
        draft = address;
        match download_form::suggested_filename(&draft.url) {
            Ok(filename) => {
                draft.path = download_form::folder_for(settings, &draft.category)
                    .join(filename)
                    .to_string_lossy()
                    .into_owned();
                break;
            }
            Err(error) => {
                message(Some(owner), "Invalid address", &error.message, false);
            }
        }
    }
    finish_download(owner, settings, draft, None)
}

fn finish_download(
    owner: HWND,
    settings: &Settings,
    mut draft: Draft,
    lease: Option<&CaptureLease>,
) -> Result<Option<AddedDownload>, AppError> {
    loop {
        let Some(info) = show(owner, Mode::Info, &draft, settings, lease)? else {
            return Ok(None);
        };
        draft = info;
        if lease.is_some_and(|l| l.state() != LeaseState::Pending) {
            return Ok(None);
        }
        if draft.category.trim().is_empty() || draft.category.chars().any(char::is_control) {
            message(
                Some(owner),
                "Choose a category",
                "Enter a category name or choose one from the list.",
                false,
            );
            continue;
        }
        let mut request = match validate_request(
            &draft.url,
            PathBuf::from(&draft.path),
            ReplacementPolicy::Deny,
        ) {
            Ok(request) => request,
            Err(error) => {
                message(Some(owner), "Invalid download", &error.message, false);
                continue;
            }
        };
        if !request.destination.is_absolute() {
            message(
                Some(owner),
                "Choose a save path",
                "Enter an absolute save path or use Browse.",
                false,
            );
            continue;
        }
        if !request.destination.parent().is_some_and(Path::is_dir) {
            message(
                Some(owner),
                "Folder unavailable",
                "The save folder does not exist. Choose an existing folder.",
                false,
            );
            continue;
        }
        request.deferred = draft.deferred;
        request.category = (!draft.category.eq_ignore_ascii_case("Automatic"))
            .then(|| draft.category.trim().to_owned());
        request.description = draft.description;
        if draft.authorization {
            request = request.with_authorization(draft.login, draft.password);
        }
        return Ok(Some(AddedDownload {
            request,
            remember_path: draft.remember,
        }));
    }
}

pub fn show_options(owner: HWND, settings: &Settings) -> Result<Option<Settings>, AppError> {
    crate::options::show(owner, settings)
}
pub(crate) fn choose_folder(owner: HWND, current: &Path) -> Result<Option<PathBuf>, AppError> {
    pick(owner, Mode::Options, current)
}
pub(crate) fn choose_program(owner: HWND) -> Result<Option<PathBuf>, AppError> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetOptions(
            dialog.GetOptions()?
                | FOS_FORCEFILESYSTEM
                | FOS_PATHMUSTEXIST
                | FOS_FILEMUSTEXIST
                | FOS_NOCHANGEDIR,
        )?;
        dialog.SetTitle(w!("Select a Windows program"))?;
        if let Err(error) = dialog.Show(Some(owner)) {
            if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(error.into());
        }
        let path = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let result = PathBuf::from(std::ffi::OsString::from_wide(path.as_wide()));
        CoTaskMemFree(Some(path.0.cast()));
        Ok(Some(result))
    }
}

fn show(
    owner: HWND,
    mode: Mode,
    draft: &Draft,
    settings: &Settings,
    lease: Option<&CaptureLease>,
) -> Result<Option<Draft>, AppError> {
    let context = Box::new(Context {
        mode,
        draft: draft.clone(),
        settings: settings.clone(),
        result: RefCell::new(None),
        lease: lease.cloned(),
    });
    let id = match mode {
        Mode::Address => ADD_DIALOG,
        Mode::Info => FILE_INFO_DIALOG,
        Mode::Options => OPTIONS_DIALOG,
    };
    let result = unsafe {
        DialogBoxParamW(
            Some(platform::instance()?),
            resource(id),
            Some(owner),
            Some(dialog_proc),
            LPARAM((&*context as *const Context) as isize),
        )
    };
    if result == -1 {
        return Err(windows::core::Error::from_thread().into());
    }
    Ok(context.result.into_inner())
}
fn value(dialog: HWND, id: i32) -> String {
    item(dialog, id).map(text).unwrap_or_default()
}
fn set_value(dialog: HWND, id: i32, value: &str) {
    if let Ok(control) = item(dialog, id) {
        set_text(control, value);
    }
}
fn checked(dialog: HWND, id: i32) -> bool {
    item(dialog, id)
        .is_ok_and(|control| platform::send(control, BM_GETCHECK, 0, 0).0 == BST_CHECKED.0 as isize)
}
fn set_checked(dialog: HWND, id: i32, checked: bool) {
    if let Ok(control) = item(dialog, id) {
        platform::send(
            control,
            BM_SETCHECK,
            if checked {
                BST_CHECKED.0
            } else {
                BST_UNCHECKED.0
            } as usize,
            0,
        );
    }
}
fn add_choice(dialog: HWND, id: i32, choice: &str) {
    if let Ok(control) = item(dialog, id) {
        let value = wide(choice);
        platform::send(control, CB_ADDSTRING, 0, value.as_ptr() as isize);
    }
}
fn enable_authorization(dialog: HWND) {
    let enabled = checked(dialog, USE_AUTHORIZATION);
    for id in [LOGIN_EDIT, PASSWORD_EDIT, LOGIN_LABEL, PASSWORD_LABEL] {
        if let Ok(control) = item(dialog, id) {
            unsafe {
                let _ = EnableWindow(control, enabled);
            }
        }
    }
}
fn enable_duplicate_action(dialog: HWND) {
    let enabled = !checked(dialog, DUPLICATE_ASK);
    for id in [DUPLICATE_ACTION, DUPLICATE_ACTION_LABEL] {
        if let Ok(control) = item(dialog, id) {
            unsafe {
                let _ = EnableWindow(control, enabled);
            }
        }
    }
}
fn update_path_hint(dialog: HWND) {
    let path = PathBuf::from(value(dialog, PATH_EDIT));
    set_value(
        dialog,
        DIRECTORY_TEXT,
        &path.parent().unwrap_or(Path::new("")).to_string_lossy(),
    );
    let category = value(dialog, CATEGORY_COMBO);
    set_value(
        dialog,
        REMEMBER_PATH,
        &format!("&Remember this path for \"{}\" category", category),
    );
}
fn category_changed(dialog: HWND, _context: &Context, selection: bool) {
    // CBN_SELCHANGE can arrive before the editable text has been synchronized.
    if selection {
        if let Ok(combo) = item(dialog, CATEGORY_COMBO) {
            let index = platform::send(combo, CB_GETCURSEL, 0, 0).0;
            if index >= 0 {
                let length = platform::send(combo, CB_GETLBTEXTLEN, index as usize, 0).0;
                if length >= 0 {
                    let mut buffer = vec![0u16; length as usize + 1];
                    platform::send(
                        combo,
                        CB_GETLBTEXT,
                        index as usize,
                        buffer.as_mut_ptr() as isize,
                    );
                    set_text(combo, &String::from_utf16_lossy(&buffer[..length as usize]));
                }
            }
        }
    }
    // Category changes classify the entry only. The selected save path is owned
    // by the user once File Info opens and must never be rewritten here.
    update_path_hint(dialog);
}
unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    std::panic::catch_unwind(|| unsafe { dialog_inner(hwnd, msg, wparam, lparam) }).unwrap_or(0)
}
unsafe fn dialog_inner(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    unsafe {
        if msg == WM_INITDIALOG {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            let context = &*(lparam.0 as *const Context);
            let draft = &context.draft;
            if context.lease.is_some() {
                SetTimer(Some(hwnd), 7, 250, None);
            }
            match context.mode {
                Mode::Address => {
                    if let Ok(edit) = item(hwnd, URL_EDIT) {
                        platform::send(edit, CB_LIMITTEXT, 65535, 0);
                    }
                    set_value(hwnd, URL_EDIT, &draft.url);
                    set_value(hwnd, LOGIN_EDIT, &draft.login);
                    set_value(hwnd, PASSWORD_EDIT, &draft.password);
                    set_checked(hwnd, USE_AUTHORIZATION, draft.authorization);
                    enable_authorization(hwnd);
                }
                Mode::Info => {
                    if let (Ok(icon), Ok(control)) =
                        (LoadIconW(None, IDI_APPLICATION), item(hwnd, FILE_TYPE_ICON))
                    {
                        platform::send(control, STM_SETICON, icon.0 as usize, 0);
                    }
                    let display = if context.lease.is_some() {
                        redact_url(&draft.url)
                    } else {
                        draft.url.clone()
                    };
                    set_value(hwnd, URL_EDIT, &display);
                    for category in CATEGORIES {
                        add_choice(hwnd, CATEGORY_COMBO, category);
                    }
                    for category in context.settings.category_destinations.keys() {
                        if !CATEGORIES
                            .iter()
                            .any(|built_in| built_in.eq_ignore_ascii_case(category))
                        {
                            add_choice(hwnd, CATEGORY_COMBO, category);
                        }
                    }
                    set_value(hwnd, CATEGORY_COMBO, &draft.category);
                    add_choice(hwnd, PATH_EDIT, &draft.path);
                    for folder in context.settings.category_destinations.values() {
                        let name = Path::new(&draft.path).file_name().unwrap_or_default();
                        add_choice(hwnd, PATH_EDIT, &folder.join(name).to_string_lossy());
                    }
                    if let Ok(edit) = item(hwnd, PATH_EDIT) {
                        platform::send(edit, CB_LIMITTEXT, 32767, 0);
                    }
                    set_value(hwnd, PATH_EDIT, &draft.path);
                    set_value(hwnd, DESCRIPTION_EDIT, &draft.description);
                    set_checked(hwnd, REMEMBER_PATH, draft.remember);
                    update_path_hint(hwnd);
                }
                Mode::Options => {
                    set_value(hwnd, PATH_EDIT, &draft.path);
                    set_checked(hwnd, DUPLICATE_ASK, draft.duplicate_choice.is_none());
                    for action in [
                        "Add a numbered filename",
                        "Overwrite the existing download",
                        "Show completed download or resume it",
                    ] {
                        add_choice(hwnd, DUPLICATE_ACTION, action);
                    }
                    let selected = match draft.duplicate_choice.as_ref() {
                        Some(DuplicateChoice::Numbered) => 0,
                        Some(DuplicateChoice::Overwrite) => 1,
                        Some(DuplicateChoice::Existing) | None => 2,
                    };
                    if let Ok(combo) = item(hwnd, DUPLICATE_ACTION) {
                        platform::send(combo, CB_SETCURSEL, selected, 0);
                    }
                    set_checked(hwnd, OPTION_COMPLETION, draft.show_completion_dialog);
                    enable_duplicate_action(hwnd);
                }
            }
            if context.lease.is_some() {
                platform::bring_to_front(hwnd);
            }
            return 1;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Context;
        if pointer.is_null() {
            return 0;
        }
        let context = &*pointer;
        if context
            .lease
            .as_ref()
            .is_some_and(|l| l.state() != LeaseState::Pending)
        {
            let _ = EndDialog(hwnd, 0);
            return 1;
        }
        match msg {
            WM_TIMER => {
                return 1;
            }
            WM_COMMAND => {
                let id = (wparam.0 & 0xffff) as i32;
                let notification = ((wparam.0 >> 16) & 0xffff) as u32;
                match id {
                    1 | DOWNLOAD_LATER => {
                        let mut result = context.draft.clone();
                        match context.mode {
                            Mode::Address => {
                                result.url = value(hwnd, URL_EDIT);
                                result.authorization = checked(hwnd, USE_AUTHORIZATION);
                                result.login = value(hwnd, LOGIN_EDIT);
                                result.password = value(hwnd, PASSWORD_EDIT);
                            }
                            Mode::Info => {
                                result.path = value(hwnd, PATH_EDIT);
                                result.category = value(hwnd, CATEGORY_COMBO);
                                result.description = value(hwnd, DESCRIPTION_EDIT);
                                result.remember = checked(hwnd, REMEMBER_PATH);
                                result.deferred = id == DOWNLOAD_LATER;
                            }
                            Mode::Options => {
                                result.show_completion_dialog = checked(hwnd, OPTION_COMPLETION);
                                result.path = value(hwnd, PATH_EDIT);
                                result.duplicate_choice = if checked(hwnd, DUPLICATE_ASK) {
                                    None
                                } else {
                                    let selected = item(hwnd, DUPLICATE_ACTION)
                                        .map(|combo| platform::send(combo, CB_GETCURSEL, 0, 0).0)
                                        .unwrap_or(2);
                                    Some(match selected {
                                        0 => DuplicateChoice::Numbered,
                                        1 => DuplicateChoice::Overwrite,
                                        _ => DuplicateChoice::Existing,
                                    })
                                };
                            }
                        }
                        *context.result.borrow_mut() = Some(result);
                        let _ = EndDialog(hwnd, 1);
                        return 1;
                    }
                    2 => {
                        let _ = EndDialog(hwnd, 0);
                        return 1;
                    }
                    USE_AUTHORIZATION => {
                        enable_authorization(hwnd);
                        return 1;
                    }
                    DUPLICATE_ASK => {
                        enable_duplicate_action(hwnd);
                        return 1;
                    }
                    CATEGORY_ADD => {
                        set_value(hwnd, CATEGORY_COMBO, "");
                        if let Ok(combo) = item(hwnd, CATEGORY_COMBO) {
                            let _ = SetFocus(Some(combo));
                        }
                        return 1;
                    }
                    CATEGORY_COMBO if matches!(notification, CBN_SELCHANGE | CBN_EDITCHANGE) => {
                        category_changed(hwnd, context, notification == CBN_SELCHANGE);
                        return 1;
                    }
                    PATH_EDIT
                        if matches!(context.mode, Mode::Info)
                            && matches!(notification, CBN_EDITCHANGE | CBN_SELENDOK) =>
                    {
                        update_path_hint(hwnd);
                        return 1;
                    }
                    BROWSE => {
                        match pick(hwnd, context.mode, Path::new(&value(hwnd, PATH_EDIT))) {
                            Ok(Some(path)) => {
                                set_value(hwnd, PATH_EDIT, &path.to_string_lossy());
                                if let Mode::Info = context.mode {
                                    update_path_hint(hwnd);
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                message(Some(hwnd), "Browse failed", &error.to_string(), false);
                            }
                        }
                        return 1;
                    }
                    _ => {}
                }
            }
            WM_CLOSE => {
                let _ = EndDialog(hwnd, 0);
                return 1;
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            _ => {}
        }
        0
    }
}
fn pick(owner: HWND, mode: Mode, current: &Path) -> Result<Option<PathBuf>, AppError> {
    unsafe {
        let dialog: IFileDialog = match mode {
            Mode::Options => {
                let open: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
                open.into()
            }
            _ => {
                let save: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
                save.into()
            }
        };
        let options =
            dialog.GetOptions()? | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR;
        dialog.SetOptions(if matches!(mode, Mode::Options) {
            options | FOS_PICKFOLDERS
        } else {
            options
        })?;
        dialog.SetTitle(if matches!(mode, Mode::Options) {
            w!("Choose the default download folder")
        } else {
            w!("Choose a new download filename")
        })?;
        if !matches!(mode, Mode::Options) {
            if let Some(name) = current.file_name() {
                let name = wide(name);
                dialog.SetFileName(PCWSTR(name.as_ptr()))?;
            }
        }
        let folder = if matches!(mode, Mode::Options) {
            current
        } else {
            current.parent().unwrap_or(current)
        };
        let folder = wide(folder);
        if let Ok(item) =
            SHCreateItemFromParsingName::<_, _, IShellItem>(PCWSTR(folder.as_ptr()), None)
        {
            let _ = dialog.SetFolder(&item);
        }
        if let Err(error) = dialog.Show(Some(owner)) {
            if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(None);
            }
            return Err(error.into());
        }
        let result = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        use std::os::windows::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_wide(result.as_wide()));
        CoTaskMemFree(Some(result.0.cast()));
        Ok(Some(path))
    }
}

struct DuplicateContext {
    display_url: String,
    initial: DuplicateChoice,
    result: RefCell<Option<(DuplicateChoice, bool)>>,
    lease: Option<CaptureLease>,
}

pub fn show_duplicate_with_lease(
    owner: HWND,
    display_url: &str,
    initial: DuplicateChoice,
    lease: Option<&CaptureLease>,
) -> Result<Option<(DuplicateChoice, bool)>, AppError> {
    let context = Box::new(DuplicateContext {
        display_url: display_url.to_owned(),
        initial,
        lease: lease.cloned(),
        result: RefCell::new(None),
    });
    let result = unsafe {
        DialogBoxParamW(
            Some(platform::instance()?),
            resource(DUPLICATE_DIALOG),
            Some(owner),
            Some(duplicate_proc),
            LPARAM((&*context as *const DuplicateContext) as isize),
        )
    };
    if result == -1 {
        return Err(windows::core::Error::from_thread().into());
    }
    Ok(context.result.into_inner())
}

unsafe extern "system" fn duplicate_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    std::panic::catch_unwind(|| unsafe {
        if msg == WM_INITDIALOG {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            let context = &*(lparam.0 as *const DuplicateContext);
            if context.lease.is_some() {
                SetTimer(Some(hwnd), 7, 250, None);
            }
            set_value(hwnd, DUPLICATE_URL, &context.display_url);
            let selected = match &context.initial {
                DuplicateChoice::Numbered => DUPLICATE_NUMBERED,
                DuplicateChoice::Overwrite => DUPLICATE_OVERWRITE,
                DuplicateChoice::Existing => DUPLICATE_EXISTING,
            };
            set_checked(hwnd, selected, true);
            if context.lease.is_some() {
                platform::bring_to_front(hwnd);
            }
            if let Ok(edit) = item(hwnd, DUPLICATE_URL) {
                let _ = SetFocus(Some(edit));
                platform::send(edit, EM_SETSEL, 0, -1);
            }
            return 0;
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const DuplicateContext;
        if pointer.is_null() {
            return 0;
        }
        if (&*pointer)
            .lease
            .as_ref()
            .is_some_and(|l| l.state() != LeaseState::Pending)
        {
            let _ = EndDialog(hwnd, 0);
            return 1;
        }
        match msg {
            WM_TIMER => 1,
            WM_COMMAND if (wparam.0 & 0xffff) == 1 => {
                let choice = if checked(hwnd, DUPLICATE_NUMBERED) {
                    DuplicateChoice::Numbered
                } else if checked(hwnd, DUPLICATE_OVERWRITE) {
                    DuplicateChoice::Overwrite
                } else {
                    DuplicateChoice::Existing
                };
                let context = &*pointer;
                // End the RefCell borrow before EndDialog sends destruction messages.
                *context.result.borrow_mut() = Some((choice, checked(hwnd, DUPLICATE_REMEMBER)));
                let _ = EndDialog(hwnd, 1);
                1
            }
            WM_COMMAND if (wparam.0 & 0xffff) == 2 => {
                let _ = EndDialog(hwnd, 0);
                1
            }
            WM_CLOSE => {
                let _ = EndDialog(hwnd, 0);
                1
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                0
            }
            _ => 0,
        }
    })
    .unwrap_or(0)
}

pub fn show_recovery_notice(owner: HWND, report: &str) -> Result<(), AppError> {
    let report = Box::new(report.to_owned());
    let result = unsafe {
        DialogBoxParamW(
            Some(platform::instance()?),
            resource(RECOVERY_DIALOG),
            Some(owner),
            Some(recovery_proc),
            LPARAM((&*report as *const String) as isize),
        )
    };
    if result == -1 {
        return Err(windows::core::Error::from_thread().into());
    }
    Ok(())
}
unsafe extern "system" fn recovery_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    std::panic::catch_unwind(|| unsafe {
        match msg {
            WM_INITDIALOG if lparam.0 != 0 => {
                let report = &*(lparam.0 as *const String);
                if let Ok(edit) = item(hwnd, RECOVERY_TEXT) {
                    platform::send(edit, EM_SETLIMITTEXT, 0x7fff_fffe, 0);
                    set_text(edit, report);
                }
                1
            }
            WM_COMMAND if matches!(wparam.0 & 0xffff, 1 | 2) => {
                let _ = EndDialog(hwnd, 1);
                1
            }
            WM_CLOSE => {
                let _ = EndDialog(hwnd, 1);
                1
            }
            _ => 0,
        }
    })
    .unwrap_or(0)
}

fn redact_url(address: &str) -> String {
    match url::Url::parse(address) {
        Ok(mut u) => {
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.set_query(None);
            u.set_fragment(None);
            u.into()
        }
        Err(_) => "Invalid address".into(),
    }
}
pub fn show_browser_download(
    owner: HWND,
    settings: &Settings,
    capture: &CaptureRequest,
    lease: &CaptureLease,
) -> Result<Option<AddedDownload>, AppError> {
    if lease.state() != LeaseState::Pending {
        return Ok(None);
    }
    let filename =
        crate::download_form::filename_hint(&capture.url, capture.filename_hint.as_deref())
            .map_err(|_| AppError("Invalid browser download.".into()))?;
    let draft = Draft {
        url: capture.url.clone(),
        path: download_form::folder_for(settings, "Automatic")
            .join(filename)
            .to_string_lossy()
            .into_owned(),
        category: "Automatic".into(),
        remember: true,
        ..Default::default()
    };
    finish_download(owner, settings, draft, Some(lease))
}
