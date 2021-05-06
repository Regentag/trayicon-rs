use super::bindings::{
    Windows::Win32::DisplayDevices::POINT,
    Windows::Win32::Gdi::HBRUSH,
    Windows::Win32::MenusAndResources::{HCURSOR, HICON, HMENU},
    Windows::Win32::SystemServices::{GetModuleHandleW, HINSTANCE, LRESULT, PWSTR},
    Windows::Win32::WindowsAndMessaging,
    Windows::Win32::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, GetCursorPos, GetWindowLongPtrW, RegisterClassW,
        RegisterWindowMessageW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW,
        CREATESTRUCTW, HWND, LPARAM, WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WINDOW_STYLE,
        WNDCLASSW, WNDCLASS_STYLES, WPARAM,
    },
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

type UINT = u32;
type DWORD = u32;
type WORD = u16;

#[inline]
#[allow(non_snake_case)]
fn LOWORD(l: DWORD) -> WORD {
    (l & 0xffff) as WORD
}

#[inline]
#[allow(non_snake_case)]
fn HIWORD(l: DWORD) -> WORD {
    ((l >> 16) & 0xffff) as WORD
}

use super::wchar::wchar;
use super::{msgs, winnotifyicon::WinNotifyIcon, MenuSys};
use crate::{trayiconsender::TrayIconSender, Error, Icon, MenuBuilder, TrayIconBase};

pub type WinTrayIcon<T> = WindowBox<T>;

/// WindowBox retains the memory for the Window object until WM_NCDESTROY
#[derive(Debug)]
pub struct WindowBox<T>(*mut WinTrayIconImpl<T>)
where
    T: PartialEq + Clone + 'static;

impl<T> Drop for WindowBox<T>
where
    T: PartialEq + Clone + 'static,
{
    fn drop(&mut self) {
        unsafe {
            // PostMessage doesn't seem to work here, because winit exits before it manages to be processed

            // https://devblogs.microsoft.com/oldnewthing/20110926-00/?p=9553
            SendMessageW(
                self.hwnd,
                WindowsAndMessaging::WM_CLOSE,
                WPARAM::default(),
                LPARAM::default(),
            );
        }
    }
}

impl<T> Deref for WindowBox<T>
where
    T: PartialEq + Clone + 'static,
{
    type Target = WinTrayIconImpl<T>;

    fn deref(&self) -> &WinTrayIconImpl<T> {
        unsafe { &mut *(self.0) }
    }
}

impl<T> DerefMut for WindowBox<T>
where
    T: PartialEq + Clone + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0) }
    }
}

/// Tray Icon WINAPI Window
///
/// In Windows the Tray Icon requires a window for message pump, it's not shown.
#[derive(Debug)]
pub struct WinTrayIconImpl<T>
where
    T: PartialEq + Clone + 'static,
{
    hwnd: HWND,
    sender: TrayIconSender<T>,
    menu: Option<MenuSys<T>>,
    notify_icon: WinNotifyIcon,
    on_click: Option<T>,
    on_double_click: Option<T>,
    on_right_click: Option<T>,
    msg_taskbarcreated: Option<UINT>,
}

unsafe impl<T> Send for WinTrayIconImpl<T> where T: PartialEq + Clone {}
unsafe impl<T> Sync for WinTrayIconImpl<T> where T: PartialEq + Clone {}

