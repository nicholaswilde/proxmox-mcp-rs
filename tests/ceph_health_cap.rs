// The 1,024-byte cap on get_ceph_health. Every string copied from Proxmox is
// cut by its serialized JSON byte length (a character can take four UTF-8
// bytes, or six as a JSON escape), the lists are filled only while the whole
// answer still fits, and a final guard shrinks the answer in a fixed order with
// a tiny fallback. These tests check the bound through the handler, on each
// part alone, with escapes right at each budget, and on thousands of random
// inputs, and that the *_omitted counts always add up.
mod common;
use common::*;
use proxmox_mcp_rs::mcp::{
    summarize_ceph_status, summarize_ceph_status_from, summarize_ceph_status_within,
    CEPH_HEALTH_MAX_BYTES,
};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn size(v: &Value) -> usize {
    serde_json::to_string(v).unwrap().len()
}

/// The JSON form of a string, without its two quotes.
fn json_bytes(s: &str) -> usize {
    serde_json::to_string(s).unwrap().len() - 2
}

/// Budget per string, in serialized bytes, plus the 3-byte ellipsis.
const NAME_MAX: usize = 64 + 3;
const MESSAGE_MAX: usize = 120 + 3;
const SOURCE_MAX: usize = 96 + 3;

/// Characters that cost more serialized bytes than characters.
const HEAVY: [(&str, &str); 9] = [
    ("four-byte emoji", "🔥"),
    ("four-byte supplementary", "\u{10FFFF}"),
    ("three-byte CJK", "中"),
    ("two-byte accented", "é"),
    ("double quote", "\""),
    ("backslash", "\\"),
    ("control \\u0001", "\u{0001}"),
    ("control \\u001f", "\u{001f}"),
    ("mixed", "a\"🔥\u{0002}\\\n中"),
];

fn text_of(res: &Value) -> String {
    res["content"][0]["text"].as_str().unwrap().to_string()
}

/// 32 placement-group state combinations.
fn many_pg_states() -> Vec<Value> {
    let names = [
        "undersized",
        "degraded",
        "remapped",
        "backfill_wait",
        "recovering",
    ];
    (0..32)
        .map(|i| {
            let suffix: String = names
                .iter()
                .enumerate()
                .filter(|(bit, _)| i & (1 << bit) != 0)
                .map(|(_, s)| format!("+{}", s))
                .collect();
            json!({"state_name": format!("active{}", suffix), "count": 1})
        })
        .collect()
}

fn long_checks(n: usize) -> serde_json::Map<String, Value> {
    (0..n)
        .map(|i| {
            (
                format!("CHECK_{:02}", i),
                json!({"severity": if i == n - 1 { "HEALTH_ERR" } else { "HEALTH_WARN" },
                       "summary": {"message": "⚠".repeat(300)}}),
            )
        })
        .collect()
}

fn checks_obj(n: usize, severity: &str, msg_len: usize) -> Value {
    let mut m = serde_json::Map::new();
    for i in 0..n {
        m.insert(
            format!("CHECK_{i:03}"),
            json!({"severity": severity, "summary": {"message": "m".repeat(msg_len)}}),
        );
    }
    Value::Object(m)
}

fn pg_states(n: usize, name_len: usize) -> Vec<Value> {
    (0..n)
        .map(
            |i| json!({"state_name": format!("{i:03}{}", "s".repeat(name_len)), "count": 1000 + i}),
        )
        .collect()
}

fn one_check(name: &str, message: &str) -> Value {
    json!({"health": {"status": "HEALTH_WARN", "checks": {
        name: {"severity": "HEALTH_WARN", "summary": {"message": message}}
    }}})
}

async fn health_via_handler(node: Option<&str>, status: Value) -> Value {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;
    let route = match node {
        Some(n) => format!("/api2/json/nodes/{}/ceph/status", n),
        None => "/api2/json/cluster/ceph/status".to_string(),
    };
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": status})))
        .expect(1)
        .mount(&mock_server)
        .await;
    let args = match node {
        Some(n) => json!({"node": n}),
        None => json!({}),
    };
    let res = server.call_tool("get_ceph_health", &args).await.unwrap();
    let text = text_of(&res);
    assert!(
        text.len() <= CEPH_HEALTH_MAX_BYTES,
        "answer is {} bytes, cap is {}",
        text.len(),
        CEPH_HEALTH_MAX_BYTES
    );
    serde_json::from_str(&text).unwrap()
}

// ---------------------------------------------------------------------------
// Through the handler
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cap_holds_with_many_pg_states_and_no_checks() {
    // No checks, but enough PG states to pass 1 KB on their own.
    let v = health_via_handler(
        None,
        json!({
            "health": {"status": "HEALTH_WARN"},
            "quorum_names": ["pve1", "pve2", "pve3"],
            "osdmap": {"num_osds": 12, "num_up_osds": 12, "num_in_osds": 12},
            "pgmap": {"num_pgs": 32, "pgs_by_state": many_pg_states(),
                      "bytes_used": 250, "bytes_total": 1000}
        }),
    )
    .await;
    assert_eq!(v["health"], "HEALTH_WARN");
    assert_eq!(v["checks_omitted"], 0);
    assert_eq!(v["monitors"]["count"], 3);
    assert_eq!(v["monitors"]["quorum_omitted"], 0);
    assert_eq!(v["osds"]["total"], 12);
    assert_eq!(v["placement_groups"]["total"], 32);
    assert_eq!(v["usage"]["percent_used"], 25.0);
    assert_eq!(v["source"], "/cluster/ceph/status");
    let kept = v["placement_groups"]["by_state"].as_object().unwrap().len();
    let omitted = v["placement_groups"]["by_state_omitted"].as_u64().unwrap() as usize;
    assert!(kept > 0, "some PG states must still be listed");
    assert_eq!(kept + omitted, 32);
    assert!(omitted > 0, "32 states cannot all fit in 1 KB");
}

