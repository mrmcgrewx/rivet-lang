//! Regressions for bounded HTTP parsing and stopping with incomplete clients.
use rivet::{language::Route, runtime::ProgramServer, workbench::Workbench};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
}
#[test]
fn workbench_stops_with_incomplete_post_body() {
    let app = Workbench::bind(0).unwrap();
    let address = app.address();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        app.serve(signal).unwrap();
        sender.send(()).unwrap();
    });
    let mut client = connect(address);
    write!(client,"POST /api/build HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nX-Rivet-Request: 1\r\nContent-Length: 2048\r\n\r\n{{").unwrap();
    thread::sleep(Duration::from_millis(50));
    stop.store(true, Ordering::Release);
    let stopped = receiver.recv_timeout(Duration::from_millis(500));
    drop(client);
    worker.join().unwrap();
    assert!(
        stopped.is_ok(),
        "incomplete body prevented bounded shutdown"
    );
    let rebound = Workbench::bind(address.port()).unwrap();
    assert_eq!(rebound.address().port(), address.port());
}
#[test]
fn program_stops_with_incomplete_get_body() {
    let mut server = ProgramServer::start(
        0,
        vec![Route {
            method: "GET".into(),
            path: "/hello".into(),
            status: 200,
            body: json!({"message":"hello world"}),
        }],
    )
    .unwrap();
    let address: SocketAddr = server
        .info(0)
        .address
        .trim_start_matches("http://")
        .parse()
        .unwrap();
    let mut client = connect(address);
    write!(
        client,
        "GET /hello HTTP/1.1\r\nHost: {address}\r\nContent-Length: 2048\r\n\r\nx"
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        server.stop().unwrap();
        sender.send(()).unwrap();
    });
    let stopped = receiver.recv_timeout(Duration::from_millis(500));
    drop(client);
    worker.join().unwrap();
    assert!(stopped.is_ok(), "incomplete GET prevented Stop");
    assert!(TcpStream::connect(address).is_err());
}
#[test]
fn oversized_ambiguous_and_incomplete_requests_are_bounded() {
    let app = Workbench::bind(0).unwrap();
    let address = app.address();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    let worker = thread::spawn(move || app.serve(signal).unwrap());
    for (headers, expected) in [
        ("Content-Length: 18446744073709551615\r\n", 413),
        ("Content-Length: 1\r\nContent-Length: 1\r\n", 400),
        ("Transfer-Encoding: chunked\r\n", 501),
        ("Host: evil.example\r\n", 400),
    ] {
        let mut client = connect(address);
        write!(
            client,
            "POST /api/build HTTP/1.1\r\nHost: {address}\r\n{headers}\r\n"
        )
        .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}")),
            "{response}"
        );
    }
    let mut client = connect(address);
    write!(
        client,
        "POST /api/build HTTP/1.1\r\nHost: {address}\r\nContent-Length: 20\r\n\r\nx"
    )
    .unwrap();
    let start = Instant::now();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 408"));
    assert!(start.elapsed() < Duration::from_secs(3));
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
}
