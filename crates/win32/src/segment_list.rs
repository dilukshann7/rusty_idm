//! Native segment report list with accessible text and themed progress cells.
use crate::{
    commands::{SEGMENT_LIST, SEGMENT_TIMER},
    platform::{item, send, wide},
    transfer_view::{format_segments, SegmentChange, SegmentTableRows},
};
use rusty_engine::{JobSnapshot, SegmentState};
use std::{
    cell::{Cell, RefCell},
    time::Instant,
};
use windows::{
    core::{w, PWSTR},
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, RECT},
        Graphics::Gdi::{
            DrawTextW, FillRect, GetSysColor, GetSysColorBrush, InvalidateRect, RestoreDC, SaveDC,
            SetBkMode, SetTextColor, COLOR_HIGHLIGHT, COLOR_WINDOW, COLOR_WINDOWTEXT, DT_CENTER,
            DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, TRANSPARENT,
        },
        UI::{Controls::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub(super) struct SegmentList {
    rows: RefCell<SegmentTableRows>,
    epoch: Instant,
    timer_running: Cell<bool>,
}
impl Default for SegmentList {
    fn default() -> Self {
        Self {
            rows: RefCell::default(),
            epoch: Instant::now(),
            timer_running: Cell::new(false),
        }
    }
}
impl SegmentList {
    pub(super) fn initialize(&self, dialog: HWND) {
        let Ok(list) = item(dialog, SEGMENT_LIST) else {
            return;
        };
        send(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_LABELTIP) as isize,
        );
        let dpi = unsafe { GetDpiForWindow(dialog).max(96) };
        let scale = |value| (value * dpi / 96) as i32;
        // Widths fit the existing 360-DLU panel at its default dialog font. The
        // range column remains horizontally scrollable for very large offsets.
        for (index, (label, width)) in [
            ("Segment", 52),
            ("Byte range", 150),
            ("Progress", 102),
            ("Status", 83),
            ("Speed", 95),
        ]
        .iter()
        .enumerate()
        {
            let mut text = wide(label);
            let column = LVCOLUMNW {
                mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                cx: scale(*width),
                pszText: PWSTR(text.as_mut_ptr()),
                iSubItem: index as i32,
                ..Default::default()
            };
            send(list, LVM_INSERTCOLUMNW, index, &column as *const _ as isize);
        }
    }

    pub(super) fn update(&self, dialog: HWND, job: &JobSnapshot, expanded: bool) {
        let changes = self.rows.borrow_mut().sync(format_segments(job));
        if let Ok(list) = item(dialog, SEGMENT_LIST) {
            if !changes.is_empty() {
                // Keep row identity, selection, keyboard focus and scroll position.
                if expanded {
                    send(list, WM_SETREDRAW, 0, 0);
                }
                for change in changes {
                    match change {
                        SegmentChange::Remove(index) => {
                            send(list, LVM_DELETEITEM, index, 0);
                        }
                        SegmentChange::Insert(index) => {
                            let row = LVITEMW {
                                iItem: index as i32,
                                ..Default::default()
                            };
                            send(list, LVM_INSERTITEMW, 0, &row as *const _ as isize);
                        }
                        SegmentChange::SetCell { row, column } => {
                            let mut text = wide(&self.rows.borrow().rows[row].cells[column]);
                            let cell = LVITEMW {
                                iSubItem: column as i32,
                                pszText: PWSTR(text.as_mut_ptr()),
                                ..Default::default()
                            };
                            send(list, LVM_SETITEMTEXTW, row, &cell as *const _ as isize);
                        }
                    }
                }
                if expanded {
                    send(list, WM_SETREDRAW, 1, 0);
                }
                unsafe {
                    let _ = InvalidateRect(Some(list), None, false);
                }
            }
        }
        self.set_expanded(dialog, expanded);
    }

    pub(super) fn set_expanded(&self, dialog: HWND, expanded: bool) {
        let animate = expanded
            && self
                .rows
                .borrow()
                .rows
                .iter()
                .any(|row| row.progress.is_none() && row.state.is_active());
        if self.timer_running.replace(animate) != animate {
            unsafe {
                if animate {
                    SetTimer(Some(dialog), SEGMENT_TIMER, 80, None);
                } else {
                    let _ = KillTimer(Some(dialog), SEGMENT_TIMER);
                }
            }
        }
    }

    pub(super) fn animate(&self, dialog: HWND) {
        if let Ok(list) = item(dialog, SEGMENT_LIST) {
            unsafe {
                let _ = InvalidateRect(Some(list), None, false);
            }
        }
    }

    /// Returns a dialog notification result, to be stored in DWLP_MSGRESULT.
    pub(super) unsafe fn custom_draw(&self, lparam: LPARAM) -> Option<isize> {
        unsafe {
            let header = &*(lparam.0 as *const NMHDR);
            if header.idFrom != SEGMENT_LIST as usize || header.code != NM_CUSTOMDRAW {
                return None;
            }
            let draw = &*(lparam.0 as *const NMLVCUSTOMDRAW);
            let stage = draw.nmcd.dwDrawStage;
            if stage == CDDS_PREPAINT {
                return Some(CDRF_NOTIFYITEMDRAW as isize);
            }
            if stage == CDDS_ITEMPREPAINT {
                return Some(CDRF_NOTIFYSUBITEMDRAW as isize);
            }
            if stage.0 != (CDDS_ITEMPREPAINT.0 | CDDS_SUBITEM.0) || draw.iSubItem != 2 {
                return Some(CDRF_DODEFAULT as isize);
            }
            let rows = self.rows.borrow();
            let Some(row) = rows.rows.get(draw.nmcd.dwItemSpec) else {
                return Some(CDRF_DODEFAULT as isize);
            };
            let list = header.hwndFrom;
            let mut rect = RECT {
                left: LVIR_BOUNDS as i32,
                top: 2,
                ..Default::default()
            };
            if send(
                list,
                LVM_GETSUBITEMRECT,
                draw.nmcd.dwItemSpec,
                &mut rect as *mut _ as isize,
            )
            .0 == 0
            {
                return Some(CDRF_DODEFAULT as isize);
            }
            let selected = draw.nmcd.uItemState.0 & CDIS_SELECTED.0 != 0;
            let saved_dc = SaveDC(draw.nmcd.hdc);
            let color = if selected {
                COLOR_HIGHLIGHT
            } else {
                COLOR_WINDOW
            };
            FillRect(draw.nmcd.hdc, &rect, GetSysColorBrush(color));
            let inset = (2 * GetDpiForWindow(list).max(96) / 96) as i32;
            rect.left += inset;
            rect.right -= inset;
            rect.top += inset;
            rect.bottom -= inset;
            let theme = OpenThemeData(Some(list), w!("PROGRESS"));
            let themed = theme.0 != 0
                && DrawThemeBackground(theme, draw.nmcd.hdc, PP_BAR.0, 0, &rect, None).is_ok();
            if !themed {
                FillRect(draw.nmcd.hdc, &rect, GetSysColorBrush(COLOR_WINDOW));
            }
            let mut fill = rect;
            fill.left += 1;
            fill.right -= 1;
            fill.top += 1;
            fill.bottom -= 1;
            let width = (fill.right - fill.left).max(0);
            if let Some(percent) = row.progress {
                fill.right = fill.left + ((i64::from(width) * i64::from(percent)) / 10_000) as i32;
            } else if row.state.is_active() {
                let travel = (self.epoch.elapsed().as_millis() % 1400) as i32;
                let block = (width / 3).max(1);
                fill.left += (width + block) * travel / 1400 - block;
                fill.right = (fill.left + block).min(rect.right - 1);
                fill.left = fill.left.max(rect.left + 1);
            } else {
                fill.right = fill.left;
            }
            if fill.right > fill.left {
                let state = match row.state {
                    SegmentState::Failed => PBFS_ERROR.0,
                    SegmentState::Paused => PBFS_PAUSED.0,
                    _ => PBFS_NORMAL.0,
                };
                if !themed
                    || DrawThemeBackground(theme, draw.nmcd.hdc, PP_FILL.0, state, &fill, None)
                        .is_err()
                {
                    FillRect(draw.nmcd.hdc, &fill, GetSysColorBrush(COLOR_HIGHLIGHT));
                }
            }
            if theme.0 != 0 {
                let _ = CloseThemeData(theme);
            }
            SetBkMode(draw.nmcd.hdc, TRANSPARENT);
            SetTextColor(draw.nmcd.hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
            let mut text: Vec<u16> = row.cells[2].encode_utf16().collect();
            DrawTextW(
                draw.nmcd.hdc,
                &mut text,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            if saved_dc != 0 {
                let _ = RestoreDC(draw.nmcd.hdc, saved_dc);
            }
            Some(CDRF_SKIPDEFAULT as isize)
        }
    }
}
