mod bindings {
    windows::include_bindings!();
}

use bindings::Windows::Win32::WindowsAndMessaging::{
    DispatchMessageA, PeekMessageA, TranslateMessage, WaitMessage, HWND, MSG,
    PEEK_MESSAGE_REMOVE_TYPE, WM_QUIT,
};
use std::time::Duration;
use trayicon::*;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Events {
    ClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    Item1,
    Item2,
    Item3,
    Item4,
    CheckItem1,
    SubItem1,
    SubItem2,
    SubItem3,
}

fn main() {
    let (s, r) = std::sync::mpsc::channel::<Events>();
    let icon = include_bytes!("../../../src/testresource/icon1.ico");
    let icon2 = include_bytes!("../../../src/testresource/icon2.ico");

    let second_icon = Icon::from_buffer(icon2, None, None).unwrap();
    let first_icon = Icon::from_buffer(icon, None, None).unwrap();

    // Needlessly complicated tray icon with all the whistles and bells
    let mut tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip("Cool Tray 👀 Icon")
        .on_click(Events::ClickTrayIcon)
        .on_double_click(Events::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("Item 3 Replace Menu 👍", Events::Item3)
                .item("Item 2 Change Icon Green", Events::Item2)
                .item("Item 1 Change Icon Red", Events::Item1)
                .separator()
                .checkable("This is checkable", true, Events::CheckItem1)
                .submenu(
                    "Sub Menu",
                    MenuBuilder::new()
                        .item("Sub item 1", Events::SubItem1)
                        .item("Sub Item 2", Events::SubItem2)
                        .item("Sub Item 3", Events::SubItem3),
                )
                .with(MenuItem::Item {
                    name: "Item Disabled".into(),
                    disabled: true, // Disabled entry example
                    id: Events::Item4,
                    icon: None,
                })
                .separator()
                .item("E&xit", Events::Exit),
        )
        .build()
        .unwrap();

    let mut msg = MSG::default();
    let h_wnd = HWND::default();
    let d = Duration::from_nanos(1);

    // Your applications message loop.
    'event: loop {
        unsafe {
            while PeekMessageA(&mut msg, h_wnd, 0, 0, PEEK_MESSAGE_REMOVE_TYPE::PM_REMOVE).as_bool()
            {
                if msg.message == WM_QUIT {
                    break 'event;
                } else {
                    TranslateMessage(&msg);
                    DispatchMessageA(&msg);
                }
            }

            // handle tray icon event
            let tray_msg = r.recv_timeout(d);
            if let Ok(ev) = tray_msg {
                match ev {
                    Events::DoubleClickTrayIcon => {
                        println!("Double click");
                    }
                    Events::ClickTrayIcon => {
                        println!("Single click");
                    }
                    Events::Exit => {
                        println!("Please exit");
                        break 'event;
                    }
                    Events::Item1 => {
                        tray_icon.set_icon(&second_icon).unwrap();
                    }
                    Events::Item2 => {
                        tray_icon.set_icon(&first_icon).unwrap();
                    }
                    Events::Item3 => {
                        tray_icon
                            .set_menu(
                                &MenuBuilder::new()
                                    .item("New menu item", Events::Item1)
                                    .item("Exit", Events::Exit),
                            )
                            .unwrap();
                    }
                    e => {
                        println!("{:?}", e);
                    }
                }
            } else {
                WaitMessage();
            }
        }
    }
}
