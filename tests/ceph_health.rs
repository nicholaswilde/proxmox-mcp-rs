// get_ceph_health: a compact summary of the Ceph status. The handler tests
// run through the MCP entry point with wiremock; the summary tests call
// summarize_ceph_status directly. The size cap has its own file,
// ceph_health_cap.rs.
mod common;
use common::*;
use proxmox_mcp_rs::mcp::{summarize_ceph_status, JsonRpcRequest};
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn text_of(res: &Value) -> String {
    res["content"][0]["text"].as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// Through the MCP entry point (wiremock)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_ceph_health_cluster() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let status = json!({
        "health": {
            "status": "HEALTH_WARN",
            "checks": {
                "OSD_NEARFULL": {"severity": "HEALTH_WARN", "muted": false,
                    "summary": {"count": 1, "message": "1 nearfull osd(s)"},
                    "detail": [{"message": "osd.3 is near full"}]},
                "POOL_NO_REDUNDANCY": {"severity": "HEALTH_WARN", "muted": false,
                    "summary": {"count": 1, "message": "1 pool(s) have no replicas configured"}}
            }
        },
        "quorum_names": ["pve1", "pve2", "pve3"],
        "osdmap": {"num_osds": 4, "num_up_osds": 3, "num_in_osds": 4},
        "pgmap": {
            "num_pgs": 129,
            "pgs_by_state": [
                {"state_name": "active+clean", "count": 128},
                {"state_name": "active+clean+scrubbing", "count": 1}
            ],
            "bytes_used": 250, "bytes_total": 1000
        },
        "mgrmap": {"filler": "x".repeat(100_000)}
    });
    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/ceph/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": status})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("get_ceph_health", &json!({}))
        .await
        .unwrap();
    let text = text_of(&res);
    assert!(text.len() < 4000, "summary is {} bytes", text.len());
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["health"], "HEALTH_WARN");
    assert_eq!(v["checks"].as_array().unwrap().len(), 2);
    assert_eq!(v["checks"][0]["name"], "OSD_NEARFULL");
    assert_eq!(v["checks"][0]["severity"], "HEALTH_WARN");
    assert_eq!(v["checks"][0]["summary"], "1 nearfull osd(s)");
    assert_eq!(v["monitors"]["count"], 3);
    assert_eq!(v["monitors"]["quorum"][2], "pve3");
    assert_eq!(v["osds"]["total"], 4);
    assert_eq!(v["osds"]["up"], 3);
    assert_eq!(v["osds"]["in"], 4);
    assert_eq!(v["placement_groups"]["total"], 129);
    assert_eq!(v["placement_groups"]["by_state"]["active+clean"], 128);
    assert_eq!(v["usage"]["percent_used"], 25.0);
    assert_eq!(v["source"], "/cluster/ceph/status");
}

#[tokio::test]
async fn test_get_ceph_health_node() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/status"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"health": {"status": "HEALTH_OK"}}})),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("get_ceph_health", &json!({"node": "pve1"}))
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&text_of(&res)).unwrap();
    assert_eq!(v["health"], "HEALTH_OK");
    assert_eq!(v["checks"].as_array().unwrap().len(), 0);
    assert!(v["osds"]["total"].is_null());
    assert!(v["monitors"]["count"].is_null());
    assert!(v["placement_groups"]["total"].is_null());
    assert!(v["usage"]["percent_used"].is_null());
    assert_eq!(v["source"], "/nodes/pve1/ceph/status");
}

#[tokio::test]
async fn get_ceph_health_with_a_node_asks_only_that_node() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/status"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data": {"health": {"status": "HEALTH_ERR"}}})),
        )
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/ceph/status"))
        .respond_with(ResponseTemplate::new(501))
        .expect(0)
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("get_ceph_health", &json!({"node": "pve1"}))
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&text_of(&res)).unwrap();
    assert_eq!(v["health"], "HEALTH_ERR");
}

#[tokio::test]
async fn test_get_ceph_health_is_listed() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let req: JsonRpcRequest = serde_json::from_value(json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list"
    }))
    .unwrap();
    let res = server.handle_request(req).await.unwrap();
    let tools = res["tools"].as_array().unwrap();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "get_ceph_health")
        .expect("get_ceph_health is advertised");
    assert_eq!(tool["inputSchema"]["required"], json!([]));
    assert!(tool["inputSchema"]["properties"]["node"].is_object());
}

#[tokio::test]
async fn get_ceph_health_upstream_error_is_propagated() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/cluster/ceph/status"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    let res = server.call_tool("get_ceph_health", &json!({})).await;
    assert!(
        res.is_err(),
        "a 500 from Proxmox must not look like HEALTH_OK"
    );
}

// ---------------------------------------------------------------------------
// summarize_ceph_status
// ---------------------------------------------------------------------------

