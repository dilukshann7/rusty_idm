use crate::{
    commands::*,
    icons::{ToolbarImages, BUTTONS, TOOLBAR_HEIGHT},
    platform::{self, send, wide, Font},
    presenter::{self, Category, ViewModel},
    AppError,
};
use rusty_engine::JobId;
use std::{collections::BTreeMap, mem::size_of};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::{ClientToScreen, ScreenToClient},
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
    navigation: Vec<(Category, String, HTREEITEM)>,
    shell: crate::shell_icons::ShellImages,
    navigation_generation: usize,
    navigation_visuals: BTreeMap<isize, (i32, String)>,
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
            send(
                tree,
                TVM_SETEXTENDEDSTYLE,
                TVS_EX_DOUBLEBUFFER as usize,
                TVS_EX_DOUBLEBUFFER as isize,
            );
            send(toolbar, TB_BUTTONSTRUCTSIZE, size_of::<TBBUTTON>(), 0);
            let mut images = ToolbarImages::new(dpi)?;
            images.attach(toolbar);
            send(
                toolbar,
                TB_SETEXTENDEDSTYLE,
                0,
                (TBSTYLE_EX_DRAWDDARROWS | TBSTYLE_EX_DOUBLEBUFFER) as isize,
            );
            for (image, definition) in BUTTONS.iter().enumerate() {
                let mut label = wide(definition.label);
                label.push(0); // TB_ADDSTRINGW takes a double-NUL string list.
                let index = send(toolbar, TB_ADDSTRINGW, 0, label.as_ptr() as isize).0;
                let button = TBBUTTON {
                    iBitmap: image as i32,
                    idCommand: definition.command as i32,
                    fsState: TBSTATE_ENABLED as u8,
                    fsStyle: (BTNS_BUTTON
                        | BTNS_SHOWTEXT
                        | if matches!(definition.command, QUEUE_START | QUEUE_STOP) {
                            BTNS_DROPDOWN
                        } else {
                            0
                        }) as u8,
                    iString: index,
                    ..Default::default()
                };
                if send(toolbar, TB_ADDBUTTONSW, 1, &button as *const _ as isize).0 == 0 {
                    return Err(AppError("Cannot create toolbar buttons.".into()));
                }
            }
            images.size_buttons(toolbar);
            let _ = ShowWindow(toolbar, SW_SHOWNOACTIVATE);
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
            let shell = crate::shell_icons::ShellImages::new(owner, tree, list, dpi)?;
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
                categories: vec![],
                navigation: vec![],
                shell,
                navigation_generation: 0,
                navigation_visuals: BTreeMap::new(),
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
            let top = scale(TOOLBAR_HEIGHT, self.dpi);
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
        self.shell.invalidate(dpi)?;
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
            self.categories.first().map_or(0, |h| h.0),
        );
    }
    /// Select the row under the pointer; empty space must not reuse an old row.
    /// Keyboard invocation anchors the menu beneath the selected row.
    pub fn context_target(&self, mut screen: POINT) -> Option<(JobId, POINT)> {
        let row = if screen.x == -1 && screen.y == -1 {
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
            send(self.list, LVM_ENSUREVISIBLE, row as usize, 0);
            let mut rect = RECT {
                left: LVIR_BOUNDS as i32,
                ..Default::default()
            };
            if send(
                self.list,
                LVM_GETITEMRECT,
                row as usize,
                &mut rect as *mut _ as isize,
            )
            .0 == 0
            {
                return None;
            }
            screen = POINT {
                x: rect.left + scale(16, self.dpi),
                y: rect.bottom,
            };
            if !unsafe { ClientToScreen(self.list, &mut screen) }.as_bool() {
                return None;
            }
            row
        } else {
            let mut client = screen;
            if !unsafe { ScreenToClient(self.list, &mut client) }.as_bool() {
                return None;
            }
            let mut hit = LVHITTESTINFO {
                pt: client,
                ..Default::default()
            };
            let row = send(
                self.list,
                LVM_SUBITEMHITTEST,
                0,
                &mut hit as *mut _ as isize,
            )
            .0;
            if row < 0 {
                return None;
            }
            row
        };
        let id = *self.rows.get(row as usize)?;
        let mut item = LVITEMW {
            stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
            ..Default::default()
        };
        send(
            self.list,
            LVM_SETITEMSTATE,
            usize::MAX,
            &item as *const _ as isize,
        );
        item.state = item.stateMask;
        send(
            self.list,
            LVM_SETITEMSTATE,
            row as usize,
            &item as *const _ as isize,
        );
        Some((id, screen))
    }
    /// Pin the queue under the pointer before TrackPopupMenu dispatches messages.
    pub fn queue_context_target(
        &self,
        mut screen: POINT,
    ) -> Option<(Option<rusty_engine::QueueId>, POINT)> {
        let handle = if screen.x == -1 && screen.y == -1 {
            let handle = HTREEITEM(send(self.tree, TVM_GETNEXTITEM, TVGN_CARET as usize, 0).0);
            if handle.0 == 0 {
                return None;
            }
            let mut rect = RECT::default();
            unsafe {
                std::ptr::write_unaligned((&mut rect as *mut RECT).cast::<HTREEITEM>(), handle);
            }
            if send(self.tree, TVM_GETITEMRECT, 1, &mut rect as *mut _ as isize).0 == 0 {
                return None;
            }
            screen = POINT {
                x: rect.left + scale(8, self.dpi),
                y: rect.bottom,
            };
            if !unsafe { ClientToScreen(self.tree, &mut screen) }.as_bool() {
                return None;
            }
            handle
        } else {
            let mut point = screen;
            if !unsafe { ScreenToClient(self.tree, &mut point) }.as_bool() {
                return None;
            }
            let mut hit = TVHITTESTINFO {
                pt: point,
                ..Default::default()
            };
            let handle = HTREEITEM(send(self.tree, TVM_HITTEST, 0, &mut hit as *mut _ as isize).0);
            if handle.0 == 0 {
                return None;
            }
            handle
        };
        let mut node = TVITEMW {
            mask: TVIF_PARAM,
            hItem: handle,
            ..Default::default()
        };
        if send(self.tree, TVM_GETITEMW, 0, &mut node as *mut _ as isize).0 == 0 {
            return None;
        }
        let queue = match self.category_for(node.lParam.0)? {
            Category::Queue(id) => Some(id),
            Category::QueueRoot => None,
            _ => return None,
        };
        send(self.tree, TVM_SELECTITEM, TVGN_CARET as usize, handle.0);
        Some((queue, screen))
    }
    pub fn refresh(&mut self, owner: HWND, view: &ViewModel, category: Category, closing: bool) {
        self.navigation(view, category);
        self.shell.drain();
        let selected = self.selected();
        let visible = view.visible_jobs(category);
        if self.rows != visible {
            send(self.list, WM_SETREDRAW, 0, 0);
            send(self.list, LVM_DELETEALLITEMS, 0, 0);
            self.rows = visible;
            self.cached.clear();
            for (row, id) in self.rows.iter().enumerate() {
                let job = &view.jobs[id];
                let columns = row_columns(view, job);
                let mut name = wide(&columns[0]);
                // Store an index into this stable current row->JobId vector, never a truncated u64.
                let item = LVITEMW {
                    mask: LVIF_TEXT | LVIF_PARAM | LVIF_IMAGE,
                    iImage: self.shell.index(crate::shell_icons::ShellImages::file(job)),
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
                let columns = row_columns(view, &view.jobs[id]);
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
        for (row, id) in self.rows.iter().enumerate() {
            let image = self
                .shell
                .index(crate::shell_icons::ShellImages::file(&view.jobs[id]));
            let item = LVITEMW {
                mask: LVIF_IMAGE,
                iItem: row as i32,
                iImage: image,
                ..Default::default()
            };
            let mut current = LVITEMW {
                mask: LVIF_IMAGE,
                iItem: row as i32,
                ..Default::default()
            };
            if send(self.list, LVM_GETITEMW, 0, &mut current as *mut _ as isize).0 == 0
                || current.iImage != image
            {
                send(self.list, LVM_SETITEMW, 0, &item as *const _ as isize);
            }
        }
        for (category, label, handle) in &self.navigation {
            let key = crate::shell_icons::ShellImages::stock(match category {
                Category::Filtered(_, kind) => {
                    Some(rusty_engine::categories::BUILTIN[*kind as usize])
                }
                _ => None,
            });
            let image = self.shell.index(key);
            let caption = if let Category::Queue(id) = category {
                view.queues
                    .get(id)
                    .map(|q| format!("{} ({}/{})", q.name, q.completed, q.entries.len()))
                    .unwrap_or_else(|| label.clone())
            } else {
                label.clone()
            };
            if self
                .navigation_visuals
                .get(&handle.0)
                .is_some_and(|old| old.0 == image && old.1 == caption)
            {
                continue;
            }
            let mut text = wide(&caption);
            let item = TVITEMW {
                mask: TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_TEXT,
                pszText: PWSTR(text.as_mut_ptr()),
                hItem: *handle,
                iImage: image,
                iSelectedImage: image,
                ..Default::default()
            };
            send(self.tree, TVM_SETITEMW, 0, &item as *const _ as isize);
            self.navigation_visuals.insert(handle.0, (image, caption));
        }
        self.actions(owner, view, closing);
        let queue_status =
            if let Category::Queue(id) = category {
                view.queues
                    .get(&id)
                    .map(|q| {
                        format!(
                        "  |  {}: {}  |  {} active / {} completed / {} entries  |  parallelism {}",
                        q.name,
                        if q.error.is_some(){"Held: queue error"}else if q.stopping {
                            "Stopping"
                        } else if q.running {
                            "Running"
                        } else {
                            "Held"
                        },
                        q.active,
                        q.completed,
                        q.entries.len(),
                        q.parallelism
                    )
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
        let text = wide(format!(
            "{} downloads  |  {} shown  |  {}/s{}{}",
            view.jobs.len(),
            self.rows.len(),
            presenter::format_bytes(view.aggregate_speed()),
            queue_status,
            if closing {
                "  |  Stopping transfers…"
            } else {
                ""
            }
        ));
        send(self.status, SB_SETTEXTW, 0, text.as_ptr() as isize);
    }
    pub fn category_for(&self, index: isize) -> Option<Category> {
        let token = index as usize;
        if token >> 16 != self.navigation_generation {
            return None;
        }
        self.navigation.get(token & 0xffff).map(|n| n.0)
    }
    pub fn invalidate_associations(&mut self) -> Result<(), AppError> {
        self.shell.invalidate(self.dpi)
    }
    pub fn poll_icons(&mut self, owner: HWND, view: &ViewModel, category: Category, closing: bool) {
        self.refresh(owner, view, category, closing);
    }
    fn navigation(&mut self, view: &ViewModel, selected: Category) {
        let mut entries = vec![];
        for (status, parent) in [
            (0, Category::All),
            (1, Category::Unfinished),
            (2, Category::Finished),
        ] {
            entries.push((parent, parent.label().to_owned()));
            for kind in 0..5 {
                entries.push((
                    Category::Filtered(status, kind),
                    rusty_engine::categories::BUILTIN[kind as usize].to_owned(),
                ));
            }
            if status == 0 {
                let custom: std::collections::BTreeSet<_> = view
                    .jobs
                    .values()
                    .filter_map(|j| j.category.as_deref())
                    .chain(view.categories.iter().map(|r| r.name.as_str()))
                    .filter(|c| {
                        !rusty_engine::categories::BUILTIN
                            .iter()
                            .any(|b| b.eq_ignore_ascii_case(c))
                            && !c.eq_ignore_ascii_case("Automatic")
                            && !c.eq_ignore_ascii_case("General")
                    })
                    .collect();
                for name in custom {
                    entries.push((
                        Category::Custom(presenter::category_key(name)),
                        name.to_owned(),
                    ));
                }
            }
        }
        entries.push((Category::QueueRoot, "Queues".into()));
        for q in view.queues.values() {
            entries.push((Category::Queue(q.id), q.name.clone()));
        }
        if self
            .navigation
            .iter()
            .map(|(c, n, _)| (*c, n.clone()))
            .collect::<Vec<_>>()
            == entries
        {
            return;
        }
        let expansion: Vec<_> = self
            .navigation
            .iter()
            .filter(|(c, _, _)| {
                matches!(
                    c,
                    Category::All | Category::Unfinished | Category::Finished | Category::QueueRoot
                )
            })
            .map(|(c, _, h)| {
                (
                    *c,
                    send(
                        self.tree,
                        TVM_GETITEMSTATE,
                        h.0 as usize,
                        TVIS_EXPANDED.0 as isize,
                    )
                    .0 != 0,
                )
            })
            .collect();
        self.navigation_generation = self.navigation_generation.wrapping_add(1).max(1);
        send(self.tree, WM_SETREDRAW, 0, 0);
        send(self.tree, TVM_DELETEITEM, 0, TVI_ROOT.0);
        self.navigation.clear();
        self.navigation_visuals.clear();
        self.categories.clear();
        let mut parent = TVI_ROOT;
        for (index, (category, label)) in entries.into_iter().enumerate() {
            let root = matches!(
                category,
                Category::All | Category::Unfinished | Category::Finished | Category::QueueRoot
            );
            let Ok(handle) = (unsafe {
                insert_tree(
                    self.tree,
                    if root { TVI_ROOT } else { parent },
                    &label,
                    ((self.navigation_generation << 16) | index) as isize,
                )
            }) else {
                continue;
            };
            if root {
                parent = handle;
                self.categories.push(handle);
            }
            if root {
                send(self.tree, TVM_EXPAND, TVE_EXPAND.0 as usize, handle.0);
            }
            self.navigation.push((category, label, handle));
        }
        for (category, _, handle) in &self.navigation {
            if expansion.iter().find(|(c, _)| c == category).map_or(
                matches!(category, Category::All | Category::QueueRoot),
                |(_, expanded)| *expanded,
            ) {
                send(self.tree, TVM_EXPAND, TVE_EXPAND.0 as usize, handle.0);
            }
        }
        if let Some((_, _, handle)) = self
            .navigation
            .iter()
            .find(|n| n.0 == selected)
            .or_else(|| self.navigation.first())
        {
            send(self.tree, TVM_SELECTITEM, TVGN_CARET as usize, handle.0);
        }
        send(self.tree, WM_SETREDRAW, 1, 0);
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(self.tree), None, false);
        }
    }
    pub fn queue_button_anchor(&self, command: u16) -> Option<POINT> {
        let mut rect = RECT::default();
        if send(
            self.toolbar,
            TB_GETRECT,
            command as usize,
            &mut rect as *mut _ as isize,
        )
        .0 == 0
        {
            return None;
        }
        let mut point = POINT {
            x: rect.left,
            y: rect.bottom,
        };
        unsafe { ClientToScreen(self.toolbar, &mut point) }
            .as_bool()
            .then_some(point)
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
            (DELETE_COMPLETED, !closing),
            (QUEUE_SCHEDULE, !view.queues.is_empty()),
            (QUEUE_START, !view.queues.is_empty()),
            (QUEUE_STOP, !view.queues.is_empty()),
            (TELL_FRIEND, !closing),
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
            WS_CHILD
                | WS_TABSTOP
                | WINDOW_STYLE(style)
                | if id == TOOLBAR {
                    WINDOW_STYLE(0)
                } else {
                    WS_VISIBLE
                },
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

fn row_columns(view: &ViewModel, job: &rusty_engine::JobSnapshot) -> [String; 7] {
    let mut columns = presenter::format_snapshot(job).columns;
    if let Some(queue) = job.queue_id.and_then(|q| view.queues.get(&q)) {
        let order = job
            .entry_id
            .and_then(|e| queue.entries.iter().position(|id| *id == e))
            .map_or(0, |i| i + 1);
        columns[6] = format!("{} #{} | {}", queue.name, order, columns[6]);
    }
    columns
}
