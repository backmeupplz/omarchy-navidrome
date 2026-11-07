# omarchy-navidrome

![Navidrome panel](preview.png)

Navidrome (or any Subsonic server) in the Omarchy bar. Browse albums, play an album or a track, shuffle songs from the albums you've checked, or play endless lo-fi hip hop generated on your machine.

- Plays through a headless `mpv`, so media keys and MPRIS work through Omarchy's `mpv-mpris`
- The library loads once when the shell starts and stays in memory. Search filters on each keystroke. The list only renders visible rows.
- Shuffle picks songs evenly at random across the checked albums. It skips recent repeats and queues the next track early so there's no gap. Every album starts checked.
- The checked albums are mirrored to a playlist called **Shuffle Selection** on the server, updated whenever you check or uncheck an album, so other Subsonic clients can shuffle the same pool. Needs Navidrome (it lists the library with an empty `search3`).
- Lo-fi: six vibes (Chill, Rainy, Sunny, Late, Jazz, Study), each track generated from scratch by a small Rust renderer (`lofi/`), a port of the generator on [vancouver.borodutch.com](https://vancouver.borodutch.com). A track takes a few seconds to render; the next one renders while the current plays. Tracks are WAVs in `~/.cache/omarchy-navidrome` (the last few are kept).
- Stores only the salted Subsonic token, never your password, in `~/.config/omarchy/navidrome.json` (mode 600)

## Requirements

- `mpv` (plays audio; preinstalled on Omarchy)
- `mpv-mpris` (optional, ships with Omarchy) for media keys and the media widget

## Install

```bash
omarchy plugin add https://github.com/backmeupplz/omarchy-navidrome.git --enable
```

Click the ♫ icon in the bar, then enter your server URL, username and password.

## Remove

```bash
omarchy plugin remove backmeupplz.navidrome
rm ~/.config/omarchy/navidrome.json   # saved server, token and album selection
rm -r ~/.cache/omarchy-navidrome      # rendered lo-fi tracks
```

## Keys (panel open)

| Key | Action |
|-----|--------|
| `j`/`k`, arrows | move |
| `Enter`/`l` | open album, or play a track |
| `Esc`/`h` | back, or close |
| `Space` | check or uncheck an album for shuffle |
| `/` | search |
| `s` | shuffle checked albums |
| `f` | play lo-fi (last vibe) |
| `p` / `n` / `b` | pause / next / previous |
| `x` | check or uncheck the playing song's album |
| `r` | reload library |

Middle-click the bar icon to pause.

## Keybinds

```bash
omarchy-shell backmeupplz.navidrome toggle   # open/close the panel
omarchy-shell navidrome shuffle
omarchy-shell navidrome lofi jazz            # chill, rainy, sunny, late, jazz, study
omarchy-shell navidrome toggle               # play/pause
omarchy-shell navidrome next
omarchy-shell navidrome previous
omarchy-shell navidrome status               # JSON
```

## Development

`node test.js` runs the shuffle-picking check. `cargo test --release` in `lofi/` checks the renderer against the web version's track generator.

`lofi/lofi` is a prebuilt static x86-64 binary so the plugin works without a Rust toolchain. Rebuild it after changing `lofi/src`:

```bash
cd lofi && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release && cp target/release/lofi lofi
```
 The service has `keepLoaded`, so edits to `Service.qml` need `omarchy restart shell`.

## License

MIT