#[tokio::test]
async fn cap_holds_with_long_quorum_names_and_long_source() {
    // A 200-character node name makes the source path long, and each monitor
    // name is 100 characters.
    let node = "n".repeat(200);
    let names: Vec<String> = (0..5).map(|i| format!("{}{}", "m".repeat(99), i)).collect();
    let v = health_via_handler(
        Some(&node),
        json!({
            "health": {"status": "HEALTH_OK"},
            "quorum_names": names,
            "osdmap": {"num_osds": 3, "num_up_osds": 3, "num_in_osds": 3},
            "pgmap": {"num_pgs": 10, "pgs_by_state": [{"state_name": "active+clean", "count": 10}]}
        }),
    )
    .await;
    assert_eq!(v["health"], "HEALTH_OK");
    assert_eq!(v["monitors"]["count"], 5);
    let kept = v["monitors"]["quorum"].as_array().unwrap();
    let omitted = v["monitors"]["quorum_omitted"].as_u64().unwrap() as usize;
    assert_eq!(kept.len() + omitted, 5);
    for name in kept {
        let n = name.as_str().unwrap();
        assert!(
            n.chars().count() <= 65,
            "monitor name not shortened: {} chars",
            n.len()
        );
        assert!(n.ends_with('…'));
    }
    let source = v["source"].as_str().unwrap();
    assert!(source.starts_with("/nodes/nnnn"));
    assert!(source.chars().count() <= 97, "source not shortened");
    assert_eq!(v["osds"]["total"], 3);
    assert_eq!(v["placement_groups"]["total"], 10);
}

#[tokio::test]
async fn cap_holds_with_sixty_long_checks_through_handler() {
    let v = health_via_handler(
        Some("pve1"),
        json!({"health": {"status": "HEALTH_ERR", "checks": long_checks(60)}}),
    )
    .await;
    assert_eq!(v["health"], "HEALTH_ERR");
    assert_eq!(v["checks"][0]["severity"], "HEALTH_ERR");
    assert_eq!(v["checks"][0]["name"], "CHECK_59");
    assert!(v["checks"][0]["summary"].as_str().unwrap().ends_with('…'));
    let kept = v["checks"].as_array().unwrap().len();
    assert!(kept > 0);
    assert_eq!(v["checks_omitted"].as_u64().unwrap() as usize, 60 - kept);
    assert_eq!(v["source"], "/nodes/pve1/ceph/status");
}

#[tokio::test]
async fn cap_holds_with_everything_oversized_at_once() {
    // Long checks, many PG states, long monitor names and a long source
    // together; the health word and every numeric total must survive.
    let node = "x".repeat(300);
    let v = health_via_handler(
        Some(&node),
        json!({
            "health": {"status": "HEALTH_ERR", "checks": long_checks(60)},
            "quorum_names": (0..50).map(|i| format!("{}{}", "q".repeat(200), i)).collect::<Vec<_>>(),
            "osdmap": {"num_osds": 12, "num_up_osds": 11, "num_in_osds": 12},
            "pgmap": {"num_pgs": 32, "pgs_by_state": many_pg_states(),
                      "bytes_used": 333, "bytes_total": 1000}
        }),
    )
    .await;
    assert_eq!(v["health"], "HEALTH_ERR");
    assert_eq!(v["monitors"]["count"], 50);
    assert_eq!(v["osds"]["total"], 12);
    assert_eq!(v["osds"]["up"], 11);
    assert_eq!(v["osds"]["in"], 12);
    assert_eq!(v["placement_groups"]["total"], 32);
    assert_eq!(v["usage"]["bytes_used"], 333);
    assert_eq!(v["usage"]["bytes_total"], 1000);
    assert_eq!(v["usage"]["percent_used"], 33.3);
    let checks = v["checks"].as_array().unwrap().len();
    let names = v["monitors"]["quorum"].as_array().unwrap().len();
    let states = v["placement_groups"]["by_state"].as_object().unwrap().len();
    assert_eq!(checks + v["checks_omitted"].as_u64().unwrap() as usize, 60);
    assert_eq!(
        names + v["monitors"]["quorum_omitted"].as_u64().unwrap() as usize,
        50
    );
    assert_eq!(
        states + v["placement_groups"]["by_state_omitted"].as_u64().unwrap() as usize,
        32
    );
    assert!(v["checks_omitted"].as_u64().unwrap() > 0);
}

// ---------------------------------------------------------------------------
// Fixed part and absent fields
// ---------------------------------------------------------------------------

#[test]
fn absent_fields_are_left_out_not_null() {
    let bare = summarize_ceph_status_from(&json!({"health": {"status": "HEALTH_OK"}}), None);
    assert_eq!(bare["checks_omitted"], 0);
    assert_eq!(bare["checks"], json!([]));
    assert!(bare.get("source").is_none());
    assert!(bare["osds"].get("total").is_none());
    assert!(bare["monitors"].get("quorum").is_none());
    assert!(bare["monitors"].get("quorum_omitted").is_none());
    assert!(bare["placement_groups"].get("by_state").is_none());
    assert!(bare["placement_groups"].get("by_state_omitted").is_none());
}

