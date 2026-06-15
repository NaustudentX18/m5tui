//! Events fed into the pure reducer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Tick,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_and_quit_are_distinct() {
        assert_ne!(Event::Tick, Event::Quit);
    }

    #[test]
    fn event_is_copy() {
        let e = Event::Tick;
        let copy = e;
        assert_eq!(e, copy);
    }
}
