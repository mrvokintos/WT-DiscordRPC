use crate::config::Lang;

/// UI and presence strings, English then Russian.
pub fn tr(lang: Lang, key: &str) -> &'static str {
    let (en, ru) = match key {
        "hangar" => ("In hangar", "В ангаре"),
        "loading" => ("Loading...", "Загружается..."),
        "in_battle" => ("In battle", "В бою"),
        "kills" => ("Kills", "Уничтожено"),
        "wiki_button" => ("Vehicle on wiki", "Машина на вики"),
        "show_kills" => ("Kills in battle", "Счёт в бою"),
        "show_kills_no_nick" => ("Kills in battle (set nickname in config)", "Счёт в бою (укажи ник в конфиге)"),
        "show_nation" => ("Nation and vehicle class", "Нация и класс техники"),
        "battle_timer" => ("Battle timer", "Таймер боя"),
        "buttons" => ("Profile buttons", "Кнопки в профиле"),
        "extras" => ("Extras", "Дополнительно"),
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
        "logo_diamond" => ("Game icon", "Иконка игры"),
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

/// Tech tree name.
pub fn nation(lang: Lang, key: &str) -> Option<&'static str> {
    let (en, ru) = match key {
        "usa" => ("USA", "США"),
        "germany" => ("Germany", "Германия"),
        "ussr" => ("USSR", "СССР"),
        "britain" => ("Great Britain", "Великобритания"),
        "japan" => ("Japan", "Япония"),
        "china" => ("China", "Китай"),
        "italy" => ("Italy", "Италия"),
        "france" => ("France", "Франция"),
        "sweden" => ("Sweden", "Швеция"),
        "israel" => ("Israel", "Израиль"),
        _ => return None,
    };
    Some(if lang == Lang::Ru { ru } else { en })
}

/// Vehicle class from names.json.
pub fn class(lang: Lang, key: &str) -> Option<&'static str> {
    let (en, ru) = match key {
        "heavy_tank" => ("Heavy tank", "Тяжёлый танк"),
        "tank_destroyer" => ("Tank destroyer", "САУ"),
        "spaa" => ("SPAA", "ЗСУ"),
        "light_tank" => ("Light tank", "Лёгкий танк"),
        "medium_tank" => ("Medium tank", "Средний танк"),
        "attack_helicopter" => ("Attack helicopter", "Ударный вертолёт"),
        "utility_helicopter" => ("Utility helicopter", "Многоцелевой вертолёт"),
        "bomber" => ("Bomber", "Бомбардировщик"),
        "strike_aircraft" => ("Strike aircraft", "Штурмовик"),
        "assault" => ("Strike aircraft", "Штурмовик"),
        "fighter" => ("Fighter", "Истребитель"),
        "battleship" => ("Battleship", "Линкор"),
        "battlecruiser" => ("Battlecruiser", "Линейный крейсер"),
        "heavy_cruiser" => ("Heavy cruiser", "Тяжёлый крейсер"),
        "light_cruiser" => ("Light cruiser", "Лёгкий крейсер"),
        "destroyer" => ("Destroyer", "Эсминец"),
        "frigate" => ("Frigate", "Фрегат"),
        "submarine_chaser" => ("Submarine chaser", "Охотник за подлодками"),
        "torpedo_boat" => ("Torpedo boat", "Торпедный катер"),
        "gun_boat" => ("Gunboat", "Артиллерийский катер"),
        "armored_boat" => ("Armoured boat", "Бронекатер"),
        "barge" => ("Barge", "Баржа"),
        "boat" => ("Boat", "Катер"),
        _ => return None,
    };
    Some(if lang == Lang::Ru { ru } else { en })
}

/// Column of names.json for the language.
pub fn names_column(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "Russian",
        _ => "English",
    }
}
