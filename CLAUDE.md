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

- **Fokus: ausschließlich Android.** Ziel ist die beste DAW für Android, perfekt darauf optimiert (Touch-Bedienung,
  Audio-Latenz, Akku/CPU, Speicher). Windows, macOS, iOS, Linux und Web sind erstmal außen vor: Ihre Builds und
  Prüfungen laufen nicht automatisch (Windows/macOS-Job in `ci.yml` nur manuell). Der gemeinsame Code soll aber
  weiter kompilieren, solange das wenig kostet (CI-Linux-Job: fmt, clippy, test, wasm-Check), damit Änderungen
  des Upstream-Projekts später übernommen werden können. Android-spezifisches darf `cfg(target_os = "android")`
  nutzen.

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

## Lizenz (GPL-3.0-or-later) – Merkzettel für später

Der Fork ist ein Derivat eines GPL-3.0-Projekts. Wer die App (APK, Web-Build, …) weitergibt, muss:
- die Lizenz GPL-3.0-or-later beibehalten (`LICENSE`), Copyright-Hinweise nicht entfernen, eigene Änderungen als
  solche kenntlich machen (Git-Verlauf genügt) und den vollständigen Quellcode der verteilten Version bereitstellen
  (z. B. Link auf diesen Fork/Tag). Der eigene Code muss ebenfalls GPL-3.0-or-later sein.
- `THIRD_PARTY_NOTICES.md` und die Sound-Credits (App: Settings → Sound credits) mitliefern. Einige Sounds sind
  CC BY 3.0, das Icon („Melon" von afitrose) verlangt Namensnennung.
- Nicht mit „FL Studio" werben; der Haftungsausschluss in `README.md` („About") bleibt. Keine FL-Studio-Mobile-Inhalte
  (Sounds, Presets, Artwork) einbauen oder verteilen. Beim öffentlichen Veröffentlichen prüfen, ob der Name
  „Honeydew" und das Branding übernommen werden dürfen/sollen.
- Neue Abhängigkeiten nur mit GPL-kompatibler Lizenz (siehe Abschnitt Konventionen).
Keine Rechtsberatung: bei einer echten Veröffentlichung die Punkte noch einmal prüfen.

## Android-Testbuild

`.github/workflows/android-debug.yml` baut bei jedem Push auf `claude/**` (und manuell) eine Debug-APK ohne Sounds
(`tools/android-dist.sh --base`), Artifact `honeydew-android-debug` im Actions-Lauf. Sie installiert sich als
„Honeydew Debug" neben der Original-App. `android/debug.keystore` ist ein bewusst eingecheckter Wegwerf-Debug-Schlüssel
(Passwort `android`), damit neue Builds über alte installiert werden können. Nie für Releases verwenden.

## UI-Änderungen ohne Sicht

Claude kann die laufende App nicht sehen (keine Anzeige in der Cloud). Layout kommt nur aus dem Code: UI-Tests in
`src/panels/tests/reference.rs` prüfen Rechtecke, nicht den Eindruck. Deshalb: UI-Wünsche bevorzugt über Screenshots
und konkrete Beschreibungen des/der Besitzer:in (Handy), nur eine UI-Änderung pro Testbuild, danach Rückmeldung
einholen. Der erste Prototyp einer UI-Änderung kann ausdrücklich unfertig sein. Idee für später: Panels offscreen zu
Bildern rendern (`src/ui/offscreen.rs` ist ein Testgerüst dafür) und selbst ansehen.

Stand: GM Synth hat auf FILTER ein neues Dropdown EXTRA (Off/BP/Notch/Comb). Es wurde auf dem Handy getestet und läuft;
das UI-Layout davon soll noch verbessert werden (Rückmeldung des/der Besitzer:in steht aus).

## Roadmap (vereinbart mit dem/der Besitzer:in, Android-Hochformat im Fokus)

