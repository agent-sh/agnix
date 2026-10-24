//! Exercise the packaged server's actual JSON-RPC framing and SDK transport.

use serde_json::{Value, json};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::task::JoinHandle;
use tokio::time::timeout;

// Bound responses and EOF shutdown even when a transport regression hangs.
const IO_TIMEOUT: Duration = Duration::from_secs(30);
// rmcp 3.6's default counts the complete frame, including its newline.
const MAX_LINE_BYTES: usize = 16 * 1024 * 1024;

struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    output: Lines<BufReader<ChildStdout>>,
    stderr: JoinHandle<String>,
    _project: tempfile::TempDir,
}

impl Peer {
    async fn start() -> Self {
        // Allows the same integration cases to establish before/after proof
        // against a previously released binary, without changing the server.
        let binary = std::env::var_os("AGNIX_MCP_TEST_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_agnix-mcp").into());
        let project = tempfile::tempdir().unwrap();
        let mut child = Command::new(binary)
            .current_dir(project.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let output = BufReader::new(stdout).lines();
        let stderr = tokio::spawn(async move {
            let mut text = String::new();
            stderr.read_to_string(&mut text).await.unwrap();
            text
        });
        let mut peer = Self {
            child,
            input,
            output,
            stderr,
            _project: project,
        };
        peer.send(json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05", "capabilities": {},
                "clientInfo": {"name": "agnix-stdio-test", "version": "1"}
            }
        }))
        .await;
        let initialized = peer.next().await;
        assert_eq!(initialized["id"], 1);
        assert_eq!(initialized["result"]["serverInfo"]["name"], "agnix");
        peer.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .await;
        peer
    }

    async fn send(&mut self, value: Value) {
        self.send_raw(&serde_json::to_vec(&value).unwrap()).await;
    }

    async fn send_raw(&mut self, bytes: &[u8]) {
        let input = self.input.as_mut().unwrap();
        timeout(IO_TIMEOUT, async {
            input.write_all(bytes).await.unwrap();
            input.write_all(b"\n").await.unwrap();
            input.flush().await.unwrap();
        })
        .await
        .expect("server stopped reading stdin");
    }

    async fn next(&mut self) -> Value {
        let line = timeout(IO_TIMEOUT, self.output.next_line())
            .await
            .expect("server response timed out")
            .unwrap()
            .expect("server closed stdout before responding");
        serde_json::from_str(&line).unwrap()
    }

    async fn tools_result(&mut self, id: i64) {
        let response = self.next().await;
        assert_eq!(response["id"], id, "{response}");
        let tools = response["result"]["tools"].as_array().unwrap();
        for name in [
            "validate_file",
            "validate_project",
            "get_rules",
            "get_rule_docs",
        ] {
            assert!(tools.iter().any(|tool| tool["name"] == name), "{response}");
        }
    }

    async fn finish(mut self) -> String {
        // Closing the client pipe must terminate the initialized server.
        self.input.take();
        // Replies can arrive out of order. Drain through EOF while waiting so
        // a delayed reply to a rejected frame cannot escape the assertions.
        let (status, remaining) = timeout(IO_TIMEOUT, async {
            tokio::join!(self.child.wait(), async {
                let mut remaining = Vec::new();
                while let Some(line) = self.output.next_line().await.unwrap() {
                    remaining.push(serde_json::from_str::<Value>(&line).unwrap());
                }
                remaining
            })
        })
        .await
        .expect("server did not close its pipes after stdin EOF");
        assert!(status.unwrap().success());
        assert!(
            remaining.iter().all(|message| message.get("id").is_none()),
            "unexpected delayed responses: {remaining:?}"
        );
        timeout(IO_TIMEOUT, self.stderr).await.unwrap().unwrap()
    }
}

#[tokio::test]
async fn stdio_handshake_tools_and_integral_float_ids() {
    let mut peer = Peer::start().await;
    peer.send_raw(br#"{"jsonrpc":"2.0","id":2.0,"method":"tools/list"}"#)
        .await;
    peer.tools_result(2).await;
    peer.send_raw(br#"{"jsonrpc":"2.0","id":3e1,"method":"tools/list"}"#)
        .await;
    peer.tools_result(30).await;
    peer.send(json!({"jsonrpc":"2.0", "id":4, "method":"tools/call",
        "params":{"name":"get_rule_docs", "arguments":{"rule_id":"AS-004"}}}))
        .await;
    let response = peer.next().await;
    assert_eq!(response["id"], 4);
    assert_ne!(response["result"]["isError"], true);
    assert!(!response["result"]["content"].as_array().unwrap().is_empty());
    peer.finish().await;
}

#[tokio::test]
async fn stdio_rejects_conflicting_fields_and_recovers() {
    let mut peer = Peer::start().await;
    peer.send(json!({"jsonrpc":"2.0", "id":5, "method":"tools/list", "result":{}}))
        .await;
    let response = peer.next().await;
    assert_eq!(response["error"]["code"], -32600, "{response}");
    assert!(response["id"].is_null(), "{response}");
    peer.send(json!({"jsonrpc":"2.0", "id":6, "method":"tools/list"}))
        .await;
    peer.tools_result(6).await;
    peer.finish().await;
}

fn tools_frame(id: i64, line_bytes: usize) -> Vec<u8> {
    let mut value = json!({"jsonrpc":"2.0", "id":id, "method":"tools/list",
        "params":{"_meta":{"payload":""}}});
    let overhead = serde_json::to_vec(&value).unwrap().len() + 1;
    value["params"]["_meta"]["payload"] = Value::String("x".repeat(line_bytes - overhead));
    let frame = serde_json::to_vec(&value).unwrap();
    assert_eq!(frame.len() + 1, line_bytes);
    frame
}

#[tokio::test]
async fn stdio_bounds_lines_and_recovers_before_eof() {
    let mut peer = Peer::start().await;
    peer.send_raw(&tools_frame(7, MAX_LINE_BYTES)).await;
    peer.tools_result(7).await;
    peer.send_raw(&tools_frame(8, MAX_LINE_BYTES + 1)).await;
    peer.send(json!({"jsonrpc":"2.0", "id":9, "method":"tools/list"}))
        .await;
    peer.tools_result(9).await;
    let stderr = peer.finish().await;
    assert!(
        stderr.contains("Incoming message exceeded the maximum line length, discarding it"),
        "{stderr}"
    );
}