impl<T> WinTrayIconImpl<T>
where
    T: PartialEq + Clone + 'static,
{
    #[allow(clippy::new_ret_no_self)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        sender: TrayIconSender<T>,
        menu: Option<MenuSys<T>>,
        notify_icon: WinNotifyIcon,
        on_click: Option<T>,
        on_double_click: Option<T>,
        on_right_click: Option<T>,
    ) -> Result<WinTrayIcon<T>, Error>
    where
        T: PartialEq + Clone + 'static,
    {
        unsafe {
            let hinstance = HINSTANCE(GetModuleHandleW(PWSTR::default()));
            let mut wnd_class_name = wchar("TrayIconCls");
            let wnd_class = WNDCLASSW {
                style: WNDCLASS_STYLES::default(),
                lpfnWndProc: Some(WinTrayIconImpl::<T>::winproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PWSTR::NULL,
                lpszClassName: PWSTR(wnd_class_name.as_mut_ptr()),
            };
            RegisterClassW(&wnd_class);

            // Create window in a memory location that doesn't change
            let window = Box::new(WinTrayIconImpl {
                hwnd: HWND::default(),
                notify_icon,
                menu,
                on_click,
                on_right_click,
                on_double_click,
                sender,
                msg_taskbarcreated: None,
            });
            let ptr = Box::into_raw(window);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PWSTR(wnd_class_name.as_mut_ptr()),
                PWSTR(wchar("TrayIcon").as_mut_ptr()),
                WINDOW_STYLE::default(), //winuser::WS_OVERLAPPEDWINDOW | winuser::WS_VISIBLE,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                WindowsAndMessaging::CW_USEDEFAULT,
                HWND::default(),
                HMENU::default(),
                hinstance,
                ptr as *mut _,
            );

            if hwnd == HWND::default() {
                return Err(Error::OsError);
            }

            Ok(WindowBox(ptr))
        }
    }

    pub fn wndproc(&mut self, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WindowsAndMessaging::WM_CREATE => {
                // Create notification area icon
                self.notify_icon.add(self.hwnd);

                // Register to listen taskbar creation
                self.msg_taskbarcreated = unsafe {
                    Some(RegisterWindowMessageW(PWSTR(
                        wchar("TaskbarCreated\0").as_mut_ptr(),
                    )))
                };
            }

            // Mouse events on the tray icon
            msgs::WM_USER_TRAYICON => {
                match lparam.0 as UINT {
                    // Left click tray icon
                    WindowsAndMessaging::WM_LBUTTONUP => {
                        if let Some(e) = self.on_click.as_ref() {
                            self.sender.send(e);
                        }
                    }

                    // Right click tray icon
                    WindowsAndMessaging::WM_RBUTTONUP => {
                        // Send right click event
                        if let Some(e) = self.on_right_click.as_ref() {
                            self.sender.send(e);
                        }

                        // Show menu, if it's there
                        if let Some(menu) = &self.menu {
                            let mut pos = POINT { x: 0, y: 0 };
                            unsafe {
                                GetCursorPos(&mut pos as _);
                                SetForegroundWindow(self.hwnd);
                            }
                            menu.menu.track(self.hwnd, pos.x, pos.y);
                        }
                    }

                    // Double click tray icon
                    WindowsAndMessaging::WM_LBUTTONDBLCLK => {
                        if let Some(e) = self.on_double_click.as_ref() {
                            self.sender.send(e);
                        }
                    }
                    _ => {}
                }
            }

            // Any of the menu commands
            //
            // https://docs.microsoft.com/en-us/windows/win32/menurc/wm-command#parameters
            WindowsAndMessaging::WM_COMMAND => {
                let identifier = LOWORD(wparam.0 as u32);
                let cmd = HIWORD(wparam.0 as u32);

                // Menu command
                if cmd == 0 {
                    if let Some(v) = self.menu.as_ref() {
                        if let Some(event) = v.ids.get(&(identifier as usize)) {
                            self.sender.send(event);
                        }
                    }
                }
            }

            // TaskbarCreated
            x if Some(x) == self.msg_taskbarcreated => {
                self.notify_icon.add(self.hwnd);
            }

            // Default
            _ => {
                return unsafe { DefWindowProcW(self.hwnd, msg, wparam, lparam) };
            }
        }
        LRESULT(0)
    }

    // This serves as a conduit for actual winproc in the subproc
    pub extern "system" fn winproc(
        hwnd: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WindowsAndMessaging::WM_CREATE => {
                    let create_struct: &mut CREATESTRUCTW = &mut *(lparam.0 as *mut _);
                    // Arc::from_raw(ptr)
                    let window: &mut WinTrayIconImpl<T> =
                        &mut *(create_struct.lpCreateParams as *mut _);
                    window.hwnd = hwnd;
                    SetWindowLongPtrW(
                        hwnd,
                        WINDOW_LONG_PTR_INDEX::GWL_USERDATA,
                        window as *mut _ as _,
                    );
                    window.wndproc(msg, wparam, lparam)
                }
                WindowsAndMessaging::WM_NCDESTROY => {
                    let window_ptr =
                        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX::GWL_USERDATA, 0);
                    if window_ptr != 0 {
                        let ptr = window_ptr as *mut WinTrayIconImpl<T>;
                        let mut window = Box::from_raw(ptr);
                        window.wndproc(msg, wparam, lparam)
                    } else {
                        DefWindowProcW(hwnd, msg, wparam, lparam)
                    }
                }
                _ => {
                    let window_ptr = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX::GWL_USERDATA);
                    if window_ptr != 0 {
                        let window: &mut WinTrayIconImpl<T> = &mut *(window_ptr as *mut _);
                        window.wndproc(msg, wparam, lparam)
                    } else {
                        DefWindowProcW(hwnd, msg, wparam, lparam)
                    }
                }
            }
        }
    }
}

impl<T> TrayIconBase<T> for WinTrayIconImpl<T>
where
    T: PartialEq + Clone + 'static,
{
    /// Set the tooltip
    fn set_tooltip(&mut self, tooltip: &str) -> Result<(), Error> {
        if !self.notify_icon.set_tooltip(tooltip) {
            return Err(Error::OsError);
        }
        Ok(())
    }

    /// Set icon
    fn set_icon(&mut self, icon: &Icon) -> Result<(), Error> {
        if !self.notify_icon.set_icon(&icon.sys) {
            return Err(Error::IconLoadingFailed);
        }
        Ok(())
    }

    /// Set menu
    fn set_menu(&mut self, menu: &MenuBuilder<T>) -> Result<(), Error> {
        if menu.menu_items.is_empty() {
            self.menu = None
        } else {
            self.menu = Some(menu.build()?);
        }
        Ok(())
    }
}

impl<T> Drop for WinTrayIconImpl<T>
where
    T: PartialEq + Clone + 'static,
{
    fn drop(&mut self) {
        self.notify_icon.remove();
    }
}
