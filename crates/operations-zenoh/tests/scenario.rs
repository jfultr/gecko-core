//! End-to-end scenario over a local Zenoh topology.

#[test]
fn scripted_scenario_passes() {
    let mut narrative = Vec::new();
    let result = operations_zenoh::demo::run(&mut narrative);
    let text = String::from_utf8_lossy(&narrative);
    assert!(result.is_ok(), "{result:?}\n{text}");
    assert!(text.contains("scenario complete"), "{text}");
}