#[test]
fn summarize_empty_object_gives_nulls_and_empty_checks() {
    let v = summarize_ceph_status(&json!({}));
    assert!(v["health"].is_null());
    assert_eq!(v["checks"], json!([]));
    assert_eq!(v["monitors"]["count"], Value::Null);
    assert!(v["monitors"]["quorum"].is_null());
    assert!(v["osds"]["total"].is_null());
    assert!(v["placement_groups"]["by_state"].is_null());
    assert!(v["usage"]["percent_used"].is_null());
}

#[test]
fn summarize_health_error_keeps_every_check_and_severity() {
    let checks: serde_json::Map<String, Value> = (0..12)
        .map(|i| {
            let sev = if i % 2 == 0 {
                "HEALTH_ERR"
            } else {
                "HEALTH_WARN"
            };
            (
                format!("CHECK_{:02}", i),
                json!({"severity": sev, "summary": {"message": format!("problem {}", i)}}),
            )
        })
        .collect();
    let v = summarize_ceph_status(&json!({"health": {"status": "HEALTH_ERR", "checks": checks}}));
    assert_eq!(v["health"], "HEALTH_ERR");
    let out = v["checks"].as_array().unwrap();
    assert_eq!(out.len(), 12);
    let errs = out.iter().filter(|c| c["severity"] == "HEALTH_ERR").count();
    assert_eq!(errs, 6);
}

#[test]
fn summarize_check_without_summary_message_gives_null_summary() {
    let v = summarize_ceph_status(&json!({"health": {"status": "HEALTH_WARN", "checks": {
        "BARE": {"severity": "HEALTH_WARN"}
    }}}));
    assert_eq!(v["checks"][0]["name"], "BARE");
    assert_eq!(v["checks"][0]["severity"], "HEALTH_WARN");
    assert!(v["checks"][0]["summary"].is_null());
}

#[test]
fn summarize_checks_that_are_not_an_object_give_empty_list() {
    let v = summarize_ceph_status(&json!({"health": {"status": "HEALTH_WARN", "checks": []}}));
    assert_eq!(v["checks"], json!([]));
}

#[test]
fn summarize_percent_is_null_when_total_is_zero() {
    let v = summarize_ceph_status(&json!({"pgmap": {"bytes_used": 0, "bytes_total": 0}}));
    assert!(v["usage"]["percent_used"].is_null());
}

#[test]
fn summarize_percent_is_null_when_bytes_missing() {
    let v = summarize_ceph_status(&json!({"pgmap": {"bytes_total": 1000}}));
    assert!(v["usage"]["percent_used"].is_null());
    assert_eq!(v["usage"]["bytes_total"], 1000);
}

#[test]
fn summarize_percent_rounds_to_one_decimal() {
    // 1 / 3 = 33.33...% must come out as 33.3
    let v = summarize_ceph_status(&json!({"pgmap": {"bytes_used": 1, "bytes_total": 3}}));
    assert_eq!(v["usage"]["percent_used"], 33.3);
}

#[test]
fn summarize_reads_nested_osdmap_from_older_ceph() {
    let v = summarize_ceph_status(&json!({"osdmap": {"osdmap": {
        "num_osds": 6, "num_up_osds": 5, "num_in_osds": 6
    }}}));
    assert_eq!(v["osds"]["total"], 6);
    assert_eq!(v["osds"]["up"], 5);
    assert_eq!(v["osds"]["in"], 6);
}

#[test]
fn summarize_skips_pg_state_entries_without_a_name() {
    let v = summarize_ceph_status(&json!({"pgmap": {"pgs_by_state": [
        {"count": 5},
        {"state_name": "active+clean", "count": 10}
    ]}}));
    let by_state = v["placement_groups"]["by_state"].as_object().unwrap();
    assert_eq!(by_state.len(), 1);
    assert_eq!(by_state["active+clean"], 10);
}

#[test]
fn summarize_pgs_by_state_that_is_not_a_list_gives_null() {
    let v = summarize_ceph_status(&json!({"pgmap": {"pgs_by_state": "oops"}}));
    assert!(v["placement_groups"]["by_state"].is_null());
}

#[test]
fn summarize_empty_quorum_gives_zero_monitors() {
    let v = summarize_ceph_status(&json!({"quorum_names": []}));
    assert_eq!(v["monitors"]["count"], 0);
    assert_eq!(v["monitors"]["quorum"], json!([]));
}

#[test]
fn summarize_huge_mgrmap_is_dropped() {
    // The raw answer is about 120 KB on a real cluster; only the summary is returned.
    let big = json!({
        "health": {"status": "HEALTH_OK"},
        "mgrmap": {"modules": vec!["x".repeat(1000); 100]}
    });
    let text = serde_json::to_string(&summarize_ceph_status(&big)).unwrap();
    assert!(text.len() < 1024, "summary is {} bytes", text.len());
}
