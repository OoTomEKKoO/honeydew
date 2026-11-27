# FORK_STATUS.md – Stand, Entscheidungen und nächste Schritte (Honeydew-Fork)

Übergabe zwischen Sitzungen. **Zuerst lesen, wenn eine neue Sitzung beginnt.** `CLAUDE.md` hat die Arbeitsregeln und die
Architektur; diese Datei den Stand. Nach jedem Schritt hier kurz nachtragen (Stand, Rückmeldung des/der Besitzer:in).

## Kurzstand (Ende Tag 2)

- Arbeitsbranch: `claude/honeydew-fork-setup-b6pc8o` (nur dort pushen, nie `main`). Jeder Push startet CI und den
  Android-Debug-Build (Artifact `honeydew-android-debug`, Link über `gh api repos/OoTomEKKoO/honeydew/actions/runs`).
- Der/die Besitzer:in arbeitet nur auf dem Handy (Android, Hochformat), testet jede APK und schickt Screenshots.
- Gebaut und vom Besitzer getestet: GM Synth mit EXTRA-Filtern (BP/Notch/Comb), FM RANGE (x1…x16), zwei Tab-Leisten
  (zwei Seiten gleichzeitig), alle Slider als Knobs, Wellen-Auswahl per Tippen auf die Vorschau, kompakte Menüs, Ein-/
  Zwei-Seiten-Schalter im ≡-Menü, dunkle Popup-Menüs (`menuFill`), Tabs auch bei gehaltener Taste (zweiter Finger).
- Zuletzt gebaut (wartet auf Handy-Test): Tab-Reihenfolge LEVEL/OSC/MOD/FILTER/EG/LFO/VOICES/FX, einzeilige Menüs,
  höhere Anzeigen (Filter-Frequenzgang mit Ziehen, Envelope-Stufenanzeige, LFO-Anzeige), ein ganzer Zyklus pro Wellenanzeige.
- Klangbefund: GM-Synth-Oszillatoren klingen im Original-Honeydew wie im Fork „kaputt" im Vergleich zu FL Mobile
  (bit-genau identisch zum Original); Verbesserung ist Punkt 8 unten.

## Farbsystem v1 (gebaut, wartet auf Handy-Test)

Vorbild laut Screenshot des Besitzers (Bitwig Polymer): neutrale dunkle Flächen, je Bereich genau eine Akzentfarbe, die nur
„Lebendiges" färbt (Titel, Kurve/Punkte, Wertbogen am Knopf); Knopfkörper neutral; verschiedene Envelopes verschiedene Farben.
Umsetzung: `theme::Accent` (Rollen Osc gelb, Mod orange, Filter cyan, Level blau, Envelope grün, Lfo violett, Fx magenta;
`Accent::color(&ThemeColors)` ist die Stelle für spätere Theme-Anpassung), `Ink::Accent`, `build::tint(part, accent)`,
im GM Synth pro Seite angewendet (VOICES bleibt Kanalfarbe). Test: Kontrast ≥ 3 auf den Panelfarben des Standard-Themes.
Noch offen: farbiger Streifen/Titel pro Seite, Modulationsring in Quellfarbe, Übertragung auf andere Module, Theme-Ableitung.

## Nächste Schritte (Reihenfolge nach Absprache)

1. **Gestaltungsprinzip / Farbsystem (Bitwig-/Massive-X-artig, eigene Umsetzung).** Dunkle Flächen bleiben (Theme),
   Akzentfarben mit festen Rollen färben nur Ringe, Kurven, Beschriftungen und kleine Balken: z. B. warm für Oszillator/
   Welle, kühl für Filter, grün für Envelopes, violett für LFO/Modulation. Die Logik soll später auf andere Module
   übertragbar sein (zentral definieren, z. B. eine Rollen-Palette in `src/theme.rs` oder `src/panels/accent.rs`, die ein Leaf
   über `Ink`/Akzentrolle benutzt) und sich später dem Theme anpassen (Hue aus der Kanalfarbe ableiten). Erst jetzt als
   Prinzip und im GM Synth; Übertragung auf andere Module und Theme-Anpassung nach Meinungsbildung des/der Besitzer:in.
   Bitwig: Modulatoren je eigene Farbe, ein modulierter Regler zeigt einen Ring in der Modulatorfarbe, der der Modulation folgt.
