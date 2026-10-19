use super::*;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_RANGE, ETAG};

// Accepting any changed validator or wrong offset here could splice two files.
#[test]
fn resumed_response_requires_exact_range_total_and_strong_validator() {
    let mut headers = HeaderMap::new();
    headers.insert(ETAG, HeaderValue::from_static("\"v1\""));
    headers.insert(CONTENT_RANGE, HeaderValue::from_static("bytes 100-199/200"));
    assert!(valid_resume(&headers, 100, 200, "\"v1\"", Some(100)));
    assert!(!valid_resume(&headers, 99, 200, "\"v1\"", Some(100)));
    assert!(!valid_resume(&headers, 100, 201, "\"v1\"", Some(100)));
    assert!(!valid_resume(&headers, 100, 200, "\"v2\"", Some(100)));
    assert!(!valid_resume(&headers, 100, 200, "\"v1\"", Some(99)));
    headers.insert(ETAG, HeaderValue::from_static("W/\"v1\""));
    assert!(strong_etag(&headers).is_none());
    assert!(!valid_resume(&headers, 100, 200, "\"v1\"", Some(100)));
}

#[test]
fn malformed_or_partial_ranges_are_rejected() {
    for range in [
        "bytes 100-150/200",
        "bytes 100-200/200",
        "bytes 100-199/*",
        "bytes -1-199/200",
        "items 100-199/200",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_static("\"v1\""));
        headers.insert(CONTENT_RANGE, HeaderValue::from_str(range).unwrap());
        assert!(!valid_resume(&headers, 100, 200, "\"v1\"", None), "{range}");
    }
}

#[tokio::test]
async fn pause_and_speed_changes_wake_waiters_and_cancel_wakes_paused_transfer() {
    let control = TransferControl::new(CancellationToken::new());
    assert!(control.try_pause());
    assert!(control.settings.borrow().paused);
    control.set_speed_limit(Some(1024));
    assert_eq!(control.settings.borrow().bytes_per_second, 1024);
    assert!(control.try_resume());
    assert!(!control.settings.borrow().paused);
    assert!(control.try_pause());
    assert!(control.try_cancel());
    assert!(!control.wait_until_resumed().await);
    assert!(!control.try_resume());
}

#[tokio::test]
async fn limiter_wait_is_interruptible_by_live_change_and_cancellation() {
    let control = TransferControl::new(CancellationToken::new());
    control.set_speed_limit(Some(1));
    let other = control.clone();
    let task = tokio::spawn(async move { other.pace(16_384).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!task.is_finished());
    control.set_speed_limit(None);
    assert!(tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap());
    control.set_speed_limit(Some(1));
    let other = control.clone();
    let task = tokio::spawn(async move { other.pace(16_384).await });
    control.try_cancel();
    assert!(!tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap());
}

#[tokio::test]
async fn limiter_charges_time_before_delivering_bytes() {
    let control = TransferControl::new(CancellationToken::new());
    control.set_speed_limit(Some(10_000));
    let start = tokio::time::Instant::now();
    assert!(control.pace(1000).await);
    assert!(start.elapsed() >= Duration::from_millis(100));
    assert!(control.commit_file(|| Ok(())).unwrap());
    assert!(!control.try_pause());
}
