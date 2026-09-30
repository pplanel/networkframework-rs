use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use networkframework::{
    AdvertiseDescriptor, ConnectionParameters, Endpoint, NetworkError, TcpListener,
};

fn loopback_tcp() -> Result<ConnectionParameters, NetworkError> {
    let mut parameters = ConnectionParameters::tcp()?;
    parameters.set_local_endpoint(Some(&Endpoint::address("127.0.0.1", 0)?));
    Ok(parameters)
}

fn unique_service_name() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    format!("networkframework-test-{stamp}")
}

#[test]
fn builder_binds_from_parameters_and_accepts() -> Result<(), NetworkError> {
    let listener = TcpListener::builder(&loopback_tcp()?).bind()?;
    let port = listener.local_port();
    assert_ne!(port, 0);

    let peer = thread::spawn(move || {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream.write_all(b"ping").expect("write");
    });
    let client = listener.accept()?;
    assert_eq!(client.receive(4)?, b"ping");
    peer.join().expect("peer thread");
    Ok(())
}

#[test]
fn builder_port_zero_reports_the_bound_port() -> Result<(), NetworkError> {
    let listener = TcpListener::builder(&loopback_tcp()?).port(0).bind()?;
    assert_ne!(listener.local_port(), 0);
    Ok(())
}

#[test]
fn builder_applies_the_new_connection_limit_before_start() -> Result<(), NetworkError> {
    let listener = TcpListener::builder(&loopback_tcp()?)
        .new_connection_limit(3)
        .bind()?;
    assert_eq!(listener.new_connection_limit(), 3);
    Ok(())
}

#[test]
fn builder_rejects_a_launchd_key_with_a_nul_byte() -> Result<(), NetworkError> {
    let result = TcpListener::builder(&loopback_tcp()?)
        .launchd_key("bad\0key")
        .bind();
    assert!(matches!(result, Err(NetworkError::InvalidArgument(_))));
    Ok(())
}

#[test]
fn advertise_descriptor_can_be_cleared_on_a_running_listener() -> Result<(), NetworkError> {
    let mut listener = TcpListener::builder(&loopback_tcp()?).bind()?;
    // Clearing an advertisement that was never set is a no-op and sends nothing.
    listener.set_advertise_descriptor(None);
    assert_ne!(listener.local_port(), 0);
    Ok(())
}

#[test]
#[ignore = "publishes a Bonjour service over mDNS on the LAN and AWDL"]
fn advertised_peer_to_peer_listener_reports_its_service_name() -> Result<(), NetworkError> {
    let service_name = unique_service_name();
    let mut parameters = ConnectionParameters::tcp()?;
    parameters.set_include_peer_to_peer(true);
    let descriptor =
        AdvertiseDescriptor::bonjour_service(Some(&service_name), "_nfwtest._tcp", None)?;

    let (advertised_tx, advertised_rx) = mpsc::channel();
    let _listener = TcpListener::builder(&parameters)
        .advertise(descriptor)
        .on_advertised_endpoint(move |endpoint, added| {
            let _ = advertised_tx.send((endpoint.and_then(|e| e.bonjour_service_name()), added));
        })
        .bind()?;

    let (name, added) = advertised_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("advertised endpoint callback");
    assert!(added);
    assert_eq!(name.as_deref(), Some(service_name.as_str()));
    Ok(())
}