#[test]
fn fixed_part_keeps_only_bounded_strings_and_numbers() {
    // A 5,000-character health word and non-numeric totals cannot blow the cap.
    let v = summarize_ceph_status_from(
        &json!({
            "health": {"status": "H".repeat(5000)},
            "osdmap": {"num_osds": "x".repeat(5000), "num_up_osds": 4, "num_in_osds": {"a": 1}},
            "pgmap": {"num_pgs": 1e300, "bytes_used": 5, "bytes_total": 10}
        }),
        Some(&"/".repeat(5000)),
    );
    let text = serde_json::to_string(&v).unwrap();
    assert!(text.len() <= CEPH_HEALTH_MAX_BYTES, "{} bytes", text.len());
    assert_eq!(v["health"].as_str().unwrap().chars().count(), 65);
    assert!(v["osds"].get("total").is_none());
    assert_eq!(v["osds"]["up"], 4);
    assert!(v["osds"].get("in").is_none());
    assert_eq!(v["usage"]["percent_used"], 50.0);
    assert_eq!(v["source"].as_str().unwrap().chars().count(), 97);
}

#[test]
fn fixed_fields_at_their_limits_stay_under_the_cap() {
    // Empty lists, six totals at f64::MAX, a 5,000-character health word and a
    // 300-character node in the source. Cutting strings by character count
    // instead of serialized bytes would give 1,028 bytes with four-byte
    // characters and 1,334 with control characters.
    for (label, c) in [
        ("four-byte Unicode", "🔥"),
        ("JSON-escaped control", "\u{0001}"),
    ] {
        let n = f64::MAX;
        let status = json!({"health": {"status": c.repeat(5000)}, "quorum_names": [],
            "osdmap": {"num_osds": n, "num_up_osds": n, "num_in_osds": n},
            "pgmap": {"num_pgs": n, "pgs_by_state": [], "bytes_used": n, "bytes_total": n}});
        let source = format!("/nodes/{}/ceph/status", c.repeat(300));
        let out = summarize_ceph_status_from(&status, Some(&source));
        let len = size(&out);
        assert!(len <= CEPH_HEALTH_MAX_BYTES, "{label}: {len} bytes");
        assert!(
            out.get("truncated").is_none(),
            "{label}: fallback not needed"
        );
        let health = out["health"].as_str().unwrap();
        assert!(health.ends_with('…') && json_bytes(health) <= NAME_MAX);
        assert!(out["source"].as_str().unwrap().starts_with("/nodes/"));
        assert_eq!(out["osds"]["total"], n);
        assert_eq!(out["usage"]["percent_used"], 100.0);
        assert_eq!(out["checks_omitted"], 0);
        assert_eq!(out["monitors"]["quorum_omitted"], 0);
        assert_eq!(out["placement_groups"]["by_state_omitted"], 0);
    }
}

#[test]
fn non_numeric_totals_are_left_out() {
    let v = summarize_ceph_status(&json!({
        "osdmap": {"num_osds": "12", "num_up_osds": {"x": 1}},
        "pgmap": {"bytes_used": {"x": 1}, "bytes_total": "9"}
    }));
    assert!(v["osds"]["total"].is_null());
    assert!(v["osds"]["up"].is_null());
    assert!(v["usage"]["bytes_used"].is_null());
    assert!(v["usage"]["bytes_total"].is_null());
    assert!(v["usage"]["percent_used"].is_null());
    assert!(size(&v) <= CEPH_HEALTH_MAX_BYTES);
}

#[test]
fn missing_health_key_gives_no_health_word_and_no_checks() {
    let v = summarize_ceph_status(&json!({"quorum_names": ["pve1"]}));
    assert!(
        v.get("health").is_none(),
        "absent health word must be left out"
    );
    assert_eq!(v["checks"], json!([]));
    assert_eq!(v["checks_omitted"], 0);
}

#[test]
fn pg_state_with_non_numeric_count_keeps_the_name_only() {
    let v = summarize_ceph_status(
        &json!({"pgmap": {"pgs_by_state": [{"state_name": "active+clean", "count": "many"}]}}),
    );
    let by_state = v["placement_groups"]["by_state"].as_object().unwrap();
    assert!(by_state.contains_key("active+clean"));
    assert!(
        by_state["active+clean"].is_null(),
        "a non-numeric count must not be copied"
    );
}

// ---------------------------------------------------------------------------
// Each variable part is bounded and counted
// ---------------------------------------------------------------------------

#[test]
fn many_long_checks_stay_within_the_cap() {
    let checks: serde_json::Map<String, Value> = (0..60)
        .map(|i| {
            (
                format!("MODULE_CHECK_{:03}", i),
                json!({"severity": "HEALTH_WARN",
                       "summary": {"message": "y".repeat(300)}}),
            )
        })
        .collect();
    let v = summarize_ceph_status(&json!({"health": {"status": "HEALTH_WARN", "checks": checks}}));
    let text = serde_json::to_string(&v).unwrap();
    assert!(
        text.len() <= 1024,
        "summary is {} bytes for 60 checks",
        text.len()
    );
}

#[test]
fn quorum_names_are_bounded_and_counted() {
    // 300 monitor names of 500 characters each: far more than fits in 1 KB.
    let names: Vec<String> = (0..300)
        .map(|i| format!("{i:03}{}", "q".repeat(500)))
        .collect();
    let v = summarize_ceph_status(&json!({"quorum_names": names}));
    assert!(
        size(&v) <= CEPH_HEALTH_MAX_BYTES,
        "answer is {} bytes",
        size(&v)
    );
    let kept = v["monitors"]["quorum"].as_array().unwrap();
    let omitted = v["monitors"]["quorum_omitted"].as_u64().unwrap() as usize;
    assert!(!kept.is_empty(), "some monitor names must still be listed");
    assert_eq!(kept.len() + omitted, 300);
    assert_eq!(
        v["monitors"]["count"], 300,
        "count stays the real quorum size"
    );
    for n in kept {
        assert!(n.as_str().unwrap().chars().count() <= 65);
    }
}

