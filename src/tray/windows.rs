// Windows notification area icon via tray-icon; needs a Win32 message loop on this thread.

use super::{Action, Item, Menu};
use crate::app::App;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu as TMenu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIconBuilder};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE};

/// 32×32 red disc: the game's colour, no image file needed.
fn icon() -> Icon {
    let n = 32usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            if dx * dx + dy * dy <= 15.0 * 15.0 {
                rgba[(y * n + x) * 4..][..4].copy_from_slice(&[200, 30, 30, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, n as u32, n as u32).expect("icon")
}

fn make(it: Item, ids: &mut HashMap<MenuId, Action>) -> Box<dyn IsMenuItem> {
    match it {
        Item::Label(l) => Box::new(MenuItem::new(l, false, None)),
        Item::Action(l, a) => {
            let m = MenuItem::new(l, true, None);
            ids.insert(m.id().clone(), a);
            Box::new(m)
        }
        Item::Check(l, checked, a) => {
            let m = CheckMenuItem::new(l, true, checked, None);
            ids.insert(m.id().clone(), a);
            Box::new(m)
        }
        Item::Radio(l, selected, options) => {
            let sub = Submenu::new(l, true);
            for (i, (label, a)) in options.into_iter().enumerate() {
                let m = CheckMenuItem::new(label, true, i == selected, None);
                ids.insert(m.id().clone(), a);
                let _ = sub.append(&m);
            }
            Box::new(sub)
        }
        Item::Sub(l, items) => {
            let sub = Submenu::new(l, true);
            for it in items {
                let _ = sub.append(make(it, ids).as_ref());
            }
            Box::new(sub)
        }
        Item::Separator => Box::new(PredefinedMenuItem::separator()),
    }
}

fn build(app: &App) -> (TMenu, HashMap<MenuId, Action>) {
    let menu = TMenu::new();
    let mut ids = HashMap::new();
    for it in Menu::build(app) {
        let _ = menu.append(make(it, &mut ids).as_ref());
    }
    (menu, ids)
}

pub fn run(app: App) -> Result<(), String> {
    let (menu, mut ids) = build(&app);
    let tray = TrayIconBuilder::new()
        .with_tooltip("WT Discord RPC")
        .with_icon(icon())
        .with_menu(Box::new(menu))
        .build()
        .map_err(|e| e.to_string())?;
    let mut last_rebuild = Instant::now();
    loop {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let mut changed = false;
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if let Some(a) = ids.get(&ev.id).copied() {
                super::run(&app, a);
                changed = true;
            }
        }
        if app.with(|s| s.quit) {
            return Ok(());
        }
        if changed || last_rebuild.elapsed() > Duration::from_secs(2) {
            let (menu, new_ids) = build(&app);
            tray.set_menu(Some(Box::new(menu)));
            let _ = tray.set_tooltip(Some(format!("WT Discord RPC\n{}", super::status_line(&app))));
            ids = new_ids;
            last_rebuild = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
