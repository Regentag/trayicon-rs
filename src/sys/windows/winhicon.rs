use super::bindings::{
    Windows::Win32::Controls::IMAGE_FLAGS,
    Windows::Win32::MenusAndResources::HICON,
    Windows::Win32::SystemServices::TRUE,
    Windows::Win32::WindowsAndMessaging::{
        CopyIcon, CreateIconFromResourceEx, DestroyIcon, LookupIconIdFromDirectoryEx,
    },
};
use crate::{Error, IconBase};

/// Purpose of this struct is to keep hicon handle, and drop it when the struct
/// is dropped
pub struct WinHIcon {
    pub hicon: HICON,
}

impl IconBase for WinHIcon {
    fn from_buffer(
        buffer: &'static [u8],
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<WinHIcon, Error> {
        let offset = unsafe {
            LookupIconIdFromDirectoryEx(
                buffer.as_ptr() as _,
                TRUE,
                width.unwrap_or_default() as i32,
                height.unwrap_or_default() as i32,
                IMAGE_FLAGS::LR_DEFAULTCOLOR,
            )
        };
        if offset <= 0 {
            return Err(Error::IconLoadingFailed);
        }
        let icon_data = &buffer[offset as usize..];
        let hicon = unsafe {
            CreateIconFromResourceEx(
                icon_data.as_ptr() as _,
                icon_data.len() as u32,
                TRUE,
                0x30000,
                width.unwrap_or_default() as i32,
                height.unwrap_or_default() as i32,
                IMAGE_FLAGS::LR_DEFAULTCOLOR,
            )
        };
        if hicon.is_null() {
            return Err(Error::IconLoadingFailed);
        }
        Ok(WinHIcon { hicon })
    }
}

impl Clone for WinHIcon {
    fn clone(&self) -> Self {
        WinHIcon {
            hicon: unsafe { CopyIcon(self.hicon) },
        }
    }
}

unsafe impl Send for WinHIcon {}
unsafe impl Sync for WinHIcon {}

impl Drop for WinHIcon {
    fn drop(&mut self) {
        unsafe { DestroyIcon(self.hicon) };
    }
}
