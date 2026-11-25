# CLAUDE.md – Honeydew (Fork)

Orientierung für Claude. Der/die Besitzer:in des Forks hat keine Programmierkenntnisse: Änderungen in einfachem
Deutsch erklären, Schritt für Schritt vorgehen, nichts Großes ungefragt ändern. Code und Kommentare bleiben englisch
(wie im Upstream-Projekt).

## Was die App ist

Honeydew ist ein freies Musikstudio (DAW) nach dem Vorbild von FL Studio Mobile, geschrieben in **Rust**
(GPL-3.0-or-later). Es läuft auf Windows, Linux, macOS, Android, iOS (experimentell) und im Browser (WebAssembly).
Upstream: `daijro/honeydew`. Version aktuell 0.5.5. Es enthält keinen Code/Samples von FL Studio Mobile (Clean-Room).

Funktionen: Playlist (Arrangement), Piano Roll, Drum-/Step-Editor, Mixer, Effekt-Racks (~45 Effekte), viele
eingebaute Instrumente, Automation, Akkord-Editor, Audio-Editor/Aufnahme, MIDI-Ein-/Ausgabe, VST3/VST2/CLAP-Plugins
(nur Desktop), Lemon-Studio-Plugins (eingebaut), Import/Export von `.flm`, `.flp`, MIDI, ZIP, Export als
WAV/FLAC/OGG/MP3, Sound-Bibliothek (~740 MB, separat), experimentelle MCP-Steuerung (`tools/honeydew-mcp`).

## Grobe Architektur

Zwei Schichten, die strikt getrennt sind:

1. **Song-Modell** (`src/song/`) – reine Daten: `Song` → `Channel`s → `Rack` (Module) + `Lane`s (Clips/Events).
   Alles `Clone + Debug + PartialEq`, keine UI-Typen. Entspricht dem Projektdateiformat. `PPQ = 128` Ticks pro Beat.
2. **UI** (egui/eframe, immediate mode) in `src/ui/`, `src/panels/`, `src/app.rs`.
   - `HoneydewApp` (`src/app.rs`, ~6700 Zeilen) ist die Hauptstruktur; `update()` läuft jeden Frame.
   - Views/Overlays (Piano Roll, Mixer, Settings, …) liegen in `src/ui/views/`, ihre Zeichen-Logik in `src/ui/*`.
   - Ein View ändert den Song direkt und **meldet die Änderung** über `AppCtx::song_edited(SongEdit::…)`
     (`src/ui/views/mod.rs`). Die Art (`Final`, `Ongoing`, `Param`, `Replaced`, …) entscheidet, ob ein Undo-Schritt
     entsteht und ob die Audio-Engine neu gebaut wird. **Das ist die wichtigste Regel für neue Features.**
3. **Audio** – `crates/audio` (`honeydew-audio`, ~35k Zeilen): Synths, Effekte, Mixer, Sequencer, Sample-Decoder,
   Export-Encoder. Echtzeit-Thread: **dort nicht allozieren/blockieren** (Test `tests/rt_alloc.rs`).
   `src/audio_engine/` baut aus einem `Song` eine `SongAudioEngine`; `src/audio_link.rs` schickt Commands an die
   Audio-Queue (Handover: Fresh / Edit / Undo / Quiet / Preview).

## Verzeichnisse

| Pfad | Inhalt |
|---|---|
| `src/song/` | Song-Datenmodell (channel, lane, rack, tempo, scales, chords, section) |
| `src/params/` | Parameterbeschreibungen (Instrumente, Effekte, Channel-Parameter, Anzeige) |
| `src/panels/inst/`, `src/panels/fx/` | Rack-Karten je Instrument (MiniSynth, DW Sampler, GM Synth, SuperSaw, 3x Osc, …) und je Effekt |
| `src/ui/` | Alle Bildschirme/Editoren: `main_screen.rs`, `piano_roll/`, `step_seq/`, `clips.rs` (Playlist), `mixer.rs`, `rack.rs`, `main_menu/`, `views/` |
| `src/app/` | Teile von `HoneydewApp`: Tasten, MIDI, Export, Audio-Edit, Automation, Updates, Selbsttest |
| `src/flm/`, `src/flp/`, `src/smf/`, `src/project_files/`, `src/zip/` | Dateiformate lesen/schreiben (FLM, FLP, MIDI, Archive) |
| `src/content/` | Sound-/Preset-/Template-Bundles, Demo-Songs, Akkord-Sets |
| `src/plugins/` | VST3/VST2/CLAP-Hosting (Desktop), eingebaute Lemon-Plugins (`builtin.rs`) |
| `src/midi/`, `src/recording/` | MIDI-Geräte, Audioaufnahme |
| `src/settings.rs`, `src/keymap.rs`, `src/theme.rs` | Einstellungen, Tastenbelegung (`assets/content/keymaps/*.json`), Themes |
| `src/history.rs` | Undo/Redo (Song-Snapshots) |
| `src/android*.rs`, `src/ios.rs`, `android/` | Mobile-Plattformcode (Java-Teil in `android/app/src/main/java`) |
| `crates/audio/` | Audio-Engine-Bibliothek (siehe oben) |
| `vendor/` | Mitgelieferte Fremdcode-Kopien (Lemon Studio, nih-plug, winit, rusty_mp3) – **nicht ändern** |
| `content/recipes/` | TOML-Rezepte, aus denen die Sound-Bibliothek gebaut wird (`tools/content-build`) |
| `sounds/` | Sound-Bibliothek in 90-MB-Teilen (nicht anfassen) |
| `tests/` | Integrationstests (Engine, FLM-Format, Plugins, Fuzz, …); viele Unit-Tests liegen als `tests.rs` neben dem Code |
| `tools/` | Build-/Release-Skripte, MCP-Adapter |