2. **Anbindung „Instrument → Oberfläche"** (`Instrument::analysis()`, `device.rs` `Visual`-Slab pro Meter, Größe je Typ,
   `audio_engine/mod.rs` `analysis(index)` ist für Instrumente leer, App-Seite `live_analysis`): liefert dem GM Synth ein
   Spektrum (für den Spektrum-Hintergrund der Filter-Anzeige, der auf den Filter reagiert) und die LFO-Phase (für den
   laufenden Punkt auf der LFO-Kurve).
3. **LFO-Erweiterung im GM Synth:** Sample & Hold, Sync-Modi (1/16 …), „bei jeder Note neuer Zufallswert" mit frei
   einstellbaren Grenzen (min/max). Neue Einstellungen im CSTM-Blob (Muster: `FILTER_EXTRA` 16300 / `FM_RANGE` 32100, Feld in
   `panels/cstm.rs` GMSYNTH, `grow()` schreibt erst bei Nutzung), Engine in `gmsynth.rs` (LFO ist bisher global, `lfo_value`).
4. **Envelope-Punkte mit dem Finger ziehen** (Anzeigen in `gm_synth.rs`, `Draw` darf Params ändern und melden).
5. **Frei mappbare Envelope(s) und LFO(s) im GM Synth** mit Ziel-Parameter und Amount, Modulationsring am Knob (je 1 zum
   Testen). Siehe Roadmap Punkt 4 unten.
6. **Custom-Wellenformen aus WAV** für Osc A/B (Roadmap Punkt 5).
7. **Header sparen** (`CARD_HEADER` 1.5 U ist global; für den GM Synth schmaler?), weitere Seiten (VOICES, FX) im neuen Stil.
8. **Oszillator-/Filter-Klangqualität** (Roadmap Punkt 7 und 6).
9. **Upstream-Änderungen** (`daijro/honeydew`) gezielt übernehmen (cherry-pick), Repo dafür kurz mit `add_repo` anhängen.

## Wunsch Gestaltung (Originalton des/der Besitzer:in, Tag 2)

