//! Pure application state. The reducer is `Event -> AppState -> AppState`.

use crate::event::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub title: String,
    pub frame_count: u64,
    pub should_quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            title: "m5Tui v0.1.0".into(),
            frame_count: 0,
            should_quit: false,
        }
    }
}

/// Pure reducer. No I/O, no side effects.
pub fn reduce(state: AppState, event: Event) -> AppState {
    match event {
        Event::Tick => AppState {
            frame_count: state.frame_count + 1,
            ..state
        },
        Event::Quit => AppState {
            should_quit: true,
            ..state
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_title() {
        assert_eq!(AppState::default().title, "m5Tui v0.1.0");
    }

    #[test]
    fn default_has_zero_frame_count_and_no_quit() {
        let s = AppState::default();
        assert_eq!(s.frame_count, 0);
        assert!(!s.should_quit);
    }

    #[test]
    fn reduce_tick_increments_frame_count() {
        let s = AppState::default();
        let s2 = reduce(s, Event::Tick);
        assert_eq!(s2.frame_count, 1);
    }

    #[test]
    fn reduce_quit_sets_should_quit() {
        let s = AppState::default();
        let s2 = reduce(s, Event::Quit);
        assert!(s2.should_quit);
    }

    #[test]
    fn reduce_preserves_other_fields() {
        let s = AppState {
            title: "hello".into(),
            frame_count: 42,
            should_quit: false,
        };
        let s2 = reduce(s.clone(), Event::Tick);
        assert_eq!(s2.title, "hello");
        assert_eq!(s2.frame_count, 43);
        assert!(!s2.should_quit);
    }
}
