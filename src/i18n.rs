use crate::config::Lang;

/// UI and presence strings, English then Russian.
pub fn tr(lang: Lang, key: &str) -> &'static str {
    let (en, ru) = match key {
        "hangar" => ("In hangar", "В ангаре"),
        "loading" => ("Loading...", "Загружается..."),
        "in_battle" => ("In battle", "В бою"),
        "plays_on" => ("Plays on", "Играет на"),
        "tas" => ("TAS", "Скорость TAS"),
        "ias" => ("IAS", "Скорость IAS"),
        "radio_alt" => ("Radio alt.", "Радиовысота"),
        "abs_alt" => ("Altitude", "Высота"),
        "speed" => ("Speed", "Скорость"),
        "crew" => ("Crew", "Экипаж"),
        "kmh" => ("km/h", "км/ч"),
        "m" => ("m", "м"),
        "game_not_running" => ("Game is not running", "Игра не запущена"),
        "discord_not_running" => ("Discord is not connected", "Discord не подключён"),
        "show_indicators" => ("Show speed and altitude", "Показывать скорость и высоту"),
        "custom_images" => ("Custom vehicle images", "Свои картинки техники"),
        "logo" => ("Logo", "Логотип"),
        "logo_red" => ("Red", "Красный"),
        "logo_white" => ("White", "Белый"),
        "language" => ("Language", "Язык"),
        "lang_auto" => ("System", "Как в системе"),
        "air_speed" => ("Aircraft speed", "Скорость самолёта"),
        "altitude" => ("Altitude", "Высота"),
        "alt_radio" => ("Radio", "Радиовысота"),
        "alt_abs" => ("Absolute", "Абсолютная"),
        "autostart" => ("Start with system", "Запускать вместе с системой"),
        "open_config" => ("Open config folder", "Открыть папку с настройками"),
        "update_names" => ("Update vehicle names", "Обновить названия техники"),
        "new_version" => ("New version available", "Доступна новая версия"),
        "quit" => ("Quit", "Выход"),
        _ => {
            debug_assert!(false, "missing string {key}");
            ("?", "?")
        }
    };
    match lang {
        Lang::Ru => ru,
        _ => en,
    }
}

/// Column of names.json for the language.
pub fn names_column(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Russian",
        _ => "English",
    }
}