#[test]
fn pg_states_are_bounded_and_counted() {
    // 400 states with 300-character names: the state list must be cut.
    let v = summarize_ceph_status(
        &json!({"pgmap": {"num_pgs": 400, "pgs_by_state": pg_states(400, 300)}}),
    );
    assert!(
        size(&v) <= CEPH_HEALTH_MAX_BYTES,
        "answer is {} bytes",
        size(&v)
    );
    let kept = v["placement_groups"]["by_state"].as_object().unwrap().len();
    let omitted = v["placement_groups"]["by_state_omitted"].as_u64().unwrap() as usize;
    assert!(kept > 0);
    assert_eq!(kept + omitted, 400);
    assert_eq!(v["placement_groups"]["total"], 400);
}

#[test]
fn long_source_is_shortened_and_answer_stays_small() {
    let src = "/nodes/".to_string() + &"n".repeat(10_000) + "/ceph/status";
    let v = summarize_ceph_status_from(&json!({"health": {"status": "HEALTH_OK"}}), Some(&src));
    assert!(size(&v) <= CEPH_HEALTH_MAX_BYTES);
    let s = v["source"].as_str().unwrap();
    assert_eq!(s.chars().count(), 97, "96 characters plus the ellipsis");
    assert!(s.ends_with('…'));
}

#[test]
fn huge_health_word_is_shortened() {
    let v = summarize_ceph_status(&json!({"health": {"status": "H".repeat(5000)}}));
    assert!(size(&v) <= CEPH_HEALTH_MAX_BYTES);
    let h = v["health"].as_str().unwrap();
    assert!(h.ends_with('…'));
    assert!(h.chars().count() <= 65);
}

#[test]
fn nothing_omitted_when_everything_fits() {
    let v = summarize_ceph_status(&json!({
        "health": {"status": "HEALTH_WARN", "checks": checks_obj(2, "HEALTH_WARN", 20)},
        "quorum_names": ["pve1", "pve2", "pve3"],
        "pgmap": {"num_pgs": 3, "pgs_by_state": pg_states(2, 5)}
    }));
    assert_eq!(v["checks"].as_array().unwrap().len(), 2);
    assert_eq!(v["checks_omitted"], 0);
    assert_eq!(v["monitors"]["quorum"].as_array().unwrap().len(), 3);
    assert_eq!(v["monitors"]["quorum_omitted"], 0);
    assert_eq!(
        v["placement_groups"]["by_state"].as_object().unwrap().len(),
        2
    );
    assert_eq!(v["placement_groups"]["by_state_omitted"], 0);
}

#[test]
fn check_counts_add_up_for_many_sizes() {
    for n in [0usize, 1, 5, 20, 60, 200] {
        let v = summarize_ceph_status(
            &json!({"health": {"status": "HEALTH_ERR", "checks": checks_obj(n, "HEALTH_WARN", 300)}}),
        );
        assert!(
            size(&v) <= CEPH_HEALTH_MAX_BYTES,
            "n={n}: {} bytes",
            size(&v)
        );
        let kept = v["checks"].as_array().unwrap().len();
        assert_eq!(
            kept as u64 + v["checks_omitted"].as_u64().unwrap(),
            n as u64,
            "n={n}"
        );
    }
}

#[test]
fn severe_checks_are_kept_before_warnings_when_cut() {
    // 40 long warnings sort first alphabetically; one error sorts last. The error must survive the cut.
    let mut checks = checks_obj(40, "HEALTH_WARN", 200);
    checks.as_object_mut().unwrap().insert(
        "ZZZ_ERROR".into(),
        json!({"severity": "HEALTH_ERR", "summary": {"message": "down"}}),
    );
    let v = summarize_ceph_status(&json!({"health": {"status": "HEALTH_ERR", "checks": checks}}));
    assert!(
        v["checks_omitted"].as_u64().unwrap() > 0,
        "the cut must actually happen"
    );
    assert_eq!(v["checks"][0]["name"], "ZZZ_ERROR");
    assert_eq!(v["checks"][0]["severity"], "HEALTH_ERR");
}

#[test]
fn checks_win_over_pg_states_when_both_are_large() {
    // Warnings matter more to an agent than the PG state breakdown, so with
    // both oversized some checks are kept and PG states are the ones cut.
    let v = summarize_ceph_status(&json!({
        "health": {"status": "HEALTH_WARN", "checks": long_checks(8)},
        "pgmap": {"num_pgs": 32, "pgs_by_state": many_pg_states()}
    }));
    assert!(serde_json::to_string(&v).unwrap().len() <= CEPH_HEALTH_MAX_BYTES);
    assert!(!v["checks"].as_array().unwrap().is_empty());
    assert!(v["placement_groups"]["by_state_omitted"].as_u64().unwrap() > 0);
}

#[test]
fn two_pg_states_cut_to_the_same_name_count_one_as_omitted() {
    let name = |tail: &str| format!("{}{tail}", "s".repeat(64));
    let v = summarize_ceph_status_from(
        &json!({"pgmap": {"pgs_by_state": [
            {"state_name": name("A"), "count": 1},
            {"state_name": name("B"), "count": 2},
            {"state_name": "active+clean", "count": 3}
        ]}}),
        None,
    );
    let by_state = v["placement_groups"]["by_state"].as_object().unwrap();
    assert_eq!(by_state.len(), 2);
    assert_eq!(by_state[&name("…")], 1, "the first of the two wins");
    assert_eq!(by_state["active+clean"], 3);
    assert_eq!(v["placement_groups"]["by_state_omitted"], 1);
}

