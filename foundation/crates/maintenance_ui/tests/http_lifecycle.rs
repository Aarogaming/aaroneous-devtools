//#![deny(unsafe_code)]
//! Socket regression and HTTP server lifecycle integration tests for maintenance_ui utility.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    sync::Arc,
    thread,
    time::Duration,
};

use maintenance_ui::http;
use tiny_http::Server;

#[test]
fn test_expect_100_continue_header_is_rejected_immediately_without_blocking() {
    let server = Server::http((Ipv4Addr::LOCALHOST, 0)).expect("start server");
    let port = server.server_addr().to_ip().expect("addr").port();

    let server = Arc::new(server);
    let server_clone = server.clone();

    // Spawn server loop in a background thread
    let handle = thread::spawn(move || {
        let mut count = 0;
        while count < 2 {
            if let Ok(Some(request)) = server_clone.recv_timeout(Duration::from_millis(500)) {
                if http::has_expect_header(&request) {
                    http::reject_expect_header(request).expect("reject expect");
                } else {
                    http::respond(
                        request,
                        200,
                        "application/json",
                        r#"{"status":"ok"}"#.into(),
                    )
                    .expect("respond");
                }
                count += 1;
            }
        }
    });

    // Client 1: Sends Expect: 100-continue and stalls (no body sent)
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut stream1 = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).unwrap();
    stream1
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    let raw_expect_request = format!(
        "POST /api/packet HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Expect: 100-continue\r\n\
         Connection: close\r\n\
         Content-Type: application/json\r\n\
         Content-Length: 100\r\n\
         \r\n"
    );
    stream1.write_all(raw_expect_request.as_bytes()).unwrap();

    // Read response from stream1: must receive 417 Expectation Failed immediately without waiting for body
    let mut response1 = String::new();
    let mut buf = [0u8; 1024];
    let n = stream1.read(&mut buf).unwrap();
    response1.push_str(&String::from_utf8_lossy(&buf[..n]));

    assert!(
        response1.contains("417") || response1.contains("Expectation Failed"),
        "Expected 417 response, got: {response1}"
    );
    drop(stream1);

    // Client 2: Second concurrent request must succeed immediately without being blocked
    let mut stream2 = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).unwrap();
    stream2
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    let raw_normal_request = format!(
        "GET /api/status HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         \r\n"
    );
    stream2.write_all(raw_normal_request.as_bytes()).unwrap();

    let mut response2 = String::new();
    let n2 = stream2.read(&mut buf).unwrap();
    response2.push_str(&String::from_utf8_lossy(&buf[..n2]));

    assert!(
        response2.contains("200 OK"),
        "Expected 200 response, got: {response2}"
    );

    handle.join().unwrap();
}

#[test]
fn test_client_disconnect_does_not_crash_server_loop() {
    let server = Server::http((Ipv4Addr::LOCALHOST, 0)).expect("start server");
    let port = server.server_addr().to_ip().expect("addr").port();
    let server = Arc::new(server);
    let server_clone = server.clone();

    let handle = thread::spawn(move || {
        let mut count = 0;
        while count < 2 {
            if let Ok(Some(request)) = server_clone.recv_timeout(Duration::from_millis(500)) {
                let _ = http::respond(request, 200, "text/plain", "data".into());
                count += 1;
            }
        }
    });

    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    // Connection 1: Client connects and immediately closes (aborted client)
    {
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).unwrap();
        stream
            .write_all(format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes())
            .unwrap();
        // Drop stream immediately to simulate connection reset / abrupt disconnect
        drop(stream);
    }

    // Connection 2: Subsequent request succeeds cleanly
    let mut stream2 = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).unwrap();
    stream2
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream2
        .write_all(format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes())
        .unwrap();

    let mut buf = [0u8; 512];
    let n = stream2.read(&mut buf).unwrap();
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("200 OK"));

    handle.join().unwrap();
}

#[test]
fn test_http_admission_authorization_and_host_validation() {
    let server = Server::http((Ipv4Addr::LOCALHOST, 0)).expect("start server");
    let port = server.server_addr().to_ip().expect("addr").port();
    let host = format!("127.0.0.1:{port}");
    let origin = format!("http://{host}");
    let token = "test-secret-token-123456";

    let server = Arc::new(server);
    let server_clone = server.clone();
    let token_clone = token.to_string();
    let host_clone = host.clone();
    let origin_clone = origin.clone();

    let handle = thread::spawn(move || {
        let mut count = 0;
        while count < 3 {
            if let Ok(Some(request)) = server_clone.recv_timeout(Duration::from_millis(500)) {
                if !http::authorize(&request, &host_clone, &origin_clone, &token_clone) {
                    http::respond(
                        request,
                        403,
                        "application/json",
                        r#"{"error":"Forbidden"}"#.into(),
                    )
                    .expect("respond");
                } else {
                    http::respond(
                        request,
                        200,
                        "application/json",
                        r#"{"status":"Authorized"}"#.into(),
                    )
                    .expect("respond");
                }
                count += 1;
            }
        }
    });

    let addr: SocketAddr = host.parse().unwrap();

    // 1. Invalid Token -> 403
    let mut s1 = TcpStream::connect(addr).unwrap();
    let req1 = format!(
        "GET /api/status HTTP/1.1\r\n\
         Host: {host}\r\n\
         Origin: {origin}\r\n\
         X-Devtools-Token: invalid-token\r\n\
         \r\n"
    );
    s1.write_all(req1.as_bytes()).unwrap();
    let mut buf = [0u8; 512];
    let n1 = s1.read(&mut buf).unwrap();
    assert!(String::from_utf8_lossy(&buf[..n1]).contains("403 Forbidden"));

    // 2. Invalid Host -> 403
    let mut s2 = TcpStream::connect(addr).unwrap();
    let req2 = format!(
        "GET /api/status HTTP/1.1\r\n\
         Host: evil.com\r\n\
         Origin: {origin}\r\n\
         X-Devtools-Token: {token}\r\n\
         \r\n"
    );
    s2.write_all(req2.as_bytes()).unwrap();
    let n2 = s2.read(&mut buf).unwrap();
    assert!(String::from_utf8_lossy(&buf[..n2]).contains("403 Forbidden"));

    // 3. Valid Token + Exact Host + Matching Origin -> 200
    let mut s3 = TcpStream::connect(addr).unwrap();
    let req3 = format!(
        "GET /api/status HTTP/1.1\r\n\
         Host: {host}\r\n\
         Origin: {origin}\r\n\
         X-Devtools-Token: {token}\r\n\
         \r\n"
    );
    s3.write_all(req3.as_bytes()).unwrap();
    let n3 = s3.read(&mut buf).unwrap();
    assert!(String::from_utf8_lossy(&buf[..n3]).contains("200 OK"));

    handle.join().unwrap();
}

#[test]
fn test_constant_time_token_comparison() {
    assert!(http::constant_time_eq(
        b"abcdef1234567890",
        b"abcdef1234567890"
    ));
    assert!(!http::constant_time_eq(
        b"abcdef1234567890",
        b"abcdef1234567891"
    ));
    assert!(!http::constant_time_eq(b"short", b"longer_string"));
    assert!(!http::constant_time_eq(b"", b"token"));
}
