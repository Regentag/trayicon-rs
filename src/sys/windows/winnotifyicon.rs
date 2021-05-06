use super::bindings::{
    Windows::Win32::Shell,
    Windows::Win32::Shell::{Shell_NotifyIconW, NOTIFYICONDATAW},
    Windows::Win32::WindowsAndMessaging::HWND,
};
use super::{msgs, wchar::wchar_array, winhicon::WinHIcon};
use std::fmt::Debug;

/// Purpose of this struct is to retain NotifyIconDataW and remove it on drop
pub struct WinNotifyIcon {
    winhicon: WinHIcon,
    nid: NOTIFYICONDATAW,
}

impl WinNotifyIcon {
    pub fn new(winhicon: &WinHIcon, tooltip: &Option<String>) -> WinNotifyIcon {
        static mut ICON_ID: u32 = 1000;
        unsafe {
            ICON_ID += 1;
        }
        let mut icon = WinNotifyIcon {
            winhicon: winhicon.clone(),
            nid: unsafe { std::mem::zeroed() },
        };
        if let Some(tooltip) = tooltip {
            wchar_array(tooltip, icon.nid.szTip.as_mut());
        }
        icon.nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        icon.nid.uID = unsafe { ICON_ID };
        icon.nid.uCallbackMessage = msgs::WM_USER_TRAYICON;
        icon.nid.hIcon = icon.winhicon.hicon;
        icon.nid.uFlags = Shell::NIF_MESSAGE | Shell::NIF_ICON | Shell::NIF_TIP;

        icon
    }
}

impl WinNotifyIcon {
    pub fn add(&mut self, hwnd: HWND) -> bool {
        self.nid.hWnd = hwnd;
        let res = unsafe { Shell_NotifyIconW(Shell::NIM_ADD, &mut self.nid) };
        res.as_bool()
    }

    pub fn remove(&mut self) -> bool {
        let res = unsafe { Shell_NotifyIconW(Shell::NIM_DELETE, &mut self.nid) };
        res.as_bool()
    }

    pub fn set_icon(&mut self, winhicon: &WinHIcon) -> bool {
        self.winhicon = winhicon.clone();
        self.nid.hIcon = self.winhicon.hicon;
        let res = unsafe { Shell_NotifyIconW(Shell::NIM_MODIFY, &mut self.nid) };
        res.as_bool()
    }

    pub fn set_tooltip(&mut self, tooltip: &str) -> bool {
        wchar_array(tooltip, self.nid.szTip.as_mut());
        let res = unsafe { Shell_NotifyIconW(Shell::NIM_MODIFY, &mut self.nid) };
        res.as_bool()
    }
}
unsafe impl Send for WinNotifyIcon {}
unsafe impl Sync for WinNotifyIcon {}

impl Debug for WinNotifyIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TrayIcon")
    }
}

impl Drop for WinNotifyIcon {
    fn drop(&mut self) {
        unsafe {
            Shell_NotifyIconW(Shell::NIM_DELETE, &mut self.nid);
        }
    }
}