// ---------------------------------------------------------------------------
// Strings are cut by serialized bytes
// ---------------------------------------------------------------------------

#[test]
fn check_messages_of_four_byte_characters_are_cut_by_serialized_bytes() {
    // Each message is 300 flames (4 bytes each). The cut is by serialized
    // JSON bytes, not characters: 120 bytes is 30 flames, plus the 3-byte
    // ellipsis.
    let mut m = serde_json::Map::new();
    for i in 0..30 {
        m.insert(
            format!("FIRE_{i:03}"),
            json!({"severity": "HEALTH_WARN", "summary": {"message": "🔥".repeat(300)}}),
        );
    }
    let v = summarize_ceph_status(
        &json!({"health": {"status": "HEALTH_WARN", "checks": Value::Object(m)}}),
    );
    assert!(
        size(&v) <= CEPH_HEALTH_MAX_BYTES,
        "answer is {} bytes",
        size(&v)
    );
    assert!(!v["checks"].as_array().unwrap().is_empty());
    for c in v["checks"].as_array().unwrap() {
        let s = c["summary"].as_str().unwrap();
        assert_eq!(s.chars().count(), 31, "30 flames plus the ellipsis");
        assert_eq!(serde_json::to_string(s).unwrap().len() - 2, 123);
    }
}

#[test]
fn every_string_field_is_cut_by_serialized_bytes() {
    for (label, c) in HEAVY {
        let long = |i: usize| format!("{i}{}", c.repeat(2000));
        let checks: serde_json::Map<String, Value> = (0..5)
            .map(|i| {
                (
                    long(i),
                    json!({"severity": long(i), "summary": {"message": long(i)}}),
                )
            })
            .collect();
        let states: Vec<Value> = (0..5)
            .map(|i| json!({"state_name": long(i), "count": 7}))
            .collect();
        let n = f64::MAX;
        let status = json!({
            "health": {"status": long(0), "checks": checks},
            "quorum_names": (0..5).map(long).collect::<Vec<_>>(),
            "osdmap": {"num_osds": n, "num_up_osds": n, "num_in_osds": n},
            "pgmap": {"num_pgs": n, "pgs_by_state": states, "bytes_used": n, "bytes_total": n}
        });
        let v = summarize_ceph_status_from(&status, Some(&long(0)));
        assert!(
            size(&v) <= CEPH_HEALTH_MAX_BYTES,
            "{label}: {} bytes",
            size(&v)
        );
        assert!(v.get("truncated").is_none(), "{label}");
        let health = v["health"].as_str().unwrap();
        assert!(
            health.ends_with('…') && json_bytes(health) <= NAME_MAX,
            "{label}: health"
        );
        let source = v["source"].as_str().unwrap();
        assert!(
            source.ends_with('…') && json_bytes(source) <= SOURCE_MAX,
            "{label}: source"
        );
        let checks = v["checks"].as_array().unwrap();
        for ch in checks {
            for (field, max) in [
                ("name", NAME_MAX),
                ("severity", NAME_MAX),
                ("summary", MESSAGE_MAX),
            ] {
                let s = ch[field].as_str().unwrap();
                assert!(
                    s.ends_with('…') && json_bytes(s) <= max,
                    "{label}: check {field}"
                );
            }
        }
        let names = v["monitors"]["quorum"].as_array().unwrap();
        for name in names {
            let s = name.as_str().unwrap();
            assert!(
                s.ends_with('…') && json_bytes(s) <= NAME_MAX,
                "{label}: monitor"
            );
        }
        let by_state = v["placement_groups"]["by_state"].as_object().unwrap();
        for (key, count) in by_state {
            assert!(
                key.ends_with('…') && json_bytes(key) <= NAME_MAX,
                "{label}: PG state"
            );
            assert_eq!(*count, 7);
        }
        assert_eq!(
            checks.len() + v["checks_omitted"].as_u64().unwrap() as usize,
            5
        );
        assert_eq!(
            names.len() + v["monitors"]["quorum_omitted"].as_u64().unwrap() as usize,
            5
        );
        assert_eq!(
            by_state.len() + v["placement_groups"]["by_state_omitted"].as_u64().unwrap() as usize,
            5
        );
        assert_eq!(v["osds"]["in"], n, "{label}: totals are kept");
    }
}

#[test]
fn two_byte_escape_that_would_cross_the_message_budget_is_not_split() {
    // 119 plain bytes, then "\n" (2 bytes): 121 > 120, so the cut falls before "\n".
    let msg = format!("{}\n{}", "a".repeat(119), "b".repeat(200));
    let v = summarize_ceph_status(&one_check("CHK", &msg));
    let s = v["checks"][0]["summary"].as_str().unwrap();
    assert_eq!(s, format!("{}…", "a".repeat(119)));
    assert_eq!(json_bytes(s), 122);
}

#[test]
fn six_byte_escape_that_would_cross_the_message_budget_is_cut_before_it() {
    // 115 plain bytes, then "\u0001" (6 bytes): 121 > 120.
    let msg = format!("{}\u{1}{}", "a".repeat(115), "b".repeat(200));
    let v = summarize_ceph_status(&one_check("CHK", &msg));
    let s = v["checks"][0]["summary"].as_str().unwrap();
    assert_eq!(s, format!("{}…", "a".repeat(115)));
    assert!(!s.contains('\u{1}'));
}

#[test]
fn escape_that_ends_exactly_on_the_message_budget_is_kept_whole() {
    // 114 plain bytes, then "\u0001" (6 bytes): exactly 120, so no cut and no ellipsis.
    let msg = format!("{}\u{1}", "a".repeat(114));
    let v = summarize_ceph_status(&one_check("CHK", &msg));
    assert_eq!(v["checks"][0]["summary"].as_str().unwrap(), msg);
}

