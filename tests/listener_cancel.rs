use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use networkframework::{NetworkError, TcpListener};

#[test]
fn cancel_wakes_an_accept_blocked_on_another_thread() -> Result<(), NetworkError> {
    let listener = Arc::new(TcpListener::bind_loopback(0)?);
    let acceptor = Arc::clone(&listener);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(acceptor.accept().map(drop));
    });
    thread::sleep(Duration::from_millis(200));
    listener.cancel();
    let accepted = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("accept stayed blocked after cancel");
    assert!(
        matches!(accepted, Err(NetworkError::Cancelled)),
        "expected Cancelled, got {accepted:?}"
    );
    Ok(())
}

#[test]
fn cancel_stops_listening() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    let port = listener.local_port();
    listener.cancel();
    thread::sleep(Duration::from_millis(200));
    let connected = std::net::TcpStream::connect_timeout(
        &([127, 0, 0, 1], port).into(),
        Duration::from_secs(2),
    );
    assert!(connected.is_err(), "port {port} still accepts after cancel");
    Ok(())
}

#[test]
fn cancel_twice_and_then_drop_is_harmless() -> Result<(), NetworkError> {
    let listener = TcpListener::bind_loopback(0)?;
    listener.cancel();
    listener.cancel();
    drop(listener);
    Ok(())
}
