use crate::{
    commands::*,
    icons::{ToolbarImages, BUTTONS},
    platform::{self, send, wide, Font},
    presenter::{self, Category, ViewModel},
    AppError,
};
use rusty_engine::JobId;
use std::{collections::BTreeMap, mem::size_of};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::{Controls::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub struct Controls {
    pub toolbar: HWND,
    pub tree: HWND,
    pub list: HWND,
    pub status: HWND,
    pub rows: Vec<JobId>,
    cached: BTreeMap<JobId, [String; 7]>,
    font: Font,
    images: ToolbarImages,
    dpi: u32,
    categories: Vec<HTREEITEM>,
}
impl Controls {
    pub fn new(owner: HWND) -> Result<Self, AppError> {
        unsafe {
            let dpi = GetDpiForWindow(owner).max(96);
            let font = Font::new(dpi)?;
            let toolbar = child(
                owner,
                TOOLBARCLASSNAMEW,
                "Download commands",
                TOOLBAR,
                TBSTYLE_FLAT | TBSTYLE_TOOLTIPS | CCS_NORESIZE as u32 | CCS_NOPARENTALIGN as u32,
                false,
            )?;
            let tree = child(
                owner,
                WC_TREEVIEWW,
                "Download categories",
                TREE,
                TVS_HASBUTTONS | TVS_HASLINES | TVS_LINESATROOT | TVS_SHOWSELALWAYS,
                true,
            )?;
            let list = child(
                owner,
                WC_LISTVIEWW,
                "Downloads",
                LIST,
                LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS,
                true,
            )?;
            let status = child(
                owner,
                STATUSCLASSNAMEW,
                "Download status",
                STATUS,
                SBARS_SIZEGRIP,
                false,
            )?;
            for hwnd in [toolbar, tree, list, status] {
                font.apply(hwnd);
            }
            send(
                list,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                0,
                (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_LABELTIP) as isize,
            );
            send(toolbar, TB_BUTTONSTRUCTSIZE, size_of::<TBBUTTON>(), 0);
            let mut images = ToolbarImages::new(dpi)?;
            images.attach(toolbar);
            for (image, definition) in BUTTONS.iter().enumerate() {
                let mut label = wide(definition.label);
                label.push(0); // TB_ADDSTRINGW takes a double-NUL string list.
                let index = send(toolbar, TB_ADDSTRINGW, 0, label.as_ptr() as isize).0;
                let button = TBBUTTON {
                    iBitmap: image as i32,
                    idCommand: definition.command as i32,
                    fsState: TBSTATE_ENABLED as u8,
                    fsStyle: (BTNS_BUTTON | BTNS_SHOWTEXT) as u8,
                    iString: index,
                    ..Default::default()
                };
                if send(toolbar, TB_ADDBUTTONSW, 1, &button as *const _ as isize).0 == 0 {
                    return Err(AppError("Cannot create toolbar buttons.".into()));
                }
            }
            let root = insert_tree(tree, TVI_ROOT, "Categories", -1)?;
            let mut categories = vec![];
            for (index, category) in Category::ALL.iter().enumerate() {
                categories.push(insert_tree(tree, root, category.label(), index as isize)?);
            }
            send(tree, TVM_EXPAND, TVE_EXPAND.0 as usize, root.0);
            send(tree, TVM_SELECTITEM, TVGN_CARET as usize, categories[0].0);
            for (index, (label, width)) in [
                ("File Name", 230),
                ("Size", 90),
                ("Status", 145),
                ("Time Left", 65),
                ("Transfer Rate", 95),
                ("Last Try Date", 140),
                ("Description", 250),
            ]
            .iter()
            .enumerate()
            {
                let mut text = wide(label);
                let column = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                    cx: scale(*width, dpi),
                    pszText: PWSTR(text.as_mut_ptr()),
                    iSubItem: index as i32,
                    ..Default::default()
                };
                if send(list, LVM_INSERTCOLUMNW, index, &column as *const _ as isize).0 < 0 {
                    return Err(AppError("Cannot create download columns.".into()));
                }
            }
            Ok(Self {
                toolbar,
                tree,
                list,
                status,
                rows: vec![],
                cached: BTreeMap::new(),
                font,
                images,
                dpi,
                categories,
            })
        }
    }
    pub fn layout(&self, owner: HWND) {
        unsafe {
            let mut client = RECT::default();
            if GetClientRect(owner, &mut client).is_err() {
                return;
            }
            let width = client.right.max(1);
            let height = client.bottom.max(1);
            let top = scale(54, self.dpi);
            let bottom = scale(20, self.dpi);
            let nav = scale(140, self.dpi).min(width / 2);
            let _ = MoveWindow(self.toolbar, 0, 0, width, top, true);
            let _ = MoveWindow(self.tree, 0, top, nav, (height - top - bottom).max(1), true);
            let _ = MoveWindow(
                self.list,
                nav + scale(2, self.dpi),
                top,
                (width - nav - scale(2, self.dpi)).max(1),
                (height - top - bottom).max(1),
                true,
            );
            send(self.status, WM_SIZE, 0, 0);
        }
    }
    pub fn change_dpi(&mut self, dpi: u32) -> Result<(), AppError> {
        let font = Font::new(dpi)?;
        let mut images = ToolbarImages::new(dpi)?;
        for control in [self.toolbar, self.tree, self.list, self.status] {
            font.apply(control);
        }
        for index in 0..7 {
            let width = send(self.list, LVM_GETCOLUMNWIDTH, index, 0).0;
            send(
                self.list,
                LVM_SETCOLUMNWIDTH,
                index,
                width * dpi as isize / self.dpi as isize,
            );
        }
        self.font = font;
        images.attach(self.toolbar);
        self.images = images;
        self.dpi = dpi;
        send(self.toolbar, TB_AUTOSIZE, 0, 0);
        Ok(())
    }
    pub fn selected(&self) -> Option<JobId> {
        let row = send(
            self.list,
            LVM_GETNEXTITEM,
            usize::MAX,
            LVNI_SELECTED as isize,
        )
        .0;
        if row < 0 {
            return None;
        }
        let mut item = LVITEMW {
            mask: LVIF_PARAM,
            iItem: row as i32,
            ..Default::default()
        };
        if send(self.list, LVM_GETITEMW, 0, &mut item as *mut _ as isize).0 == 0 {
            return None;
        }
        self.rows.get(item.lParam.0 as usize).copied()
    }
    pub fn select_all_category(&self) {
        send(
            self.tree,
            TVM_SELECTITEM,
            TVGN_CARET as usize,
            self.categories[0].0,
        );
    }
    pub fn refresh(&mut self, owner: HWND, view: &ViewModel, category: Category, closing: bool) {
        let selected = self.selected();
        let visible = view.visible_jobs(category);
        if self.rows != visible {
            send(self.list, WM_SETREDRAW, 0, 0);
            send(self.list, LVM_DELETEALLITEMS, 0, 0);
            self.rows = visible;
            self.cached.clear();
            for (row, id) in self.rows.iter().enumerate() {
                let job = &view.jobs[id];
                let columns = presenter::format_snapshot(job).columns;
                let mut name = wide(&columns[0]);
                // Store an index into this stable current row->JobId vector, never a truncated u64.
                let item = LVITEMW {
                    mask: LVIF_TEXT | LVIF_PARAM,
                    iItem: row as i32,
                    pszText: PWSTR(name.as_mut_ptr()),
                    lParam: LPARAM(row as isize),
                    ..Default::default()
                };
                send(self.list, LVM_INSERTITEMW, 0, &item as *const _ as isize);
                for (column, text) in columns.iter().enumerate().skip(1) {
                    set_cell(self.list, row, column, text);
                }
                if Some(*id) == selected {
                    let item = LVITEMW {
                        state: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
                        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
                        ..Default::default()
                    };
                    send(self.list, LVM_SETITEMSTATE, row, &item as *const _ as isize);
                }
                self.cached.insert(*id, columns);
            }
            send(self.list, WM_SETREDRAW, 1, 0);
            unsafe {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(self.list), None, true);
            }
        } else {
            for (row, id) in self.rows.iter().enumerate() {
                let columns = presenter::format_snapshot(&view.jobs[id]).columns;
                if let Some(old) = self.cached.get(id) {
                    for column in 0..7 {
                        if old[column] != columns[column] {
                            set_cell(self.list, row, column, &columns[column]);
                        }
                    }
                }
                self.cached.insert(*id, columns);
            }
        }
        self.actions(owner, view, closing);
        let text = wide(format!(
            "{} downloads  |  {} shown  |  {}/s{}",
            view.jobs.len(),
            self.rows.len(),
            presenter::format_bytes(view.aggregate_speed()),
            if closing {
                "  |  Stopping transfers…"
            } else {
                ""
            }
        ));
        send(self.status, SB_SETTEXTW, 0, text.as_ptr() as isize);
    }
    pub fn actions(&self, owner: HWND, view: &ViewModel, closing: bool) {
        let actions = view.actions(self.selected());
        for (id, enabled) in [
            (START, actions.start),
            (ADD, !closing),
            (STOP, actions.stop),
            (STOP_ALL, actions.stop_all),
            (REMOVE, actions.remove),
            (RETRY, actions.retry),
            (OPEN_FILE, actions.open_file),
            (OPEN_FOLDER, actions.open_folder),
            (DETAILS, actions.details),
            (OPTIONS, !closing),
        ] {
            let enabled = enabled && !closing;
            send(self.toolbar, TB_ENABLEBUTTON, id as usize, enabled as isize);
            unsafe {
                let menu = GetMenu(owner);
                let _ = EnableMenuItem(
                    menu,
                    id as u32,
                    if enabled { MF_ENABLED } else { MF_GRAYED },
                );
            }
        }
    }
}
impl Drop for Controls {
    fn drop(&mut self) {
        // Detach owned fonts before deletion even on a startup/error path.
        unsafe {
            let stock = windows::Win32::Graphics::Gdi::GetStockObject(
                windows::Win32::Graphics::Gdi::DEFAULT_GUI_FONT,
            );
            for hwnd in [self.toolbar, self.tree, self.list, self.status] {
                if IsWindow(Some(hwnd)).as_bool() {
                    send(hwnd, WM_SETFONT, stock.0 as usize, 0);
                }
            }
        }
    }
}
pub fn scale(value: i32, dpi: u32) -> i32 {
    ((value as i64 * dpi as i64 + 48) / 96) as i32
}
unsafe fn child(
    owner: HWND,
    class: PCWSTR,
    name: &str,
    id: u16,
    style: u32,
    border: bool,
) -> Result<HWND, AppError> {
    let name = wide(name);
    Ok(unsafe {
        CreateWindowExW(
            if border {
                WS_EX_CLIENTEDGE
            } else {
                WINDOW_EX_STYLE(0)
            },
            class,
            PCWSTR(name.as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(style),
            0,
            0,
            0,
            0,
            Some(owner),
            Some(HMENU(id as usize as *mut _)),
            Some(platform::instance()?),
            None,
        )?
    })
}
unsafe fn insert_tree(
    tree: HWND,
    parent: HTREEITEM,
    label: &str,
    category: isize,
) -> Result<HTREEITEM, AppError> {
    let mut text = wide(label);
    let item = TVINSERTSTRUCTW {
        hParent: parent,
        hInsertAfter: TVI_LAST,
        Anonymous: TVINSERTSTRUCTW_0 {
            item: TVITEMW {
                mask: TVIF_TEXT | TVIF_PARAM,
                pszText: PWSTR(text.as_mut_ptr()),
                lParam: LPARAM(category),
                ..Default::default()
            },
        },
    };
    let handle = send(tree, TVM_INSERTITEMW, 0, &item as *const _ as isize).0;
    if handle == 0 {
        return Err(AppError("Cannot create navigation categories.".into()));
    }
    Ok(HTREEITEM(handle))
}
fn set_cell(list: HWND, row: usize, column: usize, text: &str) {
    let mut text = wide(text);
    let item = LVITEMW {
        iSubItem: column as i32,
        pszText: PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    send(list, LVM_SETITEMTEXTW, row, &item as *const _ as isize);
}