#[test]
fn quote_and_backslash_mix_at_the_message_budget_stays_valid() {
    // Alternating quote and backslash (2 bytes each) past the budget: the cut is
    // on a character boundary and the answer is still valid JSON that re-parses.
    let msg = "\"\\".repeat(200);
    let v = summarize_ceph_status(&one_check("CHK", &msg));
    let s = v["checks"][0]["summary"].as_str().unwrap();
    assert!(s.ends_with('…'));
    assert_eq!(
        json_bytes(s) - 3,
        120,
        "120 budget bytes, then the ellipsis"
    );
    let round: Value = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
    assert_eq!(round, v);
}

#[test]
fn escape_that_would_cross_the_name_budget_is_cut_before_it() {
    // 63 plain bytes, then '"' (2 bytes): 65 > 64.
    let name = format!("{}\"{}", "a".repeat(63), "b".repeat(10));
    let v = summarize_ceph_status(&one_check(&name, "m"));
    assert_eq!(
        v["checks"][0]["name"].as_str().unwrap(),
        format!("{}…", "a".repeat(63))
    );
}

#[test]
fn newline_in_a_pg_state_name_at_the_budget_is_cut_before_it() {
    // 63 plain bytes, then "\n" (2 bytes): 65 > 64. The state name is the map key.
    let state = format!("{}\n{}", "a".repeat(63), "b".repeat(10));
    let s = json!({"pgmap": {"num_pgs": 1, "pgs_by_state": [{"state_name": state, "count": 1}]}});
    let v = summarize_ceph_status(&s);
    let by_state = v["placement_groups"]["by_state"].as_object().unwrap();
    assert_eq!(by_state.len(), 1);
    assert!(by_state.contains_key(&format!("{}…", "a".repeat(63))));
}

#[test]
fn control_escape_at_the_health_word_budget_keeps_a_valid_word() {
    // The health word is cut at 64 bytes, so a valid word is never touched by it.
    // A word with a control escape past the budget is cut before the escape.
    let word = format!("{}\u{1}", "H".repeat(63));
    let v = summarize_ceph_status(&json!({"health": {"status": word}}));
    assert_eq!(
        v["health"].as_str().unwrap(),
        format!("{}…", "H".repeat(63))
    );
    let valid = summarize_ceph_status(&json!({"health": {"status": "HEALTH_ERR"}}));
    assert_eq!(valid["health"], "HEALTH_ERR");
}

#[test]
fn backslash_that_would_cross_the_source_budget_is_cut_before_it() {
    // 95 plain bytes, then "\\" (2 bytes): 97 > 96.
    let src = format!("{}\\{}", "a".repeat(95), "b".repeat(50));
    let v = summarize_ceph_status_from(&json!({}), Some(src.as_str()));
    assert_eq!(
        v["source"].as_str().unwrap(),
        format!("{}…", "a".repeat(95))
    );
}

#[test]
fn mixed_escapes_near_the_cap_are_packed_and_counts_add_up() {
    // Forty checks, each with a different mix of plain text, emoji, quotes,
    // backslashes, a newline and a control character, sized around the budgets.
    let mut checks = serde_json::Map::new();
    for i in 0..40 {
        let msg = match i % 5 {
            0 => "🔥".repeat(60 + i),
            1 => "\"q\\".repeat(40 + i),
            2 => "line\n".repeat(30 + i),
            3 => "\u{1}x".repeat(25 + i),
            _ => "plain text ".repeat(10 + i),
        };
        checks.insert(
            format!("CHECK_{i:02}_{}", "n\"".repeat(10)),
            json!({"severity": if i % 7 == 0 { "HEALTH_ERR" } else { "HEALTH_WARN" },
                   "summary": {"message": msg}}),
        );
    }
    let s = json!({"health": {"status": "HEALTH_ERR", "checks": Value::Object(checks)}});
    let v = summarize_ceph_status(&s);
    assert!(
        size(&v) <= CEPH_HEALTH_MAX_BYTES,
        "answer is {} bytes",
        size(&v)
    );
    let kept = v["checks"].as_array().unwrap().len();
    let omitted = v["checks_omitted"].as_u64().unwrap() as usize;
    assert_eq!(
        kept + omitted,
        40,
        "kept + omitted must equal the checks sent"
    );
    assert!(omitted > 0, "forty long checks must not all fit");
    // The fill stops only when the next check would not fit, so the space left
    // over is less than one check (at most about 260 bytes with these budgets).
    let slack = CEPH_HEALTH_MAX_BYTES - size(&v);
    assert!(
        slack < 300,
        "{slack} bytes left unused; the fill stopped too early"
    );
}

#[test]
fn every_kept_string_is_within_its_budget_with_mixed_escapes() {
    let mut checks = serde_json::Map::new();
    for i in 0..12 {
        checks.insert(
            format!("N{i}{}", "\\\"\n".repeat(40)),
            json!({"severity": "HEALTH_WARN", "summary": {"message": "\u{1}🔥\"".repeat(60)}}),
        );
    }
    let s = json!({"health": {"status": "HEALTH_WARN", "checks": Value::Object(checks)}});
    let v = summarize_ceph_status_from(&s, Some("\\\"\u{2}🔥".repeat(60).as_str()));
    assert!(size(&v) <= CEPH_HEALTH_MAX_BYTES);
    for c in v["checks"].as_array().unwrap() {
        assert!(json_bytes(c["name"].as_str().unwrap()) <= 64 + 3);
        assert!(json_bytes(c["summary"].as_str().unwrap()) <= 120 + 3);
    }
    assert!(json_bytes(v["source"].as_str().unwrap()) <= 96 + 3);
}

