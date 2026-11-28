# FORK_STATUS.md – Stand, Entscheidungen und offene Punkte (Honeydew-Fork)

Übergabe zwischen Sitzungen. **Zuerst lesen, wenn eine neue Sitzung beginnt** (Abschnitt 0 genügt für den Start). `CLAUDE.md` hat die
Arbeitsregeln und die Architektur; diese Datei den Stand. Nach jedem Schritt hier nachtragen. Erledigtes und Überholtes liegt in
`FORK_ARCHIV.md` (nur bei Bedarf lesen). Stand: Tag 4, Commit `748ffc6`.

## 0. Aktueller Stand (Tag 4, 28.11., Ende der Sitzung)

- **Neuester Build:** Commit `748ffc6` (Lauf 46173979799, grün, APK-Artifact `honeydew-android-debug`). Enthält Paket 6 komplett
  (siehe FORK_ARCHIV.md 3l, 3m): Design-Feinschliff, WAV-Import für Wavetables, Zeichenmodus im Automations-Editor. Vom Besitzer noch **nicht
  getestet/bestätigt**. Letzter bestätigter Stand in `STAENDE.md` (Anleitung zum Zurückgehen steht dort); `f9471bc` und
  `748ffc6` sind dort noch einzutragen, sobald der Besitzer sie gut findet.
- Alle Tests, Clippy und fmt waren vor dem Push grün. Arbeitsbaum sauber.
- **Als Nächstes** (Plan in 4f): Automation direkt in der Zeitleiste bearbeiten, zunächst nur Punkte (erst aufklappbare Spur-Anzeige,
  dann Punkte setzen/ziehen; vorher die Geometrie vereinheitlichen). Zeichnen in der Zeitleiste und Latch/Aufnahme kommen später.
  Danach die Liste „später“ (Abschnitt 4 unten).
- Zuerst Feedback des Besitzers zur APK `748ffc6` einholen (Zeichenmodus: Schalter oben rechts im Raster, Strichvorschau).
- Regel: nach neuem Feedback Plan einmal abstimmen, dann bauen und APK-Link melden (siehe CLAUDE.md).

## 1. Worum es geht

