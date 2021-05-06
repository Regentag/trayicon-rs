fn main() {
    windows::build!(
        Windows::Win32::Controls::*,
        Windows::Win32::DisplayDevices::POINT,
        Windows::Win32::Gdi::HBRUSH,
        Windows::Win32::MenusAndResources::*,
        Windows::Win32::Shell::*,
        Windows::Win32::SystemServices::*,
        Windows::Win32::WindowsAndMessaging::*
    );
}
