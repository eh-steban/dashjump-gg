use super::*;

// ---------------------------------------------------------------------------
// BS-1: boss_name_hash serializes as a JSON string, not a bare number
// ---------------------------------------------------------------------------
//
// Regression: BossSnapshot previously declared boss_name_hash as u64, which
// serde serialized as a JSON number. Every real fxhash is an 18-20 digit u64
// that exceeds JavaScript's Number.MAX_SAFE_INTEGER (2^53 - 1 = 9007199254740991).
// JSON.parse silently truncates such numbers to the nearest IEEE 754 double,
// corrupting the BOSS_NAME_HASH_MAP lookup and breaking boss-type labeling.
//
// This test pins the wire format as a JSON string (quoted). Any future revert
// of boss_name_hash to u64 will fail the quoted-literal assertion.
#[test]
fn test_boss_snapshot_hash_serializes_as_json_string() {
    let snapshot = BossSnapshot {
        entity_index: 42,
        custom_id: 21,
        boss_name_hash: "12946736302082733589".to_string(),
        team: 2,
        lane: 1,
        x: 100.0,
        y: 200.0,
        z: 0.0,
        spawn_time_s: 0,
        max_health: 5500,
        life_state_on_create: 0,
        death_time_s: None,
        life_state_on_delete: None,
    };

    let json = serde_json::to_string(&snapshot).expect("BossSnapshot must serialize");

    // The hash must appear as a quoted JSON string, not a bare number.
    // A bare number would look like: "boss_name_hash":12946736302082733589
    // A JSON string looks like:     "boss_name_hash":"12946736302082733589"
    assert!(
        json.contains(r#""boss_name_hash":"12946736302082733589""#),
        "boss_name_hash must be serialized as a quoted JSON string to preserve \
         u64 precision across the JavaScript boundary; got: {json}"
    );
}
