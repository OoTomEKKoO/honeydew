//! The piano roll's `Lock` item of the graph lane (Honeydew's own, [`crate::song::Lock`]): the pure parts. Which
//! parameters a channel's instruments offer, and how a painting finger sets the locks of the notes under it.
//! The drawing and the touch handling are `PianoRoll::lock_lane`.

use super::notes::note;
use crate::params::{ParamKind, module_spec};
use crate::song::{AutomationTarget, Channel, Clip};
use crate::ui::rack::is_instrument;

/// The parameters of `channel`'s instruments a note can lock, named as the automation names them: knobs and sliders
/// with a name (menus and switches would jump, and unnamed ones are internal), in rack order. With more than one
/// instrument in the rack each name carries its instrument's position.
pub fn lockable(channel: &Channel) -> Vec<(AutomationTarget, String)> {
    let instruments: Vec<_> = (channel.rack.modules.iter())
        .filter(|m| is_instrument(m.type_id))
        .collect();
    let mut out = Vec::new();
    for (n, module) in instruments.iter().enumerate() {
        let Some(spec) = module_spec(module.type_id) else {
            continue;
        };
        for (index, param) in spec.params.iter().enumerate() {
            let smooth = matches!(param.kind, ParamKind::Continuous { .. });
            if param.name.is_empty() || !smooth || index >= module.params.len() {
                continue;
            }
            let name = if instruments.len() > 1 {
                format!("{} ({})", param.name, n + 1)
            } else {
                param.name.to_owned()
            };
            out.push((
                AutomationTarget {
                    instance_id: module.instance_id,
                    param: index as i32,
                },
                name,
            ));
        }
    }
    out
}

/// `target`'s saved value on `channel` (what a note without a lock plays), if it has one.
pub fn base_value(channel: &Channel, target: AutomationTarget) -> Option<f32> {
    let module = (channel.rack.modules.iter()).find(|m| m.instance_id == target.instance_id)?;
    module
        .params
        .get(usize::try_from(target.param).ok()?)
        .copied()
}

/// Whether `target` rests in the middle (a knob whose default is 0.5: Pan, a pitch): its lock bars grow from the centre
/// band as the drum editor's Pan and Pitch do.
pub fn is_bipolar(channel: &Channel, target: AutomationTarget) -> bool {
    let module = (channel.rack.modules.iter()).find(|m| m.instance_id == target.instance_id);
    let spec = module.and_then(|m| module_spec(m.type_id));
    let param = spec.and_then(|s| s.params.get(usize::try_from(target.param).ok()?));
    param.is_some_and(|p| p.default == 0.5)
}

/// Locks `target` to `value` (0…1) on every eligible note whose extent [pos, pos + len) holds `tick`: all the notes, or
/// only the selected ones while any is selected (`selected` is parallel to the clip's events, as the other graph lane
/// items'). Returns whether a lock changed.
pub fn paint_locks(
    clip: &mut Clip,
    selected: &[bool],
    target: AutomationTarget,
    tick: f64,
    value: f32,
) -> bool {
    let any = selected.contains(&true);
    let mut painted = Vec::new();
    for (i, e) in clip.events.iter().enumerate() {
        let Some((pos, len, key)) = note(e) else {
            continue;
        };
        let eligible = !any || selected.get(i) == Some(&true);
        if eligible && pos <= tick && tick < pos + len {
            painted.push((pos as u32, key));
        }
    }
    let mut changed = false;
    for (pos, key) in painted {
        let now = clip.lock(pos, key, target).map(|l| l.value);
        if now != Some(value.clamp(0.0, 1.0)) {
            clip.set_lock(pos, key, target, value);
            changed = true;
        }
    }
    changed
}

/// Removes every lock of `target` in `clip`; returns whether there was one.
pub fn clear_locks(clip: &mut Clip, target: AutomationTarget) -> bool {
    let before = clip.file.locks.len();
    clip.file.locks.retain(|l| l.target != target);
    clip.file.locks.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{ChannelKind, Event, Song, SynthKind};

    fn clip_with_notes() -> Clip {
        let mut lane = crate::song::Lane::new(crate::song::LaneKind::Notes);
        let clip = lane.add_clip(0, 4.0);
        for (pos, key) in [(0, 60), (128, 62), (256, 64)] {
            clip.events.push(Event::Note {
                pos,
                len_beats: 1.0,
                key,
                velocity: 200,
                pan: 127,
                slide: false,
            });
        }
        clip.clone()
    }

    const T: AutomationTarget = AutomationTarget {
        instance_id: 3,
        param: 21,
    };

    #[test]
    fn painting_locks_the_notes_under_the_finger_or_only_the_selected_ones() {
        let mut clip = clip_with_notes();
        let none = [false; 3];
        assert!(paint_locks(&mut clip, &none, T, 130.0, 0.4));
        assert_eq!(clip.file.locks.len(), 1);
        assert_eq!(clip.lock(128, 62, T).unwrap().value, 0.4);
        assert!(
            !paint_locks(&mut clip, &none, T, 130.0, 0.4),
            "no change, no edit"
        );
        assert!(paint_locks(&mut clip, &none, T, 130.0, 2.0));
        assert_eq!(clip.lock(128, 62, T).unwrap().value, 1.0, "clamped");
        // With a selection only its notes take the value.
        let selected = [true, false, false];
        assert!(
            !paint_locks(&mut clip, &selected, T, 130.0, 0.1),
            "note 2 is not selected"
        );
        assert!(paint_locks(&mut clip, &selected, T, 10.0, 0.1));
        assert_eq!(clip.file.locks.len(), 2);
        // Off every note nothing happens.
        assert!(!paint_locks(&mut clip, &none, T, 5000.0, 0.5));
    }

    #[test]
    fn clearing_removes_only_that_parameters_locks() {
        let mut clip = clip_with_notes();
        let other = AutomationTarget { param: 22, ..T };
        paint_locks(&mut clip, &[false; 3], T, 130.0, 0.4);
        paint_locks(&mut clip, &[false; 3], other, 130.0, 0.6);
        assert!(clear_locks(&mut clip, T));
        assert_eq!(clip.file.locks.len(), 1);
        assert!(clip.lock(128, 62, other).is_some());
        assert!(!clear_locks(&mut clip, T));
    }

    #[test]
    fn a_gm_synth_offers_its_named_knobs_and_not_its_menus() {
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::GmSynth));
        let channel = &song.channels[1];
        let id = channel.rack.modules[0].instance_id;
        let names: Vec<_> = lockable(channel)
            .into_iter()
            .map(|(t, n)| (t.param, t.instance_id, n))
            .collect();
        assert!(names.iter().all(|(_, i, _)| *i == id));
        let has = |name: &str| names.iter().any(|(_, _, n)| n == name);
        assert!(has("Cutoff") && has("Resonance") && has("Level"));
        assert!(
            !has("Filter Kind") && !has("LFO Shape"),
            "menus jump, so they are not offered"
        );
        assert!(base_value(channel, lockable(channel)[0].0).is_some());
        // Pan rests in the middle, Cutoff does not.
        let find = |name: &str| {
            lockable(channel)
                .into_iter()
                .find(|(_, n)| n == name)
                .unwrap()
                .0
        };
        assert!(is_bipolar(channel, find("Pan")));
        assert!(!is_bipolar(channel, find("Cutoff")));
    }
}
