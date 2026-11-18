# WT Discord RPC

**English** | [Русский](README.ru.md)

Discord Rich Presence for War Thunder on Linux and Windows. It shows whether you are in the hangar, loading or in a battle, the vehicle you are playing with its picture, and optionally its speed and altitude, or its speed and crew.

Inspired by [WarThunderDiscordRPC](https://github.com/ledxdeliveryflopp/WarThunderDiscordRPC) by ledxdeliveryflopp. This is a separate implementation in Rust that shares no code with it.

The program only reads the game's local API (`localhost:8111`, the same one the browser map uses). It doesn't touch game files or memory.

## Installation

Download the archive for your system from [Releases](https://github.com/mrvokintos/WT-DiscordRPC/releases), unpack it and run `wt-discord-rpc`. An icon appears in the tray. Use the "Start with system" item to launch the program automatically at login.

On Linux the tray uses StatusNotifierItem. KDE supports it out of the box; GNOME needs the AppIndicator extension. If there is no tray, the program keeps running without one. `--no-tray` turns the tray off on purpose.

Discord installed natively, as a Flatpak (including Vesktop) or as a Snap is found automatically.

## Settings

Most settings are in the tray menu. All of them are also stored in `config.toml`, which "Open config folder" opens: `~/.config/wt-discord-rpc/` on Linux, `%APPDATA%\WT Discord RPC\` on Windows. If a `config.toml` sits next to the executable, the program uses that folder instead (portable mode).

| Key | Default | Meaning |
|---|---|---|
| `client_id` | the project's application | Discord application id (see below) |
| `lang` | `auto` | `auto`, `ru` or `en`: interface and vehicle name language |
| `logo` | `main_white` | `main_white`, `main_red` or `main_diamond` (the game icon) |
| `show_indicators` | `true` | speed, altitude and crew in the second line |
| `air_speed` | `IAS` | `IAS` or `TAS` |
| `altitude` | `RADIO` | `RADIO` (falls back to absolute when unavailable) or `ABSOLUTE` |
| `custom_images` | `true` | use the application's own image for a vehicle when one exists |
| `show_kills` | `false` | your kills in the current battle, from the kill feed (needs `nickname`) |
| `nickname` | empty | your in-game nickname without the squadron tag, case-sensitive |
| `show_nation` | `false` | nation and vehicle class under the small image; with its flag (the application's `flag_<nation>` image if uploaded, otherwise the one from `assets/flags`) |
| `battle_timer` | `false` | in a battle the timer counts from the battle's start, in the hangar from the game's start |
| `wiki_button` | `false` | "Vehicle on wiki" button while on a vehicle; Discord shows buttons to others, not to you |
| `repo_button` | `false` | "WT Discord RPC" button linking to this project |
| `api_url` | `http://localhost:8111` | game API address (the `WT_API_URL` environment variable overrides it) |
| `names_url` | WT-Vehicles-Parser `db-latest` | where vehicle names are updated from; empty disables updates |
| `check_updates` | `true` | check for a new version on start |

## Vehicle names

Names come from [WT Vehicles Parser](https://github.com/mrvokintos/WT-Vehicles-Parser), which reads them from the game's own localisation (`units.csv`), so they match the tech tree. A copy is built in. A newer one is downloaded automatically after a game patch; "Update vehicle names" in the tray forces a download.

## Your own Discord application

The name shown in Discord ("Playing War Thunder") and the logos come from a Discord application. To use your own:

1. Create an application named "War Thunder" at <https://discord.com/developers/applications>.
2. Optionally, under Rich Presence → Art Assets, upload your own logos as `main_white`, `main_red` and `main_diamond` (otherwise the ones in `assets/logo` are used). You can also upload vehicle pictures named by vehicle id (for example `us_m1a1_aim_abrams`); with `custom_images` on, they replace the encyclopedia picture.
3. Put the Application ID into `client_id`.

Flags work out of the box (`assets/flags` in this repository). To use your own, upload images named `flag_usa`, `flag_germany`, `flag_ussr`, `flag_britain`, `flag_japan`, `flag_china`, `flag_italy`, `flag_france`, `flag_sweden`, `flag_israel`. The game's own flags are in the datamine: `atlases.vromfs.bin_u/gameuiskin/country_<nation>_round.svg`.

## Building

```bash
cargo build --release
```

No system libraries are needed on Linux.

## License

[GPL-3.0](LICENSE)