Jeder Schritt = eigener Build zum Testen auf dem Handy, danach Rückmeldung. UI muss zum Stil von FL Mobile/Honeydew
passen (Farben, Schriften, Abstände der bestehenden Panels; nichts Überlappendes, touchfreundliche Flächen).
**Entscheidung: alles direkt im GM Synth, das Modulator-Modul (Typ 1005) wird vorerst nicht angefasst.** Die
GM-Synth-Stimmen haben ihre eigenen Envelopes: Envelopes starten bei jedem Anschlag neu und gelten pro Stimme.

1. **GM-Synth-Layout im Hochformat.** `KIND`/`EXTRA` überlappen (Spinner-Beschriftung vs. Pfeile, EXTRA-Zeile zu eng:
   Reihe auf FILTER hat 3 Zeilen in 6.24 U) – reparieren. Neu: **zwei Tabs gleichzeitig übereinander** anzeigen
   (z. B. MOD + OSC oder zwei beliebige), welche beiden sichtbar sind, lässt sich durchschalten/wählen. Karte im
   Hochformat entsprechend höher (Vorbild: `SOUNDFONT_PORTRAIT` in `src/ui/rack.rs`; `ui_height(type_id)` ist bisher
   nur pro Typ statisch, `card_height(module)` bekommt das Modul).
2. **Wellenform-Vorschau pro Oszillator (A und B), hochwertig und reaktiv** (folgt Auswahl, später Custom-Welle, Pitch/
   Phase B/Balance), im Stil des Moduls (Display via `.draw(fn)` wie in `src/panels/inst/three_osc.rs`; Daten aus
   `crates/audio/src/gm_waveshapes.rs`). Vorbereitung für späteren Wavetable-Einbau.
3. **Grafische Vorschau für die vorhandenen Envelopes** (Level, Filter, Mod, EG) und den LFO, **mit Fingerbedienung**:
   Punkte (Attack, Decay, Sustain, Release) direkt ziehen statt Slider. Eine `Draw`-Funktion darf Params ändern und
   melden (`src/panels/mod.rs` `Draw`), das geht also ohne Formatänderung.
4. **Neuer Tab für frei mappbare Envelopes und LFOs** im GM Synth (wie 3x Osc, mit ziehbaren Punkten): mehrere
   Envelopes (starten bei jedem Anschlag) und LFOs, jeder mit Ziel-Parameter (aus den GM-Synth-Parametern) und Amount.
   LFO-Formen auch **Sample & Hold**, mit Sync-Modi (1/16 usw.) und Trigger-Modi (frei / bei jeder Note neu).
   Entschieden: zum Testen **je 1 Envelope und 1 LFO** (später per Kopie erweiterbar); Ziele: Cutoff, Resonanz, Pitch,
   Osc-B-Pitch, Balance, Noise, Mod-Amount, Level, Pan. Neue Daten brauchen Platz im Song: Params
   hinter Index 74 (Vorvorbild: Honeydew-eigene SoundFont-Player-Params 6–11 in `src/params/instruments.rs`) – vorher
   prüfen, wie FLM-Laden/Speichern mit zusätzlichen Params umgeht. Engine: `crates/audio/src/gmsynth.rs`, ein LFO ist
   bisher global.
5. **Custom-Wellenformen aus WAV-Dateien** für Osc A und B (Vorbild: `three_osc::osc::Wave::custom`; Welle auf dem
   App-Thread bauen wie `gmsynth::prepare_setting`; Pfad im Song speichern; in ZIP-Export aufnehmen).
6. **Klang des Filters** (wirkt „steril"): Drive/Sättigung vor dem Filter, Resonanzverhalten, minimale analoge
   Unregelmäßigkeiten; nach Gehör des/der Besitzer:in.

Erledigt: GM Synth `EXTRA`-Filter (Off/BP/Notch/Comb), Android-Debug-Build per GitHub Actions.
Schritt 1 gebaut, wartet auf Handy-Test: Zwei-Tabs-Ansicht im Hochformat (`PanelState::dual_tabs/tab2`, `tap_tab`,
`item_list_marked`, `GMSYNTH_PORTRAIT` 15 U in `src/ui/rack.rs`), FILTER mit 7 Spalten (EXTRA eigene Spalte).
