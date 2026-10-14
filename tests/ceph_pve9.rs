// Ceph pool and OSD tools on Proxmox VE 9. Pools moved from
// /nodes/{node}/ceph/pools to /nodes/{node}/ceph/pool, and the OSD list is now
// a CRUSH tree ({flags, root}) instead of a flat list.
mod common;
use common::*;
use proxmox_mcp_rs::mcp::flatten_ceph_osds;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn text_of(res: &Value) -> String {
    res["content"][0]["text"].as_str().unwrap().to_string()
}

fn osd(id: u32, host: &str, status: &str, in_: u32) -> Value {
    json!({
        "id": id.to_string(), "name": format!("osd.{}", id), "host": host,
        "type": "osd", "leaf": 1, "status": status, "in": in_,
        "device_class": "nvme", "crush_weight": 0.2, "percent_used": 50.5,
        "pgs": 64, "ceph_version": "ceph version 19.2.6", "blfsdev": "/dev/dm-0"
    })
}

// ---------------------------------------------------------------------------
// Through the MCP entry point (wiremock)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_ceph_pools_uses_pool_path() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/pools"))
        .respond_with(ResponseTemplate::new(501))
        .expect(0)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/pool"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [
            {"pool": 1, "pool_name": ".mgr", "size": 2, "min_size": 2},
            {"pool": 2, "pool_name": "rbd", "size": 2, "min_size": 2}
        ]})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("list_ceph_pools", &json!({"node": "pve1"}))
        .await
        .unwrap();
    let pools: Value = serde_json::from_str(&text_of(&res)).unwrap();
    assert_eq!(pools.as_array().unwrap().len(), 2);
    assert_eq!(pools[1]["pool_name"], "rbd");
    assert_eq!(pools[1]["min_size"], 2);
}

#[tokio::test]
async fn test_ceph_pool_write_paths() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("POST"))
        .and(path("/api2/json/nodes/pve1/ceph/pool"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:create"})))
        .expect(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api2/json/nodes/pve1/ceph/pool/test-pool"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "UPID:delete"})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool(
            "manage_cluster_config",
            &json!({"type": "ceph", "action": "create", "node": "pve1", "name": "test-pool"}),
        )
        .await
        .unwrap();
    assert!(text_of(&res).contains("UPID:create"));
    let res = server
        .call_tool(
            "manage_cluster_config",
            &json!({"type": "ceph", "action": "delete", "node": "pve1", "name": "test-pool"}),
        )
        .await
        .unwrap();
    assert!(text_of(&res).contains("UPID:delete"));
}

#[tokio::test]
async fn test_list_ceph_osds_flattens_tree() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    let tree = json!({
        "flags": "sortbitwise,recovery_deletes",
        "root": {
            "id": "-1", "name": "default", "type": "root", "leaf": 0,
            "children": [
                {"id": "-2", "name": "hostA", "type": "host", "leaf": 0,
                 "children": [osd(0, "hostA", "up", 1), osd(1, "hostA", "down", 1)]},
                {"id": "-3", "name": "hostB", "type": "host", "leaf": 0,
                 "children": [osd(2, "hostB", "up", 0), osd(3, "hostB", "up", 1)]}
            ]
        }
    });
    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/osd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": tree})))
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("list_ceph_osds", &json!({"node": "pve1"}))
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&text_of(&res)).unwrap();
    assert_eq!(v["flags"], "sortbitwise,recovery_deletes");
    let osds = v["osds"].as_array().unwrap();
    assert_eq!(osds.len(), 4);
    assert_eq!(osds[1]["name"], "osd.1");
    assert_eq!(osds[1]["status"], "down");
    assert_eq!(osds[2]["host"], "hostB");
    assert_eq!(osds[2]["in"], 0);
    // Only the compact fields are kept.
    assert!(osds[0].get("ceph_version").is_none());
    assert_eq!(osds[0]["pgs"], 64);
}

#[tokio::test]
async fn test_list_ceph_osds_accepts_list() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/osd"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": 0, "name": "osd.0"}]})),
        )
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("list_ceph_osds", &json!({"node": "pve1"}))
        .await
        .unwrap();
    let v: Value = serde_json::from_str(&text_of(&res)).unwrap();
    assert_eq!(v["osds"].as_array().unwrap().len(), 1);
    assert_eq!(v["osds"][0]["name"], "osd.0");
}