Der/die Besitzer:in (keine Programmierkenntnisse, arbeitet **nur auf dem Handy, Android, Hochformat**, spricht Deutsch)
entwickelt diesen Fork von Honeydew (Open-Source-DAW in Rust, Vorbild FL Studio Mobile, GPL-3.0) zur besten DAW für
Android („gibt dort keine vergleichbar gute"). Erster Arbeitsbereich: der **GM Synth**. Wünsche: modernes, touchfreundliches
Layout im Hochformat, grafische reaktive Vorschauen (Filter, Envelopes, LFO, Oszillatoren), frei mappbare Envelopes/LFOs mit
Modulationsanzeige am Knopf, Custom-Wellenformen (später Wavetable), ein Farbsystem im Stil von Bitwig/Massive X (eigene
Umsetzung, übertragbar auf andere Module, später ans Theme anpassbar). Der Grundstil der FL-Mobile-Optik soll nicht
zu weit verloren gehen.

## 2. Arbeitsweise (Kurzfassung, Details in CLAUDE.md)

- Branch: `claude/honeydew-fork-setup-b6pc8o` (nur dort pushen, nie `main`, keine PRs ohne Wunsch). Fork: `OoTomEKKoO/honeydew`
  (öffentlich; Original `daijro/honeydew`, Version 0.5.5, unverändert = Commit `ed8fb6c`).
- **Arbeitsablauf mit Builds:** Jeder Push auf `claude/**` startet CI und den Android-Debug-Build. Nach neuem Feedback den Plan einmal abstimmen,
  dann codieren, testen, pushen und bis zur APK durchziehen (Regel in CLAUDE.md). Doku-Commits mit `[skip ci]`.
- Erklärungen in einfachem Deutsch, für das Handy formuliert (GitHub im Browser, APK installieren). Claude sieht die App nicht:
  Screenshots und konkrete Beschreibungen des/der Besitzer:in sind die Grundlage für UI-Arbeit.
- Vor jedem Push lokal: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
  (ALSA-Entwicklerpakete installieren: `apt-get install -y libasound2-dev libxkbcommon-dev libwayland-dev libclang-dev`).
- APK holen: Link zum Lauf (`gh api repos/OoTomEKKoO/honeydew/actions/runs`), Artifact `honeydew-android-debug` (ZIP mit APK),
  installiert sich als „Honeydew Debug" über die vorige (fester Debug-Schlüssel `android/debug.keystore`). Ein Build braucht
  etwa 8 Minuten.
- Vergleich mit dem Original: Actions → „Android debug APK" → Run workflow, Feld `ref` (gleiche App-ID und Signatur).
- **Fallstrick:** GitHub schaltet Workflows auf Forks von Zeit zu Zeit ab (Banner „Workflows on this fork have been disabled",
  `gh api … dispatches` antwortet „Actions has been disabled"): der/die Besitzer:in muss im Browser auf den grünen Knopf
  „I understand my workflows, go ahead and re-enable them" tippen (github.com/OoTomEKKoO/honeydew/actions). Danach Build mit
  `gh api -X POST repos/OoTomEKKoO/honeydew/actions/workflows/android-debug.yml/dispatches -f ref=claude/honeydew-fork-setup-b6pc8o`.
- Referenztabelle des GM-Synth-Panels (`REFERENCE` in `src/panels/inst/gm_synth.rs`) pinnt Rechtecke je Seite: nach
  Layoutänderungen Zeilen von Hand anpassen (Reihenfolge, Widget-Art, Ränder, Größenregeln, Tinte) und die Rechtecke mit
  `tools/harvest-gmsynth-reference.py` aus den Testmeldungen übernehmen (zweimal laufen lassen).

## 3. Wichtige Fundstellen (Kurzfassung des Archivs)

- **GM Synth** (`crates/audio/src/gmsynth.rs`, Panel `src/panels/inst/gm_synth.rs`): 77 Parameter (75/76 = POS A/B), Zusatzeinstellungen als
  feste Felder im `CSTM`-Blob (`src/panels/cstm.rs`, `grow()`, nur geschrieben wenn benutzt, alte Songs bleiben byte-gleich). Seiten:
  LEVEL, OSC, MOD, FILTER, EG, LFO, VOICES, FX (+ MATRIX). Zwei Tab-Leisten im Hochformat (nur pro Sitzung).
- **Mod-Matrix**: 4 Routen (Quelle LFO/EG, 19 Ziele, Betrag bipolar), `MOD_BASE` 66000; die LFO-/EG-Seiten haben zusätzlich eine feste Route
  (DEST+AMOUNT), sichtbar als Ring am Regler und Hinweis auf der MATRIX-Seite. Live-Werte im Analyse-Block: erst 96 Spektrum-Bänder
  (`live_bands`), dahinter die Zielwerte (`LIVE_TARGETS`).
- **Wavetables** (`crates/audio/src/wavetable.rs`, `wavetable_factory.rs`): 12 Werks-Tabellen + eigene per WAV-Import („Custom", Daten im Song als
  Chunk `HDWt`, Registry in `gmsynth.rs`), Position pro Stimme geglättet (kein Knistern). Oszillator-Menü mit Untermenüs (`SOURCE_GROUPS`).
- **P-Locks** (Piano Roll, Grafikstreifen „Lock"): `Lock{tick,key,target,value}` im Clip, Chunk `HDLk`, `src/ui/piano_roll/locks.rs`,
  Wiedergabe `src/audio_engine/locks.rs`. Locks folgen verschobenen/transponierten/quantisierten Noten (`locks::reconcile`) und fallen beim
  Löschen weg; **Kopieren/Einfügen/Duplizieren und Clip-Teilen tragen sie noch nicht mit.**
- **Design**: Theme „Graphite" (Nr. 9) mit den Theme-Schaltern `knobArc` (massiver Bogen-Regler, Wert nur in der Drag-Blase), `keyShadow`,
  `roomyMenus` (größere abgerundete Popup-Menüs, `ui/popup.rs`). Farbsystem je Seite (`theme::Accent`). Tabs: gewählter Tab in Seitenfarbe.
- **Automation-Editor** (`src/ui/automation/mod.rs`, ~4200 Zeilen): Punkte setzen/ziehen/auswählen; **Zeichenmodus** (Schalter oben rechts im
  Raster, Glatt/Steps, Logik `points::drawn` in `src/song/points.rs`).
- **Referenztabelle** des GM-Synth-Panels (`REFERENCE` in `gm_synth.rs`): nach Layoutänderungen von Hand anpassen und
  `tools/harvest-gmsynth-reference.py` zweimal laufen lassen.
- **Bekannte Fallen**: (1) Der Echtzeit-Thread darf nicht allozieren (`tests/rt_alloc.rs`). (2) Neue Instrument-Parameter ändern die Parameterzahl:
  Tests zu Legacy-Presets/Picks zählen mit (zuletzt 77). (3) Ein Push auf `claude/**` baut sofort; `[skip ci]` im Commit-Text verhindert das.
  (4) Tags lassen sich über den Proxy nicht anlegen, darum `STAENDE.md`. (5) Die GUI lässt sich in der Cloud nicht ansehen.
- **Klang-Befund**: Zwei Saw-Wellen/FM klingen „kaputt", im Original-Honeydew bit-genau genauso; FL Mobile klingt sauberer (siehe 4.G).

## 4. Offen

**C. Envelope-Punkte mit dem Finger ziehen** (Anzeigen in `gm_synth.rs`; `Draw` darf Params ändern: Beispiel `filter_display`).
 Die Zeit-Parameter-Gesetze sind exponentiell (`attack_rate`, `decay_rate`), Umkehrfunktion nötig.
**F. Gestaltung weiter:** Header sparen (`CARD_HEADER` 1.5 U ist global), Übertragung des Systems auf andere Module, Theme-Ableitung der
 Akzente (`Accent::color(&ThemeColors)`), VOICES- und FX-Seite im neuen Stil, Ein-/Zwei-Seiten-Wahl speichern, abgerundete Karten mit dünnem Rand.
**G. Klang:** (1) Oszillator-Qualität vs. FL Mobile. Hypothesen: `gm_waveshapes::read` nimmt den nächsten Tabellenwert plus Dither (1024 Punkte)
 statt zu interpolieren, Aliasing bei hohen Tönen/starker FM. Vorgehen: Rauschabstand offline messen, Verbesserungen einzeln mit Messung und
 Hörtest. (2) Filter klingt „steril": Drive/Sättigung, Resonanzverhalten. Level-Spitzen zweier Saw-Wellen liegen um 1,5.
**H. Sonstiges:** Upstream-Änderungen von `daijro/honeydew` gezielt übernehmen (cherry-pick; Repo mit `add_repo` anhängen); der Workflow
 `lemon-effects.yml` des Originals läuft wöchentlich auf `main`; Veröffentlichung später: GPL-Pflichten, Name/Branding prüfen (CLAUDE.md);
 FL-Mobile-Kompatibilität der Songs prüfen; P-Locks bei Kopieren/Einfügen/Clip-Teilen mitführen, P-Locks im Drum-/Step-Editor und für Kanalregler;
 Trance Delay aufwerten (3/16-Timing, moderne Oberfläche im Farbthema); mehr als ein EG/LFO im GM Synth; Modulations-Live-Punkt pro Stimme.

## 4d. Feature-Liste des Besitzers (Tag 4), geordnet und geplant (Planung; A2 und die Zeichenmodus-Hälfte von B3/B4 sind inzwischen gebaut)

Quelle: Liste des Besitzers vom Morgen von Tag 4. Zuerst der Abgleich mit dem, was schon im Code steht, dann die
Themen-Gruppen mit Aufwand (S klein, M mittel, L groß), Abhängigkeiten, Recherche und der empfohlene Ablauf.

### Abgleich der als erledigt markierten Punkte ([X] in der Liste)

| Punkt | Stand im Code | Offen davon |
|---|---|---|
| GM Synth Wavetable (zwei Tabellen, Pitch, Morph, FM) | Gebaut (Tag 3): 12 Werks-Tabellen, POS A/B, 3D-Vorschau, FM/Ring/Sync bleiben. Noch nicht auf dem Handy getestet. | **Warp-Modi** (siehe A1), WAV-Import (A2) |
| GM Synth Envelopes frei mappbar | Matrix mit LFO/EG als Quelle, bis 4 Routen, Ring am Regler, Inspektor. | Mehr als ein EG (pro Stimme), mehr Quellen (Velocity, Key, Zufall) |
| LFO-Upgrades (Random, S&H, Grafik, freies Mapping) | S&H, Note-Random, Tempo-Sync, Grafik, freie Ziele über die Matrix: gebaut. | Mehr LFOs (2+), freie Hz-Rate, LFO je Stimme |
| Drum-Sequencer-Automation (Zeilen, Pro-Schritt) | **Nur teilweise:** P-Locks gibt es im Piano Roll, nicht im Drum-/Step-Editor. | P-Locks im Drum-Editor und Step-Sequencer (A3/B), Rückfrage an den Besitzer: war mit [X] "geplant" oder "aus dem Original" gemeint? |

### Gruppen und Reihenfolge

**A. Klang und Synth** (Aufbau auf der frischen Wavetable-Engine)
- A1 *Warp-Modi für Wavetables* (M). Recherche: Serum bietet Sync (1/2 und voll, auch "Window"), Bend ±, Asym ±, PWM,
  Flip, Mirror, Quantize, FM, AM, RM. Plan: pro Oszillator ein Warp-Menü plus Warp-Regler (Parameter, damit mappbar), gerechnet
  beim Lesen der Phase (Phasenverzerrung ist billig und aliasarm, Quantize/Mirror brauchen Bandbegrenzung der Tabelle).
  Die Warp-Wirkung zeigt die 3D-Vorschau mit. Vorher mit dem Besitzer klären, welche 6 bis 8 Modi zuerst.
- A2 *(ERLEDIGT, Build 748ffc6)* *Single-Cycle- und Wavetable-Import per WAV*: Datei in Frames zu 2048 Punkten, Einzelzyklus = 1 Frame.
  Speicherung der Daten im Song und Dateiauswahl wie bei der 3x-Osc-Custom-Welle. Vorher Fragen-Panel.
- A3 *Modulator-Modul*: zusätzliche LFO-Formen, Grafik für LFO und MSEG (mehrstufige Hüllkurve) (L). Baut auf der Anzeige-
  Technik der GM-Synth-LFO-Seite auf; MSEG braucht einen Punkte-Editor (Vorlage: Automations-Editor `src/ui/automation`).
- A4 *Mehr Hüllkurven/LFOs im GM Synth* (S bis M): Matrix-Quellen erweitern (EG 2, LFO 2, Velocity, Key).

**B. Automation 2.0 und P-Locks** (ein zusammenhängendes Großthema, das Fundament für P-Locks "für alle Parameter")
Reihenfolge innerhalb, weil jeder Schritt auf dem vorigen aufbaut:
1. B1 *Gemeinsame Skalierung und Einrasten* (Zeit und Wert) zwischen Automations-Lane und Projekt-Zeitleiste (M). Zuerst
   die Geometrie vereinheitlichen (ein Zeitachsen-Objekt für Playlist und Editoren; `ui/automation/geom`, `ui/timeline`).
2. B2 *Lane-Oberfläche wie bei Ableton* (L): klare Parameterbeschriftung an der Lane, direktes Bearbeiten in der Zeitleiste,
   Lanes mit dem Song mitskalieren. Plan: Lane-Kopf mit Name "Parameter : Modul", Farbe des Moduls, Auswahl per Tipp.
3. B3 *Automation direkt im Piano Roll zeichnen* (L): derselbe Zeichnen-Editor als Streifen unter den Noten (Grafikstreifen
   existiert; die Lock-Lane ist die Vorstufe). Hier laufen P-Locks (pro Note) und Kurven (pro Zeit) zusammen.
4. B4 *S-Pen-Unterstützung* (M): flüssiges Zeichnen von Kurven plus Stepped-Modus. Recherche: Android liefert Stylus als
   `TOOL_TYPE_STYLUS` mit Druck, Neigung und vielen Zwischenpunkten pro Frame (`getHistorical…`), dazu gibt es Bibliotheken
   für niedrige Latenz und Bewegungsvorhersage. Risiko: Das vendorte `winit` reicht den Stift nur als `Touch` mit optionalem
   `force` durch, die Zwischenpunkte kommen nicht an. Vermutlich braucht es einen kleinen Java-Teil in `HoneydewActivity`, der
   Stylus-Punkte per JNI (Muster wie `onMic`/`onHeadphones` in `src/android.rs`) an die Automation liefert. Erst prüfen, ob
   `force` und die Touch-Rate reichen (billig), sonst die JNI-Variante.
5. P-Lock-Reste : Locks folgen Noten, Drum-/Step-Editor, Kanalregler.

**C. Sequencing, Routing, Instrumente**
- C1 *MIDI-Note steuert eigenen Synth im selben Rack* und *Effekte nur einem Synth zuweisen* (L, hängt zusammen). Heute hat
  jeder Kanal ein Rack mit Instrumenten und Effekten, die alle denselben Kanal hören; `midi_sends` verbindet Kanäle. Plan:
  Rack in "Stimmen/Zweige" gliedern (Instrument + seine Effektkette), jeder Zweig mit eigener Note-Quelle (Notenspur, Drum-Pad,
  Arp). Das ist eine Änderung am Datenmodell (`Rack`) und an `audio_engine`, also gründlich testen. Vorher Fragen-Panel mit
  Skizze, weil es die Bedienung des Racks stark verändert.
- C2 *Drum-Rack-Sequenzen an Synths per MIDI-Sends* (M): auf C1 aufbauen oder vorher als Zwischenlösung die vorhandenen
  Sends für Drum-Pads erlauben.
- C3 *Arpeggiator-Update* (L): eigene Muster pro Schritt (Pitch, Velocity, Länge, Wahrscheinlichkeit, Transposition),
  Parameter-Steuerung, Zwei-Arp-Zusammenspiel. Recherche: Ableton hat dafür das Step Arp (Sequencers-Pack, Max for Live) mit sechs
  Spuren (Velocity, Länge, Chance, Transposition …), jede mit eigener Schrittzahl und Richtung; "Snake"-artige Ideen stammen
  von LFO-Phasen-Sequenzern (z. B. LF-ARP); Dual-Arp-Geräte gibt es nach dem Vorbild von Reason. Plan: neuer Arp-Modus
  "Pattern" mit Spuren und eigener Länge je Spur (polyrhythmisch), Anzeige wie die Drum-Editor-Lane. Vorlage im Code:
  `crates/audio/src/sequencer.rs` (Arpeggiator) und `src/panels/fx/arpeggiator.rs`.
- C4 *Slicer: manuelle Regler für Länge, Start und Ende der Slices* (M): Die Slicer-Karte hat heute Sensitivity, Pitch,
  Attack, Decay. Plan: Wellenform-Ansicht mit ziehbaren Slice-Markern (Vorlage: Audio-Editor `ui/audio_editor`) und Start/Ende-Regler pro Slice.

**D. Arbeitsfluss und Oberfläche**
- D1 *Clips manuell einfärben* (S): Kanal hat `hue`; ein Clip bekommt optional eigene Farbe (neues Feld im Clip, eigener
  Honeydew-Chunk, Menü "Farbe" im Clip-Menü).
- D2 *Marker-Spur mit Text* (S bis M): Es gibt bereits `Section` (benannte Bereiche, `src/song/section.rs`). Plan: Marker als
  Punkt-Markierungen mit Text auf einer eigenen Spur über der Zeitleiste, Sections bleiben Bereiche.
- D3 *Sample-Browser Drag & Drop* auf die Zeitleiste und in Drum-Sequencer-Lanes (M). Ansatz da: der Modul-Browser kennt
  schon "Lang drücken, dann ziehen" (`ui/file_drag`, `modules_browser.rs`). Ziel-Erkennung in Playlist und Drum-Lanes ergänzen.
- D4 *EQ: mehr Bänder und Analyser direkt im EQ* (M): Der Equalizer hat 4 Bänder; mehr Bänder heißt mehr Parameter (Honeydew-
  eigene, wie beim GM Synth), die Anzeige zeigt dann das Spektrum der Wavetable-/Filter-Anzeige-Technik (`SpectrumProbe`
  im Audio-Crate) direkt hinter der Kurve.
- D5 *Rack-Scrollleiste mit Klick und Hervorhebung* (M): Die Leiste schnappt je Modul ein (wie die App-Schublade bei Android), das
  Modul skaliert kurz leicht hoch. Braucht Haptik (D6) für den "Klick".
- D6 *Haptik* (M, danach überall einsetzbar): Recherche: Android bietet `VibrationEffect` mit vordefinierten Stärken (CLICK,
  TICK, LOW_TICK, jeweils skalierbar, älter als "Composition-Primitives" mit Skalierung und Verzögerung). Die Geräte-
  Unterstützung ist unterschiedlich (`Vibrator.arePrimitivesSupported` prüfen). Hinweis aus der Doku: viele Ticks hintereinander
  addieren sich in der Wahrnehmung, also kalibrieren. Plan: ein kleines Haptik-Modul (Rust → Java über JNI wie `src/android.rs`)
  mit wenigen benannten Ereignissen (Regler-Raster, Tab-Wechsel, Menü-Auswahl, Rastpunkt der Scrollleiste, Drücken) statt
  einer langen Vibration; jedes Ereignis mit eigener Stärke/Länge und zeitlich an die Animation gekoppelt; Schalter und
  Stärke in den Einstellungen. Vorher das Gerät des Besitzers erfragen (Samsung-Haptik-Motor kann feiner sein).
- D7 *MIDI-Controller-Unterstützung* (S bis M): Im Code ist MIDI-Eingabe/-Ausgabe schon da (`src/midi`, MIDI-CC-Links pro
  Kanal). Unklar, was fehlt: **Rückfrage** (Fragen-Panel): MIDI-Learn auf jeden Regler, Controller-Presets, Pitchbend/Aftertouch,
  USB-OTG am Handy, oder Bluetooth-MIDI?


**E. Für später (vom Besitzer am Tag 4 auf die Liste gesetzt, noch nicht begonnen)**
- E1 *Desktop-Layout und DeX-Modus* (L): Wenn der Besitzer die App über Samsung DeX nutzt, soll ein Modus die Oberfläche für
  Desktop-Skalierung und Mausbedienung optimieren, mit Tastenkürzeln und Desktop-Funktionen, möglicherweise auch mit
  Anpassungen für Fenstergröße und Einrasten (Snapping) wie unter Windows. Ausgangslage im Code: Tastenbelegungen als JSON
  (`assets/content/keymaps/windows.json`, `mac.json`, `src/keymap.rs`), UI-Skalierung als Einstellung (`ui_scale`), Maus- und
  Hover-Verhalten in den Widgets (Rechtsklick öffnet schon das Parametermenü). Plan: (1) DeX/Desktop-Modus erkennen
  (`Configuration`-Änderungen, Fenstergröße und Zeigergerät über JNI, Muster `src/android.rs`), (2) eine Android-Tastenbelegung mit
  den üblichen Kürzeln, (3) Hover-Hinweise, Mausrad-Zoom und Scrollen, Rechtsklick-Menüs überall, (4) dichtere Desktop-
  Skalierung der Bedienelemente, (5) Fenster-Größenänderung und Snapping sauber durchreichen (kein Neustart der Engine).
  Vorher klären: welche Kürzel zuerst, welcher Bildschirm und welche Skalierung in DeX.
- E2 *Sampler umbauen* (L): mehr Regler wie Start- und Endpunkt, Loop-Funktion und mehr; als Basis dient der **Slicer** (er hat
  schon Wellenform-Ansicht und Slice-Marker, `src/panels/inst/slicer.rs`; der DW Sampler `dw_sampler.rs` bleibt für Zonen).
  Dazu **Drag and Drop aus dem Dateimanager direkt in den Sampler**. Ausgangslage: Datei-Ablage für Desktop gibt es
  (`src/app/file_drop.rs`, `ui/file_drag`); unter Android kommt das Ablegen aus dem Dateimanager als `DragEvent` mit URIs in der
  Activity an und braucht einen kleinen Java-Teil, der die Datei in den App-Speicher kopiert und der App den Pfad meldet (Muster
  wie die Mikrofon-Rückmeldung). Passt zusammen mit C4 (Slicer-Regler) und D3 (Browser-Drag-and-Drop); sinnvoll als ein Paket.

### Empfohlener Ablauf (zur Bestätigung durch den Besitzer)
1. *(erledigt)* APK-Tests, WAV-Import. Offen davon: Lock-Folge-Verhalten bei Kopien (B5).
2. **Automation 2.0** (B1 bis B4): größter Hebel, trägt P-Locks, Wavetable-Position, Arp-Muster und Modulator.
3. **Routing im Rack** (C1, C2) mit Skizzen-Abstimmung; danach Arp (C3) und Slicer (C4).
4. **Klang-Ausbau** (A1 Warp, A3 Modulator/MSEG, A4) und Trance Delay.
5. **Arbeitsfluss** (D1 bis D7), in kleinen Paketen zwischen den großen Themen eingestreut (D1, D2 sind schnell fertig).
6. Haptik (D6) früh anfangen, weil sie viele andere Punkte (Scrollleiste, Regler) verbessert.
7. Zum Schluss, auf Wunsch des Besitzers erst später: Sampler-Umbau mit Dateimanager-Drop (E2) zusammen mit C4/D3, danach der Desktop-/DeX-Modus (E1).

### Quellen der Recherche
- Android-Stift: [Stylus input](https://developer.android.com/develop/ui/views/touch-and-input/stylus-input/advanced-stylus-features)
  (Druck, `TOOL_TYPE_STYLUS`, Zwischenpunkte `getHistorical…`, Low-Latency- und Motion-Prediction-Bibliotheken).
- Android-Haptik: [Custom haptic effects](https://developer.android.com/develop/ui/views/haptics/custom-haptic-effects) und
  [Constants and primitives](https://source.android.com/docs/core/interaction/haptics/haptics-constants-primitives).
- Warp-Modi: [Serum-Warp-Menü (Kurs)](https://phoenix.creativelive.com/class/xfer-serum-synthesizer-sound-design-music-production-tomas-george/lessons/warp-menu),
  [Wavetable Wave & Warp Effects (Minimal Audio)](https://support.minimal.audio/articles/29407564499739-wavetable-wave-and-warp-effects).
  Vital: dazu keine verlässliche Quelle gefunden, erst beim Bauen genauer ansehen.
- Arpeggiator: [Step Arp, Sequencers-Pack](https://www.ableton.com/en/packs/sequencers/),
  [DualArpPlus (Max for Live)](https://maxforlive.com/library/device/10150/dualarpplus).
  Ein Gerät, das "Snake" und Dual-Arp zugleich bietet, wurde nicht gefunden; "MDD Snake" bleibt für den Besitzer zu erklären.


## 4f. Plan: Automation (Stand nach dem Zeichenmodus)

Wunsch des Besitzers: Automation direkt im Zeitleisten-Fenster bearbeiten, ähnlich wie bei Ableton Live.

**Erledigt (Build 748ffc6):** Zeichenmodus im geöffneten Automations-Editor (Schalter Points/Draw, Glatt/Steps; Finger zeichnet auch, keine Stift-Erkennung nötig).
Ein Strich ersetzt die Punkte darunter, ist ein Undo-Schritt. Optional später: Historienpunkte des Stifts durchreichen (vendorter `winit`
reicht Zwischenpunkte nicht durch; `android-activity` 0.6.1 prüfen).

**Als Nächstes: Teil 3, Automation direkt in der Zeitleiste, zunächst nur mit Punkten.** Entscheidung des Besitzers; Zeichnen in der Zeitleiste später.
- Ausgangslage: Automations-Lanes sind Playlist-Zeilen mit Clips (`LaneKind::Automation`), der Clip hält die Punkte, eine Miniatur der Kurve ist sichtbar.
  Voraussetzung ist die gemeinsame Zeit- und Wertgeometrie von Playlist und Editor (Punkt B1 in 4d): ein Zeitachsen-Objekt, gleiche Zoomstufen und Einrasten.
- Die Automations-Zeile lässt sich **aufklappen** (höher, etwa 4 U) und zeigt die Kurve groß; aufgeklappt arbeitet derselbe Editor **in der Zeile**
  (Punkte setzen und ziehen, mit den Gesten des Editors, auf dem Maßstab der Zeitleiste). Setzen **außerhalb eines Clips** legt einen Clip an oder verlängert ihn.
- Zeilenkopf: „Parameter : Modul" in der Modulfarbe, Tippen öffnet die Parameterwahl.
- Risiko: Der Editor ist ein eigener Bildschirm (Scroller, Auswahl, Zwischenablage); als Teil der Playlist muss er zu einer Komponente mit eigener
  Fläche und geteiltem Scroller werden. Darum in Stufen: (a) Zeile aufklappen und nur anzeigen, (b) Punkte setzen/ziehen, (c) Zeichnen, (d) Auswahl und
  Zwischenablage. Je Stufe Tests, ein Commit; **vorher Skizze/Plan mit dem Besitzer abstimmen** (Fragen-Panel).

**Danach: Latch (Aufnahme).** Während der Wiedergabe schreibt ein bewegter Regler Automation; Latch hält den letzten Wert bis zum Stopp, Touch springt zurück.
Kern in `src/app/auto_record.rs` vorhanden; offen ist ein sichtbarer Schalter (Aus, Touch, Latch) im Transport. Eigener Schritt, vorher absprechen.

## 5. Bekannte Einschränkungen

- Die Oszillator-FM-Anzeige ist ein **Schnappschuss** (im Klang sieht jeder A-Zyklus einen anderen Teil von B); PITCH B springt in
  Halbtonstufen (FINE B ist stetig). Starke FM (x16) zeigt stauchende Spitzen – das ist der echte Verlauf.
- Envelope-Anzeige: Zeitachse nur grob proportional; Filter-Envelope (Seite FILTER: ATTACK/DECAY/AMOUNT) hat noch keine Anzeige.
- Ein-/Zwei-Seiten-Wahl ist nicht gespeichert. Push-Benachrichtigungen kamen beim Besitzer nicht an (Einstellung dort).
- Windows/macOS/iOS/Web: Code kompiliert weiter (CI Linux: wasm-Check), wird aber nicht getestet.
