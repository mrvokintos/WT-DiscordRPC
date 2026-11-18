// StatusNotifierItem tray (KDE, GNOME with the AppIndicator extension, most other desktops).

use super::Menu;
use crate::app::App;
use ksni::blocking::TrayMethods;
use ksni::menu::*;
use std::time::Duration;

struct Tray {
    app: App,
}

fn convert(items: Vec<super::Item>) -> Vec<ksni::MenuItem<Tray>> {
    use super::Item;
    items
        .into_iter()
        .map(|it| match it {
            Item::Label(label) => StandardItem { label, enabled: false, ..Default::default() }.into(),
            Item::Action(label, action) => StandardItem {
                label,
                activate: Box::new(move |t: &mut Tray| super::run(&t.app, action)),
                ..Default::default()
            }
            .into(),
            Item::Check(label, checked, action) => CheckmarkItem {
                label,
                checked,
                activate: Box::new(move |t: &mut Tray| super::run(&t.app, action)),
                ..Default::default()
            }
            .into(),
            Item::Radio(label, selected, options) => SubMenu {
                label,
                submenu: vec![RadioGroup {
                    selected,
                    select: Box::new({
                        let actions: Vec<_> = options.iter().map(|(_, a)| *a).collect();
                        move |t: &mut Tray, i| {
                            if let Some(a) = actions.get(i) {
                                super::run(&t.app, *a)
                            }
                        }
                    }),
                    options: options.into_iter().map(|(l, _)| RadioItem { label: l, ..Default::default() }).collect(),
                }
                .into()],
                ..Default::default()
            }
            .into(),
            Item::Sub(label, items) => SubMenu { label, submenu: convert(items), ..Default::default() }.into(),
            Item::Separator => MenuItem::Separator,
        })
        .collect()
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "wt-discord-rpc".into()
    }
    fn title(&self) -> String {
        "WT Discord RPC".into()
    }
    fn icon_name(&self) -> String {
        "applications-games".into()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: "WT Discord RPC".into(), description: super::status_line(&self.app), ..Default::default() }
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        convert(Menu::build(&self.app))
    }
}

pub fn run(app: App) -> Result<(), String> {
    let handle = Tray { app: app.clone() }.spawn().map_err(|e| e.to_string())?;
    loop {
        if app.with(|s| s.quit) {
            handle.shutdown().wait();
            return Ok(());
        }
        // status and settings can change outside the menu: rebuild it
        handle.update(|_| {});
        std::thread::sleep(Duration::from_millis(1000));
    }
}