// ---------------------------------------------------------------------------
// Shrink order and fallback
// ---------------------------------------------------------------------------

#[test]
fn valid_health_word_survives_at_every_cap_down_to_the_fallback() {
    let status = json!({"health": {"status": "HEALTH_WARN", "checks": {"X": {"severity": "HEALTH_WARN", "summary": {"message": "m".repeat(200)}}}},
        "quorum_names": ["a", "b", "c"], "osdmap": {"num_osds": 12, "num_up_osds": 11, "num_in_osds": 12},
        "pgmap": {"num_pgs": 32, "pgs_by_state": [{"state_name": "active+clean", "count": 32}], "bytes_used": 1, "bytes_total": 4}});
    for cap in (1..=CEPH_HEALTH_MAX_BYTES).rev() {
        let v = summarize_ceph_status_within(&status, Some("/cluster/ceph/status"), cap);
        assert_eq!(v["health"], "HEALTH_WARN", "cap {cap}");
        assert_eq!(v["osds"]["up"], 11, "cap {cap}");
        assert_eq!(v["usage"]["percent_used"], 25.0, "cap {cap}");
        if v.get("truncated").is_none() {
            assert!(size(&v) <= cap, "cap {cap}: {} bytes", size(&v));
        } else {
            assert_eq!(
                v,
                json!({"health": "HEALTH_WARN", "osds": {"total": 12, "up": 11, "in": 12},
                "placement_groups": {"total": 32}, "usage": {"bytes_used": 1, "bytes_total": 4, "percent_used": 25.0},
                "truncated": true}),
                "cap {cap}"
            );
            assert!(size(&v) <= 327);
        }
    }
}

#[test]
fn fallback_through_the_real_pipeline_with_the_longest_numbers() {
    // Every total at the longest JSON number (24 bytes) and heavy strings
    // everywhere: a 200-byte cap is below the fixed part, so the lists stay
    // empty, source and health are shortened, and the fallback is returned.
    let n = -f64::MIN_POSITIVE;
    let status = json!({"health": {"status": format!("HEALTH_ERR{}", "🔥".repeat(100))},
        "quorum_names": ["🔥".repeat(100)],
        "osdmap": {"num_osds": n, "num_up_osds": n, "num_in_osds": n},
        "pgmap": {"num_pgs": n, "pgs_by_state": [{"state_name": "\u{0001}".repeat(100), "count": 1}],
                  "bytes_used": n, "bytes_total": n}});
    let v = summarize_ceph_status_within(&status, Some(&"\"".repeat(500)), 200);
    assert_eq!(v["truncated"], true);
    assert_eq!(v["health"], "HEALTH_ERR…");
    assert_eq!(v["osds"], json!({"total": n, "up": n, "in": n}));
    assert_eq!(v["placement_groups"], json!({"total": n}));
    assert_eq!(v["usage"]["bytes_used"], n);
    assert!(
        v["usage"].get("percent_used").is_none(),
        "a negative total gives no percent"
    );
    assert!(size(&v) <= 327, "{} bytes", size(&v));
    // The same input at the real cap needs no fallback.
    let full =
        summarize_ceph_status_within(&status, Some(&"\"".repeat(500)), CEPH_HEALTH_MAX_BYTES);
    assert!(full.get("truncated").is_none());
    assert!(size(&full) <= CEPH_HEALTH_MAX_BYTES);
}

// ---------------------------------------------------------------------------
// Sweeps: the cap holds and the counts add up for many inputs
// ---------------------------------------------------------------------------

fn lcg(seed: &mut u64) -> usize {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as usize
}

#[test]
fn cap_holds_and_counts_add_up_for_500_mixed_inputs() {
    let mut seed = 20261013u64;
    for case in 0..500 {
        let n_checks = lcg(&mut seed) % 80;
        let n_quorum = lcg(&mut seed) % 60;
        let n_states = lcg(&mut seed) % 60;
        let msg_len = lcg(&mut seed) % 400;
        let name_len = lcg(&mut seed) % 300;
        let src_len = lcg(&mut seed) % 3000;
        let health_len = lcg(&mut seed) % 2000;

        let names: Vec<String> = (0..n_quorum)
            .map(|i| format!("{i:03}{}", "q".repeat(name_len)))
            .collect();
        let s = json!({
            "health": {"status": "H".repeat(health_len), "checks": checks_obj(n_checks, "HEALTH_WARN", msg_len)},
            "quorum_names": names,
            "osdmap": {"num_osds": n_checks, "num_up_osds": n_quorum, "num_in_osds": n_states},
            "pgmap": {"num_pgs": n_states, "pgs_by_state": pg_states(n_states, name_len),
                      "bytes_used": 1, "bytes_total": 3}
        });
        let src = "p".repeat(src_len);
        let v = summarize_ceph_status_from(&s, Some(&src));

        assert!(
            size(&v) <= CEPH_HEALTH_MAX_BYTES,
            "case {case}: {} bytes",
            size(&v)
        );
        let kept_checks = v["checks"].as_array().unwrap().len() as u64;
        assert_eq!(
            kept_checks + v["checks_omitted"].as_u64().unwrap(),
            n_checks as u64,
            "case {case}: checks"
        );
        let kept_quorum = v["monitors"]["quorum"].as_array().unwrap().len() as u64;
        assert_eq!(
            kept_quorum + v["monitors"]["quorum_omitted"].as_u64().unwrap(),
            n_quorum as u64,
            "case {case}: quorum"
        );
        let kept_states = v["placement_groups"]["by_state"].as_object().unwrap().len() as u64;
        assert_eq!(
            kept_states + v["placement_groups"]["by_state_omitted"].as_u64().unwrap(),
            n_states as u64,
            "case {case}: states"
        );
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
    fn string(&mut self, max_len: usize) -> String {
        const ALPHABET: [char; 16] = [
            'a',
            'Z',
            '_',
            '+',
            ' ',
            '"',
            '\\',
            '\n',
            '\t',
            '\u{0001}',
            '\u{001f}',
            '\u{7f}',
            'é',
            '中',
            '🔥',
            '\u{10FFFF}',
        ];
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| ALPHABET[self.below(ALPHABET.len())])
            .collect()
    }
    fn number(&mut self) -> Value {
        match self.below(14) {
            0 => json!(0),
            1 => json!(12),
            2 => json!(-1),
            3 => json!(u64::MAX),
            4 => json!(i64::MIN),
            5 => json!(f64::MAX),
            6 => json!(-f64::MAX),
            7 => json!(f64::MIN_POSITIVE),
            8 => json!(-f64::MIN_POSITIVE),
            9 => json!(0.1),
            10 => json!(1e-300),
            11 => json!("12"),
            12 => Value::Null,
            _ => json!({"x": 1}),
        }
    }
}

