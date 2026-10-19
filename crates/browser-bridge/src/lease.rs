use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaseState {
    Pending,
    Accepted,
    Revoked,
    Expired,
}
#[derive(Debug)]
pub enum LeaseFailure<E> {
    Expired,
    Revoked,
    AlreadyAccepted,
    Action(E),
}
#[derive(Clone, Debug)]
pub struct CaptureLease {
    inner: Arc<Mutex<LeaseState>>,
    deadline: Instant,
}
impl CaptureLease {
    pub fn new(deadline: Instant) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LeaseState::Pending)),
            deadline,
        }
    }
    fn expire(&self, state: &mut LeaseState) {
        if *state == LeaseState::Pending && Instant::now() >= self.deadline {
            *state = LeaseState::Expired;
        }
    }
    pub fn state(&self) -> LeaseState {
        let mut state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        self.expire(&mut state);
        *state
    }
    pub fn revoke(&self) -> LeaseState {
        let mut state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        self.expire(&mut state);
        if *state == LeaseState::Pending {
            *state = LeaseState::Revoked;
        }
        *state
    }
    pub fn try_accept<T, E>(
        &self,
        action: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, LeaseFailure<E>> {
        let mut state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        self.expire(&mut state);
        match *state {
            LeaseState::Expired => Err(LeaseFailure::Expired),
            LeaseState::Revoked => Err(LeaseFailure::Revoked),
            LeaseState::Accepted => Err(LeaseFailure::AlreadyAccepted),
            LeaseState::Pending => {
                let value = action().map_err(LeaseFailure::Action)?;
                *state = LeaseState::Accepted;
                Ok(value)
            }
        }
    }
}
