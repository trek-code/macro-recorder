use crate::types::{Macro, MacroEvent, MacroEventData};

/// Remove mouse-move events that aren't immediately before a click/scroll/release.
/// Keeps only the last move before each action, and removes all other moves.
pub fn compress(m: &Macro) -> Macro {
    let events = &m.events;
    let n = events.len();

    // Mark which MouseMove events to keep
    let mut keep = vec![true; n];

    for i in 0..n {
        if let MacroEventData::MouseMove { .. } = &events[i].event {
            // Look ahead: keep only if the next non-move event is a press/release/scroll
            let mut next_action = false;
            let mut all_moves = true;
            for j in (i + 1)..n {
                match &events[j].event {
                    MacroEventData::MouseMove { .. } => continue,
                    MacroEventData::MousePress { .. }
                    | MacroEventData::MouseRelease { .. }
                    | MacroEventData::Scroll { .. } => {
                        next_action = true;
                        all_moves = false;
                        break;
                    }
                    _ => {
                        all_moves = false;
                        break;
                    }
                }
            }
            // Keep this move only if it's immediately before an action
            // i.e., the very last move in a sequence leading to a click
            if !next_action {
                keep[i] = false;
                continue;
            }
            // Also drop if there's a later move before the same action
            if !all_moves {
                for j in (i + 1)..n {
                    match &events[j].event {
                        MacroEventData::MouseMove { .. } => {
                            // There's a later move before the action — drop this one
                            keep[i] = false;
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
    }

    let compressed: Vec<MacroEvent> = events
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, e)| e.clone())
        .collect();

    let duration_ms = compressed.last().map(|e| e.timestamp_ms).unwrap_or(0);

    Macro {
        name: m.name.clone(),
        events: compressed,
        duration_ms,
        tags: m.tags.clone(),
        description: m.description.clone(),
        relative_window: m.relative_window.clone(),
    }
}

#[tauri::command]
pub fn compress_macro(macro_data: Macro) -> Macro {
    compress(&macro_data)
}
