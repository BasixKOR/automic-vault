use base64::Engine;
use serde_json::json;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

struct Helper {
    child: Child,
    input: ChildStdin,
    _output: ChildStdout,
    port: u16,
    credential: String,
}

impl Helper {
    fn start() -> Self {
        let credential = "credential_0123456789012345678901".to_string();
        let mut child = Command::new(env!("CARGO_BIN_EXE_av-proxy-helper"))
            .env_clear()
            .env("AV_PROXY_CONTROL", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = child.stdout.take().unwrap();
        write_frame(
            &mut input,
            &json!({
                "type": "bootstrap",
                "session_id": "session_0123456789",
                "proxy_credential": credential,
                "target": {
                    "pid": std::process::id(),
                    "pid_version": 1,
                    "start_usec": 1
                },
                "references": {
                    "API_TOKEN": "avref_01234567890123456789012345"
                }
            }),
        );
        let ready = read_frame(&mut output);
        assert_eq!(ready["type"], "ready");
        let port = ready["port"].as_u64().unwrap() as u16;
        Self {
            child,
            input,
            _output: output,
            port,
            credential,
        }
    }

    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
    }

    fn authorization(&self) -> String {
        let value =
            base64::engine::general_purpose::STANDARD.encode(format!("av:{}", self.credential));
        format!("Proxy-Authorization: Basic {value}\r\n")
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        write_frame(&mut self.input, &json!({ "type": "shutdown" }));
        let _ = self.child.wait();
    }
}

#[test]
fn proxy_requires_auth_rejects_private_and_never_falls_back_to_a_raw_tunnel() {
    let helper = Helper::start();

    let mut unauthenticated = helper.connect();
    unauthenticated
        .write_all(b"GET http://api.github.com/ HTTP/1.1\r\nHost: api.github.com\r\n\r\n")
        .unwrap();
    unauthenticated.flush().unwrap();
    assert!(read_headers(&mut unauthenticated).starts_with("HTTP/1.1 407"));

    let mut private = helper.connect();
    write!(
        private,
        "CONNECT 127.0.0.1:443 HTTP/1.1\r\nHost: 127.0.0.1:443\r\n{}\r\n",
        helper.authorization()
    )
    .unwrap();
    private.flush().unwrap();
    assert!(read_headers(&mut private).starts_with("HTTP/1.1 403"));

    let mut raw = helper.connect();
    write!(
        raw,
        "CONNECT api.github.com:443 HTTP/1.1\r\nHost: api.github.com:443\r\n{}\r\n",
        helper.authorization()
    )
    .unwrap();
    raw.flush().unwrap();
    let connected = read_headers(&mut raw);
    assert!(connected.starts_with("HTTP/1.1 200"), "{connected:?}");
    raw.write_all(b"GET / HTTP/1.1\r\nHost: api.github.com\r\n\r\n")
        .unwrap();
    raw.shutdown(Shutdown::Write).unwrap();
    let mut byte = [0_u8; 1];
    match raw.read(&mut byte) {
        Ok(0) | Err(_) => {}
        Ok(_) => assert_ne!(byte[0], b'H', "proxy forwarded plaintext after CONNECT"),
    }
}

fn write_frame(writer: &mut impl Write, value: &serde_json::Value) {
    let payload = serde_json::to_vec(value).unwrap();
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .unwrap();
    writer.write_all(&payload).unwrap();
    writer.flush().unwrap();
}

fn read_frame(reader: &mut impl Read) -> serde_json::Value {
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length).unwrap();
    let mut payload = vec![0; u32::from_be_bytes(length) as usize];
    reader.read_exact(&mut payload).unwrap();
    serde_json::from_slice(&payload).unwrap()
}

fn read_headers(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 16 * 1024);
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).unwrap()
}
