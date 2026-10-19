use super::*;
#[test]
fn existing_intent_maps_to_an_actor_action_without_executing_it() {
    let intent = DownloadIntent::Existing {
        job_id: rusty_engine::JobId(12),
        action: ExistingAction::Restart,
    };
    assert!(matches!(
        intent.into_submission(),
        Some(rusty_engine::submissions::SubmissionAction::Restart(
            rusty_engine::JobId(12)
        ))
    ));
    assert!(DownloadIntent::Cancelled.into_submission().is_none());
}
