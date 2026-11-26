//! A one-connection HTTP server on localhost for the client tests (FEAT-0052,
//! FEAT-0056). Plain HTTP, so only a client with HTTPS not forced can reach it.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

/// Whether `raw` holds a request's headers and as many body bytes as its
/// `Content-Length` promised.
fn request_is_whole(raw: &[u8]) -> bool {
    let text = String::from_utf8_lossy(raw);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    let promised = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    body.len() >= promised
}

/// A one-connection HTTP server on localhost: it waits `delay`, answers
/// with `status` and `body`, and sends the raw request it read to the
/// returned receiver. Plain HTTP, so only a test client (HTTPS not forced)
/// can reach it.
pub(crate) fn serve(
    status: &'static str,
    body: Vec<u8>,
    delay: Duration,
) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}/index.php", listener.local_addr().expect("addr"));
    let (seen, requests) = mpsc::channel();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut raw = Vec::new();
        let mut chunk = [0u8; 1024];
        // Read until the headers and the whole form body are in: closing
        // a socket with unread bytes resets the connection.
        while !request_is_whole(&raw) {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => raw.extend_from_slice(&chunk[..n]),
            }
        }
        let _ = seen.send(String::from_utf8_lossy(&raw).into_owned());
        std::thread::sleep(delay);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let mut answer = head.into_bytes();
        answer.extend_from_slice(&body);
        let _ = stream.write_all(&answer);
        // Stay open until the client closes: closing first, or shutting
        // down the write side, resets the connection under it on Windows.
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        while matches!(stream.read(&mut chunk), Ok(n) if n > 0) {}
    });
    (base, requests)
}
