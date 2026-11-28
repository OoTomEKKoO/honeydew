# Rücksprung-Punkte (bekannte gute Stände)

Jeder Eintrag ist ein Commit im Branch `claude/honeydew-fork-setup-b6pc8o`, von dem eine APK gebaut wurde und der sich auf dem Handy
des Besitzers bewährt hat (oder getestet wurde). Der Branch wird nie überschrieben (kein Force-Push), alle Commits bleiben erhalten.
Git-Tags lassen sich aus der Cloud-Sitzung nicht auf GitHub setzen (der Proxy erlaubt es nicht), deshalb steht der Commit hier.

## So kommt man zurück

Sag Claude: **„Geh zurück auf Stand `<Commit>`"**. Das passiert ohne Verlust: Claude legt einen neuen Commit an, der den Zustand
des alten Standes wiederherstellt (`git revert`), der Verlauf bleibt vollständig. Danach lässt sich jederzeit wieder nach vorn gehen.
Die APK eines alten Standes lässt sich auch ohne Zurücksetzen neu bauen: GitHub, Actions, „Android debug APK", „Run workflow", im Feld
`ref` den Commit eintragen.

## Die Stände (neuester oben)

| Commit | Datum | Was funktioniert | Build |
|---|---|---|---|
| `7162300` | 28.11. | Theme Graphite (invertierte Tastatur mit Schatten von oben, Bogen-Regler, Modulationsring mit Live-Punkt), Seiten-Routen sichtbar, Locks folgen Noten, Untermenüs für die Oszillatoren, Wavetables, Matrix, P-Locks | Actions-Lauf 46145534973 |
| `50a113a` | 28.11. | Graphite (erste Fassung), glatte Wavetable-Position (kein Knistern), LFO-Hinweis | Lauf 46136772112 |
| `9ef5e41` | 27.11. | Wavetable-Oszillatoren (12 Werks-Tabellen, POS A/B, 3D-Vorschau) | Lauf 46074618290 |
| `43653b9` | 27.11. | Drive stärker, Matrix mit Off, Modulate-Dialog, Lock-Lane im Drum-Editor-Look, Reset mit zweitem Finger | Lauf 46069301189 |
| `45d52a7` | 27.11. | Erste P-Locks im Piano Roll | Lauf 46065487648 |
| `9115dd9` | 27.11. | Mod-Matrix, Modulationsring, Inspektor, Filter-DRIVE | Lauf 46062946158 |
| `92b3eb3` | 27.11. | GM Synth Seiten: feineres Spektrum, bewegter Filter, LFO-Notenwerte | Lauf 46057166993 |
| `a053c5b` | 27.11. | GM Synth: neue Seiten, Hüllkurven-Anzeige, LFO-Modi | Lauf 46051083907 |
| `ed8fb6c` | 20.11. | **Unverändertes Honeydew 0.5.5** (Original, Ausgangspunkt) | per `ref` baubar |

Neue Einträge trägt Claude nach jedem Build ein, der sich auf dem Handy bewährt hat.
