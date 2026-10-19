use rusty_browser_bridge::framing::*;
use std::io::{self, Cursor, Read};
#[test]
fn frame_uses_utf8_byte_length_and_handles_fragmented_reads() {
    struct Fragment(Cursor<Vec<u8>>);
    impl Read for Fragment {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            self.0.read(&mut out[..1])
        }
    }
    let body = "https://example.org/தமிழ்".as_bytes();
    let mut wire = Vec::new();
    write_frame(&mut wire, body).unwrap();
    assert_eq!(
        u32::from_le_bytes(wire[..4].try_into().unwrap()) as usize,
        body.len()
    );
    assert_eq!(
        read_frame(&mut Fragment(Cursor::new(wire)))
            .unwrap()
            .unwrap(),
        body
    );
}
#[test]
fn oversize_prefix_is_rejected_before_reading_body() {
    struct Prefix(bool);
    impl Read for Prefix {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            assert!(!self.0);
            self.0 = true;
            out.copy_from_slice(&65537u32.to_le_bytes());
            Ok(4)
        }
    }
    assert!(read_frame(&mut Prefix(false)).is_err());
}
#[test]
fn eof_truncation_and_multiple_frames() {
    assert!(read_frame(&mut Cursor::new([])).unwrap().is_none());
    for bytes in [vec![1], vec![2, 0, 0, 0, 1]] {
        assert!(read_frame(&mut Cursor::new(bytes)).is_err());
    }
    let mut wire = Vec::new();
    write_frame(&mut wire, b"one").unwrap();
    write_frame(&mut wire, b"two").unwrap();
    let mut wire = Cursor::new(wire);
    assert_eq!(read_frame(&mut wire).unwrap().unwrap(), b"one");
    assert_eq!(read_frame(&mut wire).unwrap().unwrap(), b"two");
}
