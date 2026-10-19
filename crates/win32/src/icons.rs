//! Original transparent toolbar artwork, embedded at several DPI resolutions.
use crate::{
    commands::*,
    controls::scale,
    platform::{self, resource, send},
    AppError,
};
use windows::Win32::{
    Foundation::HWND,
    UI::{Controls::*, WindowsAndMessaging::*},
};

pub struct ToolbarButton {
    pub command: u16,
    pub label: &'static str,
    normal: u16,
    disabled: u16,
}
pub const BUTTONS: [ToolbarButton; 5] = [
    ToolbarButton {
        command: ADD,
        label: "Add URL",
        normal: 301,
        disabled: 306,
    },
    ToolbarButton {
        command: STOP,
        label: "Stop",
        normal: 302,
        disabled: 307,
    },
    ToolbarButton {
        command: STOP_ALL,
        label: "Stop All",
        normal: 303,
        disabled: 308,
    },
    ToolbarButton {
        command: REMOVE,
        label: "Delete Entry",
        normal: 304,
        disabled: 309,
    },
    ToolbarButton {
        command: OPTIONS,
        label: "Options",
        normal: 305,
        disabled: 310,
    },
];

struct ImageList(HIMAGELIST);
impl ImageList {
    fn new(size: i32) -> Result<Self, AppError> {
        let list = unsafe { ImageList_Create(size, size, ILC_COLOR32 | ILC_MASK, 5, 0) };
        if list.is_invalid() {
            return Err(AppError("Cannot create toolbar icon images.".into()));
        }
        Ok(Self(list))
    }
    fn add(&self, id: u16, size: i32) -> Result<(), AppError> {
        unsafe {
            // Without LR_SHARED this HICON is owned; the image list copies it.
            let image = LoadImageW(
                Some(platform::instance()?),
                resource(id),
                IMAGE_ICON,
                size,
                size,
                LR_DEFAULTCOLOR,
            )?;
            let icon = HICON(image.0);
            let added = ImageList_ReplaceIcon(self.0, -1, icon);
            let _ = DestroyIcon(icon);
            if added < 0 {
                return Err(AppError("Cannot load a toolbar icon.".into()));
            }
        }
        Ok(())
    }
}
impl Drop for ImageList {
    fn drop(&mut self) {
        unsafe {
            let _ = ImageList_Destroy(Some(self.0));
        }
    }
}

pub struct ToolbarImages {
    normal: ImageList,
    disabled: ImageList,
    dpi: u32,
    toolbar: Option<HWND>,
}
impl ToolbarImages {
    pub fn new(dpi: u32) -> Result<Self, AppError> {
        let size = scale(24, dpi);
        let images = Self {
            normal: ImageList::new(size)?,
            disabled: ImageList::new(size)?,
            dpi,
            toolbar: None,
        };
        for button in &BUTTONS {
            images.normal.add(button.normal, size)?;
            images.disabled.add(button.disabled, size)?;
        }
        Ok(images)
    }
    pub fn attach(&mut self, toolbar: HWND) {
        let size = scale(24, self.dpi);
        send(toolbar, TB_SETBITMAPSIZE, 0, packed(size, size));
        send(toolbar, TB_SETIMAGELIST, 0, self.normal.0 .0);
        send(toolbar, TB_SETDISABLEDIMAGELIST, 0, self.disabled.0 .0);
        send(toolbar, TB_SETMAXTEXTROWS, 1, 0);
        send(
            toolbar,
            TB_SETBUTTONSIZE,
            0,
            packed(scale(68, self.dpi), scale(48, self.dpi)),
        );
        self.toolbar = Some(toolbar);
    }
}
impl Drop for ToolbarImages {
    fn drop(&mut self) {
        if let Some(toolbar) = self.toolbar {
            if unsafe { IsWindow(Some(toolbar)).as_bool() } {
                // A DPI replacement may already have installed different lists.
                // Detach only our own handles before their owners destroy them.
                if send(toolbar, TB_GETIMAGELIST, 0, 0).0 == self.normal.0 .0 {
                    send(toolbar, TB_SETIMAGELIST, 0, 0);
                }
                if send(toolbar, TB_GETDISABLEDIMAGELIST, 0, 0).0 == self.disabled.0 .0 {
                    send(toolbar, TB_SETDISABLEDIMAGELIST, 0, 0);
                }
            }
        }
    }
}
fn packed(width: i32, height: i32) -> isize {
    ((width as u32 & 0xffff) | ((height as u32 & 0xffff) << 16)) as isize
}