#[test]
fn sweep_three_thousand_random_inputs_never_exceed_the_cap() {
    let mut rng = Rng(20261013);
    for case in 0..3000 {
        let n_checks = rng.below(31);
        let n_quorum = rng.below(31);
        let n_states = rng.below(31);
        let mut checks = serde_json::Map::new();
        for i in 0..n_checks {
            // Distinct keys: a JSON object cannot hold two checks with one name.
            checks.insert(
                format!("{i}{}", rng.string(300)),
                json!({"severity": rng.string(300), "summary": {"message": rng.string(400)}}),
            );
        }
        let quorum: Vec<String> = (0..n_quorum).map(|_| rng.string(300)).collect();
        let states: Vec<Value> = (0..n_states)
            .map(|_| json!({"state_name": rng.string(300), "count": rng.number()}))
            .collect();
        let totals = [
            rng.number(),
            rng.number(),
            rng.number(),
            rng.number(),
            rng.number(),
            rng.number(),
        ];
        let health = rng.string(400);
        let osdmap =
            json!({"num_osds": totals[0], "num_up_osds": totals[1], "num_in_osds": totals[2]});
        let status = json!({
            "health": {"status": health, "checks": checks},
            "quorum_names": quorum,
            "osdmap": if case % 2 == 0 { osdmap } else { json!({"osdmap": osdmap}) },
            "pgmap": {"num_pgs": totals[3], "pgs_by_state": states,
                      "bytes_used": totals[4], "bytes_total": totals[5]},
            "mgrmap": {"filler": "x".repeat(rng.below(5000))}
        });
        let source = if case % 3 == 0 {
            None
        } else {
            Some(rng.string(600))
        };
        let v = summarize_ceph_status_from(&status, source.as_deref());

        let len = size(&v);
        assert!(
            len <= CEPH_HEALTH_MAX_BYTES,
            "case {case}: {len} bytes: {v}"
        );
        assert!(
            v.get("truncated").is_none(),
            "case {case}: fallback must not be needed"
        );
        let h = v["health"].as_str().unwrap();
        assert!(json_bytes(h) <= NAME_MAX, "case {case}: health");
        if let Some(src) = &source {
            let s = v["source"].as_str().unwrap();
            assert!(json_bytes(s) <= SOURCE_MAX, "case {case}: source");
            assert!(
                src.starts_with(s.trim_end_matches('…')),
                "case {case}: source prefix"
            );
        }
        let kept_checks = v["checks"].as_array().unwrap();
        for ch in kept_checks {
            assert!(
                json_bytes(ch["name"].as_str().unwrap()) <= NAME_MAX,
                "case {case}"
            );
            assert!(
                json_bytes(ch["severity"].as_str().unwrap()) <= NAME_MAX,
                "case {case}"
            );
            assert!(
                json_bytes(ch["summary"].as_str().unwrap()) <= MESSAGE_MAX,
                "case {case}"
            );
        }
        assert_eq!(
            kept_checks.len() + v["checks_omitted"].as_u64().unwrap() as usize,
            n_checks,
            "case {case}: checks"
        );
        let kept_quorum = v["monitors"]["quorum"].as_array().unwrap();
        for name in kept_quorum {
            assert!(
                json_bytes(name.as_str().unwrap()) <= NAME_MAX,
                "case {case}"
            );
        }
        assert_eq!(
            kept_quorum.len() + v["monitors"]["quorum_omitted"].as_u64().unwrap() as usize,
            n_quorum,
            "case {case}: quorum"
        );
        assert_eq!(v["monitors"]["count"], n_quorum, "case {case}");
        let kept_states = v["placement_groups"]["by_state"].as_object().unwrap();
        for key in kept_states.keys() {
            assert!(json_bytes(key) <= NAME_MAX, "case {case}");
        }
        assert_eq!(
            kept_states.len()
                + v["placement_groups"]["by_state_omitted"].as_u64().unwrap() as usize,
            n_states,
            "case {case}: PG states"
        );
        for (at, want) in [
            ("/osds/total", &totals[0]),
            ("/osds/up", &totals[1]),
            ("/osds/in", &totals[2]),
            ("/placement_groups/total", &totals[3]),
            ("/usage/bytes_used", &totals[4]),
            ("/usage/bytes_total", &totals[5]),
        ] {
            let got = v.pointer(at);
            if want.is_number() {
                assert_eq!(got, Some(want), "case {case}: {at}");
            } else {
                assert!(got.is_none(), "case {case}: {at} must be left out");
            }
        }
    }
}
