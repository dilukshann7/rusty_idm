/// Native bars have a signed 32-bit range; use u128 for safe scaling of u64 files.
pub fn scale(received: u64, total: Option<u64>, completed: bool) -> Option<i32> {
    match total {
        Some(0) => Some(if completed { 10_000 } else { 0 }),
        Some(total) => Some(((received as u128 * 10_000) / total as u128).min(10_000) as i32),
        None => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scales_maximum_files_without_overflow_or_invented_unknown_progress() {
        assert_eq!(scale(u64::MAX, Some(u64::MAX), false), Some(10_000));
        assert_eq!(scale(u64::MAX / 2, Some(u64::MAX), false), Some(4999));
        assert_eq!(scale(10, None, false), None);
        assert_eq!(scale(0, Some(0), true), Some(10_000));
        assert_eq!(scale(0, Some(0), false), Some(0));
    }
}
