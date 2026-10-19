use rusty_browser_bridge::lease::*;
use std::{
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};
#[test]
fn lease_expiry_and_revoke_prevent_acceptance() {
    let expired = CaptureLease::new(Instant::now() - Duration::from_secs(1));
    assert!(expired
        .try_accept(|| -> Result<(), ()> { panic!("expired action") })
        .is_err());
    let revoked = CaptureLease::new(Instant::now() + Duration::from_secs(10));
    revoked.revoke();
    assert!(revoked
        .try_accept(|| -> Result<(), ()> { panic!("revoked action") })
        .is_err());
}
#[test]
fn accept_and_revoke_have_one_order() {
    let lease = CaptureLease::new(Instant::now() + Duration::from_secs(10));
    let other = lease.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b = barrier.clone();
    let thread = std::thread::spawn(move || {
        b.wait();
        other.revoke()
    });
    barrier.wait();
    let accepted = lease.try_accept(|| Ok::<_, ()>(7)).is_ok();
    let state = thread.join().unwrap();
    assert_eq!(accepted, state == LeaseState::Accepted);
}
