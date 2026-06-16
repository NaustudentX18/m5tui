//! Events fed into the pure reducer.
//!
//! M0 had only `Tick` and `Quit`. M1 adds the input layer (`Key`) and the
//! side-effect carrier (`Outgoing`) plus the `CloseOverlay` event used by
//! the framework to pop the palette/help modal.

/// Which pane of the cockpit has keyboard focus. Cycled by `Tab`:
/// `Prompt -> Agents -> Session -> Prompt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The bottom-bar prompt line. Char input lands here.
    Prompt,
    /// The left-pane agent list. Up/Down moves the selection.
    Agents,
    /// The right-pane session state. Read-only in M1.
    Session,
}

/// to one of these by `keymap::parse_chord`; the reducer only ever sees
/// `KeyAction` values, never raw scan codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Open the command palette. Also used as the second key of the `;/`
    /// chord (re-enter the palette with the chord prefix consumed).
    Palette,
    /// Open the hotkey/help overlay. Triggered by the `;?` chord.
    Help,
    /// Open the on-device theme editor. Triggered by the `;t` chord.
    OpenThemeEditor,
    /// Pop the current overlay, or in the cockpit move focus to the prompt.
    Esc,
    /// Submit the prompt or confirm a selection.
    Enter,
    /// Move selection up in the agent list.
    Up,
    /// Move selection down in the agent list.
    Down,
    Left,
    Right,
    /// Cycle focus: Prompt -> Agents -> Session -> Prompt.
    Tab,
    /// Pop the last char from the prompt.
    Backspace,
    /// Any other printable char. The reducer pushes it into the prompt
    /// when `focus == Focus::Prompt` and ignores it elsewhere.
    Char(char),
}

/// A side-effect the reducer wants the framework to perform. The reducer
/// itself stays pure; `step` is the side-channel variant that returns
/// these along with the next state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// Open the command palette modal.
    OpenPalette,
    /// Open the help overlay.
    OpenHelp,
    /// Close whichever modal is open and return to the cockpit.
    CloseOverlay,
    /// The user pressed Enter in the prompt with a non-empty message.
    /// The framework will route this to OMP (or the local echo in M1).
    SubmitPrompt(String),
    /// The user picked `quit` from the palette or pressed a quit chord.
    Quit,
    /// The user selected the agent at `index` in the agent list.
    SelectAgent(usize),
    /// The user pressed Tab; the framework may want to play a focus
    /// change sound or update a status line.
    CycleFocus(Focus),
}

/// All events the reducer accepts. `Tick` and `Quit` were M0; everything
/// else is M1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Tick,
    Quit,
    Key(KeyAction),
    Outgoing(Outgoing),
    /// The framework telling the reducer to drop whichever modal is open.
    /// This is a duplicate of `Outgoing(Outgoing::CloseOverlay)` but it
    /// keeps the public surface symmetric (one variant per direction).
    CloseOverlay,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_and_quit_are_distinct() {
        assert_ne!(Event::Tick, Event::Quit);
    }

    #[test]
    fn key_and_outgoing_are_distinct() {
        assert_ne!(
            Event::Key(KeyAction::Enter),
            Event::Outgoing(Outgoing::CloseOverlay),
        );
    }

    #[test]
    fn close_overlay_event_is_a_variant() {
        let e = Event::CloseOverlay;
        assert_eq!(e, Event::CloseOverlay);
    }

    #[test]
    fn keyaction_char_carries_payload() {
        let k = KeyAction::Char('h');
        match k {
            KeyAction::Char(c) => assert_eq!(c, 'h'),
            _ => panic!("expected Char variant"),
        }
    }
}
