use super::bindings::{
    Windows::Win32::MenusAndResources::HMENU,
    Windows::Win32::SystemServices::PWSTR,
    Windows::Win32::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, TrackPopupMenu, HWND, MENU_ITEM_FLAGS,
        TRACK_POPUP_MENU_FLAGS,
    },
};
use super::wchar::wchar;
use crate::Error;
use std::fmt::Debug;

/// Purpose of this struct is to keep hmenu handle, and drop it when the struct
/// is dropped
#[derive(Debug)]
pub struct WinHMenu {
    hmenu: HMENU,
    child_menus: Vec<WinHMenu>,
}

impl WinHMenu {
    pub(crate) fn new() -> Result<WinHMenu, Error> {
        Ok(WinHMenu {
            hmenu: unsafe {
                let res = CreatePopupMenu();
                if res.is_null() {
                    return Err(Error::OsError);
                }
                res
            },
            child_menus: vec![],
        })
    }

    pub fn add_menu_item(&self, name: &str, id: usize, disabled: bool) -> bool {
        let res = unsafe {
            AppendMenuW(
                self.hmenu,
                {
                    if disabled {
                        MENU_ITEM_FLAGS::MF_GRAYED
                    } else {
                        MENU_ITEM_FLAGS::MF_STRING
                    }
                },
                id,
                PWSTR(wchar(name).as_mut_ptr()),
            )
        };
        res.as_bool()
    }

    pub fn add_checkable_item(
        &self,
        name: &str,
        is_checked: bool,
        id: usize,
        disabled: bool,
    ) -> bool {
        let mut flags = if is_checked {
            MENU_ITEM_FLAGS::MF_CHECKED
        } else {
            MENU_ITEM_FLAGS::MF_UNCHECKED
        };

        if disabled {
            flags |= MENU_ITEM_FLAGS::MF_GRAYED
        }
        let res = unsafe { AppendMenuW(self.hmenu, flags, id, PWSTR(wchar(name).as_mut_ptr())) };
        res.as_bool()
    }
    pub fn add_child_menu(&mut self, name: &str, menu: WinHMenu, disabled: bool) -> bool {
        let mut flags = MENU_ITEM_FLAGS::MF_POPUP;
        if disabled {
            flags |= MENU_ITEM_FLAGS::MF_GRAYED
        }
        let res = unsafe {
            AppendMenuW(
                self.hmenu,
                flags,
                menu.hmenu.0 as usize,
                PWSTR(wchar(name).as_mut_ptr()),
            )
        };
        self.child_menus.push(menu);
        res.as_bool()
    }

    pub fn add_separator(&self) -> bool {
        let res = unsafe { AppendMenuW(self.hmenu, MENU_ITEM_FLAGS::MF_SEPARATOR, 0, PWSTR::NULL) };
        res.as_bool()
    }

    pub fn track(&self, hwnd: HWND, x: i32, y: i32) {
        unsafe {
            TrackPopupMenu(
                self.hmenu,
                TRACK_POPUP_MENU_FLAGS::default(),
                x,
                y,
                0,
                hwnd,
                std::ptr::null_mut(),
            )
        };
    }
}

unsafe impl Send for WinHMenu {}
unsafe impl Sync for WinHMenu {}

impl Drop for WinHMenu {
    fn drop(&mut self) {
        unsafe { DestroyMenu(self.hmenu) };
    }
}