Plattform-Schalter kommen aus `build.rs`: `desktop`, `mobile`, `plugin_host`, `x11`, `native_midi`, `multi_core`
(plus `target_arch = "wasm32"` fürs Web). Neuer Code muss auf **allen** Plattformen kompilieren (Web hat z. B. keine
Dateidialoge, keine Plugins, keine Threads wie Desktop).

## Befehle

```sh
cargo run --release                       # Desktop-App starten (Binary: honeydew-native)
cargo fmt --check                         # Formatierung prüfen (cargo fmt zum Korrigieren)
cargo clippy --workspace --all-targets -- -D warnings   # Lint, Warnungen sind Fehler
cargo test --workspace                    # alle Tests
cargo check --workspace --target wasm32-unknown-unknown # Web-Build muss kompilieren
trunk serve                               # Web-Version lokal (braucht trunk + wasm32-Target)
```

CI (`.github/workflows/ci.yml`) nutzt Rust **1.91.0** und führt fmt, clippy, test und den wasm-Check aus. Vor jedem
Commit mindestens `cargo fmt` + `cargo clippy` + betroffene Tests laufen lassen. Linux braucht u. a.
`libasound2-dev libxkbcommon-dev libwayland-dev libclang-dev`. Die Cloud-Umgebung hat keine Audiogeräte und kein
Display: Die GUI lässt sich hier nur eingeschränkt ausprobieren (Tests + ggf. Playwright für die Web-Version).

## Konventionen im Code

- Sehr ausführliche `///`-Kommentare, die das *Warum* erklären; Issue-Nummern wie `(#149)` stehen in Kommentaren.
  Neuen Code im gleichen Stil kommentieren.
- Neue Features bekommen Tests (Unit-Test neben dem Code oder in `tests/`). Das Projekt ist stark getestet.
- Neue Modul-/Effekt-/Instrumentlisten haben meist mehrere Stellen, die zusammenpassen müssen (Modell in
  `src/song`, Parameter in `src/params`, Karte in `src/panels`, DSP in `crates/audio`, Dateiformat in `src/flm`).
  Beim Hinzufügen immer ein bestehendes ähnliches Feature als Vorlage suchen (`grep` nach dem Namen) und alle
  Fundstellen nachbauen.
- Dateiformat (`.flm`) bleibt kompatibel zu FL Studio Mobile: Unbekannte Chunks werden als `Raw` erhalten.
  Format-Änderungen nur mit Vorsicht.
- Keine neuen Abhängigkeiten ohne Rücksprache (Lizenz GPL, Web/Android/iOS müssen weiter bauen).
- `RELEASE_NOTES.md` und `README.md` beschreiben Nutzer-Features; bei sichtbaren neuen Features ergänzen.

## Arbeitsweise in diesem Fork

- **Der/die Besitzer:in arbeitet ausschließlich auf Android, ohne Computer.** Alles muss über Claude (Cloud) und
  GitHub laufen. Ausprobieren geschieht über eine Android-APK, die per GitHub Actions gebaut wird
  (Debug-APK: `tools/android-dist.sh`, braucht keinen Release-Schlüssel; die Release-Workflow-Datei verlangt
  Signatur-Secrets) oder über die Web-Version. Bevor ein Feature als „fertig" gilt: Tests grün, CI grün, APK gebaut.
  Anleitungen für den/die Besitzer:in deshalb immer für das Handy formulieren (GitHub-App/Browser, APK installieren).
- In der Cloud-Umgebung fehlen ALSA-Entwicklerbibliotheken: `cargo check` scheitert dort an `alsa-sys`, bis
  `libasound2-dev` (u. a., siehe CI) installiert ist.

- Entwicklung auf einem Feature-Branch, nie direkt auf `main`. Aktueller Arbeitsbranch der Session steht im
  Session-Prompt (z. B. `claude/honeydew-fork-setup-…`).
- Pull Requests nur auf ausdrücklichen Wunsch.
- Pro Feature: kleine, einzeln lauffähige Schritte, jeder Schritt = ein Commit.