#[tokio::test]
async fn list_ceph_osds_bad_shape_is_an_error_not_a_panic() {
    let mock_server = MockServer::start().await;
    let server = setup_mcp_server(&mock_server).await;

    Mock::given(method("GET"))
        .and(path("/api2/json/nodes/pve1/ceph/osd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {"flags": "x"}})))
        .mount(&mock_server)
        .await;

    let res = server
        .call_tool("list_ceph_osds", &json!({"node": "pve1"}))
        .await;
    assert!(res.is_err(), "bad OSD shape must be an error");
}

// ---------------------------------------------------------------------------
// flatten_ceph_osds
// ---------------------------------------------------------------------------

#[test]
fn flatten_empty_tree_gives_no_osds_and_keeps_flags() {
    let v = flatten_ceph_osds(&json!({
        "flags": "sortbitwise",
        "root": {"id": "-1", "name": "default", "type": "root", "children": []}
    }))
    .unwrap();
    assert_eq!(v["flags"], "sortbitwise");
    assert_eq!(v["osds"], json!([]));
}

#[test]
fn flatten_tree_without_flags_gives_null_flags() {
    let v = flatten_ceph_osds(&json!({"root": {"type": "root", "children": []}})).unwrap();
    assert!(v["flags"].is_null());
    assert_eq!(v["osds"], json!([]));
}

#[test]
fn flatten_walks_three_levels_deep() {
    // root -> row -> host -> osd: the walk must not stop at the first level.
    let v = flatten_ceph_osds(&json!({
        "flags": "",
        "root": {"type": "root", "children": [
            {"type": "row", "children": [
                {"type": "host", "name": "hostC", "children": [
                    {"type": "osd", "id": "7", "name": "osd.7", "host": "hostC",
                     "status": "up", "in": 1}
                ]}
            ]}
        ]}
    }))
    .unwrap();
    let osds = v["osds"].as_array().unwrap();
    assert_eq!(osds.len(), 1);
    assert_eq!(osds[0]["name"], "osd.7");
    assert_eq!(osds[0]["host"], "hostC");
}

#[test]
fn flatten_skips_host_and_root_nodes_as_osds() {
    // Only type == "osd" nodes may appear in the output.
    let v = flatten_ceph_osds(&json!({
        "root": {"type": "root", "name": "default", "children": [
            {"type": "host", "name": "hostA", "children": []}
        ]}
    }))
    .unwrap();
    assert_eq!(v["osds"], json!([]));
}

#[test]
fn flatten_osd_with_missing_fields_gives_nulls_without_panic() {
    let v = flatten_ceph_osds(&json!({
        "root": {"type": "root", "children": [
            {"type": "osd"}
        ]}
    }))
    .unwrap();
    let osd = &v["osds"][0];
    for key in [
        "id",
        "name",
        "host",
        "status",
        "in",
        "device_class",
        "crush_weight",
        "percent_used",
        "pgs",
    ] {
        assert!(osd[key].is_null(), "{} should be null", key);
    }
}

#[test]
fn flatten_keeps_osd_id_as_sent_string() {
    // PVE 9 sends the id as a string ("0"); it must not be converted.
    let v = flatten_ceph_osds(&json!({
        "root": {"type": "root", "children": [
            {"type": "osd", "id": "12", "name": "osd.12"}
        ]}
    }))
    .unwrap();
    assert_eq!(v["osds"][0]["id"], "12");
}

#[test]
fn flatten_empty_plain_list_gives_no_osds() {
    let v = flatten_ceph_osds(&json!([])).unwrap();
    assert_eq!(v["osds"], json!([]));
    assert!(v["flags"].is_null());
}

#[test]
fn flatten_rejects_object_without_root_and_names_the_shape() {
    let err = flatten_ceph_osds(&json!({"flags": "x"}))
        .unwrap_err()
        .to_string();
    assert!(err.contains("Unexpected Ceph OSD answer"), "{}", err);
    assert!(err.contains("without \"root\""), "{}", err);
}

#[test]
fn flatten_rejects_null_string_number_and_bool() {
    let cases = [
        (json!(null), "null"),
        (json!("osd"), "a string"),
        (json!(3), "a number"),
        (json!(true), "a boolean"),
    ];
    for (input, named) in cases {
        let err = flatten_ceph_osds(&input).unwrap_err().to_string();
        assert!(err.contains(named), "for {} got: {}", input, err);
    }
}