Gefällt: Massive X (dunkle Oberfläche gepaart mit Farben). Soll **keine Kopie** sein, aber dynamisch; später auf andere Module
übertragbar (Kohärenz, „aus einem Guss"), der Grundstil der DAW (FL-Mobile-Optik) soll nicht zu stark verändert werden.
Orientierung an Massive X **und Bitwig Studio** (Farbgebung der Module nachsehen); **die Farben wie in Bitwig-Modulen schon
jetzt**, also ein eigenes Gestaltungsprinzip; die Anpassung der Palette an das Theme kann nachträglich folgen, wenn der/die
Besitzer:in sich eine Meinung gebildet hat.

## Wichtige Fakten (nicht vergessen)

- Dateiformat-Kompatibilität zu FL Studio Mobile: Honeydew-eigene GM-Synth-Einstellungen stehen im CSTM-Blob **hinter** den
  ersten 20 Bytes und nur, wenn benutzt (`FILTER EXTRA` @20, `FM RANGE` @24); alte Songs bleiben unverändert.
- Tests: Referenztabelle `REFERENCE` in `src/panels/inst/gm_synth.rs` pinnt Rechtecke je Seite; nach Layoutänderungen
  lassen sich die Rechtecke aus der Fehlermeldung des Tests (`cargo test --lib panels::tests::reference`) übernehmen.
- Popups: `src/ui/popup.rs` (`panel_fill`, `label_ink`); Menüs kompakt: `Leaf::compact`; Wellen-Auswahl: `widgets::tap_picker`.
- Vergleich mit dem Original: Actions → „Android debug APK" → Run workflow, Feld `ref` (z. B. `ed8fb6c` = unverändertes 0.5.5).
- Lokale Prüfung vor jedem Push: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Die Öffentlichkeit: Fork ist öffentlich (GPL), Commits als „Claude"; die E-Mail-Adresse steht nicht im Repository.

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
Schritt 1 (Layout) auf dem Handy getestet: UI gut; Wunsch nach zwei Tab-Leisten umgesetzt (`PartKind::Pager` +
`build::pager`, `PanelState::{tab, tab2, dual_tabs, pick_tab}`, untere Leiste `LOWER_TABS` = Control-ID 1,
`GMSYNTH_PORTRAIT` 16 U in `src/ui/rack.rs`), FILTER mit 7 Spalten (EXTRA eigene Spalte). Wartet auf Handy-Test.
Schritt 2 gebaut, wartet auf Handy-Test: Wellenform-Vorschau pro Oszillator auf der OSC-Seite (`wave_a`/`wave_b` in
`src/panels/inst/gm_synth.rs`, Daten `gmsynth::preview_cycle`; folgt Shape-Auswahl, BALANCE-Anteil, PHASE B).
Klang-Befund: Besitzer:in meldete, zwei Saw-Wellen im GM Synth (Regler ziehen) klingen „kaputt". Bit-genauer Vergleich
(Original ed8fb6c vs. Fork, über `SongAudioEngine` und direkt, alle Osc-Mod-Arten × 3 Filter, Saw/Saw bei vielen
FINE-B/BALANCE-Werten, EXTRA Off) ist identisch; die unveränderte Original-Debug-App und die Original-Honeydew-App klingen
genauso, **FL Studio Mobile klingt dort sauberer**. Also ein bestehender Unterschied zum Vorbild, kein Fehler des Forks.
Das GM-Synth-„FM" ist Phasenmodulation (A wird bei φ_A + (B+1)·m gelesen). Vergleichsbuild eines beliebigen Commits:
Actions → Android debug APK → Run workflow, Feld `ref` (gleiche App-ID und Signatur, installiert über die Debug-App).

Rückmeldung zu Schritt 2 (Screenshot): Tabs funktionieren; Wellen-Vorschau als Entwurf ok, soll hochauflösender, zentriert,
ohne Anstoßen am Rand sein und auf FM reagieren; Layout verschwenderisch. Umgesetzt: `gmsynth::FM_RANGE` (CSTM 32100,
x1/x4/x8/x16, x1 = Original), Vorschau mit Pixelauflösung und `curve_a` (Ring/FM/Sync).
Schritt A (Knobs, Wellen-Auswahl per Tippen, Ein-/Zwei-Seiten-Schalter) auf dem Handy getestet: „sieht gut aus".
Offene Wünsche daraus: Platz besser nutzen (Knobs größer, Pfeil-Menüs kompakter), Header sparen, Filter-Vorschau, Envelopes/LFOs.
Schritt A2 gebaut (wartet auf Handy-Test): kompakte Menüs (`Leaf::compact`/`.compact()`, `Control::compact`,
`paint_compact` in `src/ui/widgets/spinner.rs`: Kasten mit Label über Wert, Tippen irgendwo öffnet die Liste), niedrige
Menü-Reihe (`MENUS` = 2 U) unter größeren Knobs; FILTER-Seite mit Frequenzgang-Anzeige (`gmsynth::filter_response`,
per Zeitbereichsmessung des echten Filters geprüft; Ziehen setzt CUTOFF (x) und RES (y)); Envelope-Anzeigen auf LEVEL,
EG und MOD (`gmsynth::envelope_preview` lässt die echte `Envelope` mit 1 kHz laufen) und LFO-Anzeige (`lfo_preview`),
alle nur Anzeige (Punkte ziehen und freies Mapping noch offen, siehe Roadmap 3 und 4). Karte 18 U.
Rückmeldung zum Test: FM RANGE bringt hörbar etwas; die Auswahlmenüs (Popups) waren hell/weiß und passten nicht zur UI ->
in `src/ui/popup.rs` jetzt `menuFill`-Panel mit `text`-Labels (Honeydew-Änderung, FL Mobile ist invertiert), der aktuelle Eintrag
ein `background`-Balken unter vollem `text`; Hilfen `popup::panel_fill` und `popup::label_ink` (Tests nutzen sie).
Hinweis: Die A-Vorschau bei starker FM (x16) wirkt „gestaucht" – das ist der echte FM-Verlauf (B ist spitz), kein Zeichenfehler.
Entschieden: Modulationsanzeige am Knob = farbiger Ring, der sich mit dem Amount bewegt und von der Grundstellung abhebt
(kommt mit Schritt B, den Envelopes/LFOs und dem Mapping). Header (CARD_HEADER 1.5 U, global) noch nicht verkleinert.

Feedback Tag 2 (Screenshot FILTER/OSC): (1) Tab-Reihenfolge logisch: LEVEL, OSC, MOD, FILTER, EG, LFO, VOICES, FX
(umgesetzt, Seitenindex 0..7 in dieser Reihenfolge). (2) FILTER: Anzeige höher, Knobs größer, Menüs/KBD klein in einer
Zeile (umgesetzt: einzeilige kompakte Menüs `paint_compact`, `text_toggle` für KBD/VEL/MONO/AUTO, `MENUS` = 1.4 U);
Resonanz der Anzeige per Zeitbereichstest gegen den echten Filter geprüft (stimmt, war nur zu flach: Bereich jetzt
−30…+24 dB). WUNSCH OFFEN: Spektrum-Analyser als Hintergrund der Filter-Anzeige, der auf den Filter reagiert (der
Analyzer-Effekt existiert; Instrumente liefern bisher keine Analyse: `AudioRenderer::analysis` ist für Instrumente leer,
`device.rs` `Visual`-Slab pro Meter, Größe je Effekttyp; für den GM Synth bräuchte es `Instrument::analysis()` + Anbindung
in `audio_engine/mod.rs` und `live_analysis`). (3) OSC: Anzeige beschneidet an den Rändern, ruckelt bei kleiner Frequenz von
B, Knobs größer (umgesetzt: genau ein ganzer Zyklus pro Anzeige, seitlicher Rand; Ruckeln kommt von den Halbtonstufen
von PITCH B). (4) MOD/LEVEL/EG: Anzeige höher; Attack wurde erst ab ~20 % sichtbar (umgesetzt: `envelope_preview` rechnet die
echten Exponentialkurven stufenweise mit festen Breitenanteilen A 22 %/D 30 %/Halten 13 %/R 35 %, echte Zeiten als Label,
gegen die echte `Envelope` geprüft). WUNSCH OFFEN LFO: Sample & Hold, Sync-Modi, „bei jeder Note neuer Zufallswert" mit frei
einstellbaren Grenzen, laufender Punkt auf der Kurve synchron zur Modulation (braucht die LFO-Phase aus der Engine, siehe
Analyse-Anbindung oben). WUNSCH OFFEN Farbsystem: übersichtliche, zum Theme passende, vordefinierte Kontraste (Vorbild
Massive X von Native Instruments), damit die Module nicht monoton wirken, ohne die FL-Mobile-Optik ganz zu verlieren.

7. **Oszillator-Klangqualität im GM Synth angleichen an FL Mobile** (später): Hypothesen, noch ungeprüft: der Tabellen-
   lesezugriff `gm_waveshapes::read` nimmt den nächsten Tabellenwert plus Dither (1024 Punkte) statt zu interpolieren;
   Aliasing der bandbegrenzten Tabellen bei hohen Tönen und starker FM; Detune zweier Saw-Wellen. Vorgehen: Spektrum/
   Rauschabstand offline messen (Test rendert Saw/Saw), Verbesserungen (Interpolation, mehr Tabellenpunkte,
   Oversampling bei FM) einzeln mit Messung und Hörtest des/der Besitzer:in.
