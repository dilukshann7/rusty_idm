#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Open,
    Closing,
    Closed,
}
impl Lifecycle {
    pub fn begin_close(&mut self, active: bool, confirmed: bool) -> bool {
        if *self != Self::Open || (active && !confirmed) {
            return false;
        }
        *self = Self::Closing;
        true
    }
    pub fn accepts_submissions(self) -> bool {
        self == Self::Open
    }
    pub fn engine_stopped(&mut self) {
        *self = Self::Closed;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declining_close_preserves_work_and_acceptance_waits_for_cleanup() {
        let mut state = Lifecycle::Open;
        assert!(!state.begin_close(true, false));
        assert!(state.accepts_submissions());
        assert!(state.begin_close(true, true));
        assert!(!state.accepts_submissions());
        assert_eq!(state, Lifecycle::Closing);
        assert!(!state.begin_close(true, true));
        state.engine_stopped();
        assert_eq!(state, Lifecycle::Closed);
    }
}
