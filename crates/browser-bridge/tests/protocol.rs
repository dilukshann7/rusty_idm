use rusty_browser_bridge::protocol::*;
fn capture() -> serde_json::Value {
    serde_json::json!({"v":1,"id":"12345678-1234-4234-8234-123456789abc","op":"capture","payload":{"url":"https://example.org/file?token=secret","source":"automatic","download_id":1,"expires_at_ms":123}})
}
#[test]
fn capture_validation_rejects_unsafe_fields() {
    let good = capture();
    assert!(Operation::decode(&serde_json::to_vec(&good).unwrap()).is_ok());
    let mut cases = Vec::new();
    for (key, value) in [
        ("v", serde_json::json!(2)),
        ("op", serde_json::json!("other")),
        ("extra", serde_json::json!(true)),
    ] {
        let mut v = good.clone();
        v[key] = value;
        cases.push(v);
    }
    for url in [
        "https://user:pass@example.org/".to_owned(),
        "https://example.org/\n".into(),
        format!("https://example.org/{}", "a".repeat(8192)),
    ] {
        let mut v = good.clone();
        v["payload"]["url"] = url.into();
        cases.push(v);
    }
    let mut v = good.clone();
    v["payload"]["filename_hint"] = "😀".repeat(128).into();
    cases.push(v);
    for v in cases {
        assert!(Operation::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}
#[test]
fn queries_preserved_and_debug_redacted() {
    let (_, op) = Operation::decode(&serde_json::to_vec(&capture()).unwrap()).unwrap();
    if let Operation::Capture(request) = op {
        assert!(request.url.contains("token=secret"));
        assert!(!format!("{request:?}").contains("secret"));
    } else {
        panic!()
    }
}
#[test]
fn refresh_context_is_optional_scoped_and_redacted() {
    let mut payload = capture();
    payload["payload"]["source_page"] = "https://example.org/downloads?session=private".into();
    payload["payload"]["request_context"] = serde_json::json!([{
        "url": "https://example.org/file?token=secret",
        "headers": [{"name": "Cookie", "value": "session=private"}]
    }]);
    let (_, operation) = Operation::decode(&serde_json::to_vec(&payload).unwrap()).unwrap();
    let Operation::Capture(request) = operation else { panic!("capture expected") };
    assert_eq!(request.request_context.len(), 1);
    assert!(!format!("{request:?}").contains("session=private"));
    let encoded = Operation::Capture(request).encode(RequestId::parse("12345678-1234-4234-8234-123456789abc").unwrap()).unwrap();
    assert!(Operation::decode(&encoded).is_ok());
    for (name, value) in [("Host", "example.org"), ("Cookie", "one\r\nHost: evil.org")] {
        let mut bad = payload.clone();
        bad["payload"]["request_context"][0]["headers"][0]["name"] = name.into();
        bad["payload"]["request_context"][0]["headers"][0]["value"] = value.into();
        assert!(Operation::decode(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}
