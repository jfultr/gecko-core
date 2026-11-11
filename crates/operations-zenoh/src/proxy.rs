//! A TCP proxy whose link can be cut and restored, to test reconnection.
//! Test support only.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Forwards `127.0.0.1:<port>` to an upstream address until [`Proxy::cut`].
pub struct Proxy {
    port: u16,
    up: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    links: Arc<Mutex<Vec<TcpStream>>>,
}

impl Proxy {
    /// Starts a proxy on a free local port forwarding to `upstream`.
    ///
    /// # Errors
    ///
    /// The I/O error binding the listener.
    pub fn start(upstream: &str) -> io::Result<Self> {
        Self::start_at(0, upstream)
    }

    /// Starts a proxy on local `port` (0 picks a free one) forwarding to `upstream`.
    ///
    /// # Errors
    ///
    /// The I/O error binding the listener, for example a port in use.
    pub fn start_at(port: u16, upstream: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let up = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let links: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let upstream = upstream.to_string();
        let (up2, stop2, links2) = (Arc::clone(&up), Arc::clone(&stop), Arc::clone(&links));
        thread::spawn(move || {
            for incoming in listener.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(client) = incoming else { continue };
                if !up2.load(Ordering::SeqCst) {
                    continue; // dropped: the link is down
                }
                let Ok(server) = TcpStream::connect(&upstream) else {
                    continue;
                };
                let mut registry = links2.lock().expect("links");
                for (from, to) in [(&client, &server), (&server, &client)] {
                    if let (Ok(from), Ok(to)) = (from.try_clone(), to.try_clone()) {
                        thread::spawn(move || pipe(from, to));
                    }
                }
                registry.push(client);
                registry.push(server);
            }
        });
        Ok(Self {
            port,
            up,
            stop,
            links,
        })
    }

    /// The Zenoh endpoint clients connect to.
    pub fn endpoint(&self) -> String {
        format!("tcp/127.0.0.1:{}", self.port)
    }

    /// Drops every connection and refuses new ones until [`Proxy::restore`].
    pub fn cut(&self) {
        self.up.store(false, Ordering::SeqCst);
        for link in self.links.lock().expect("links").drain(..) {
            let _ = link.shutdown(Shutdown::Both);
        }
    }

    /// Whether the link is up (connections are accepted).
    pub fn is_up(&self) -> bool {
        self.up.load(Ordering::SeqCst)
    }

    /// Accepts connections again.
    pub fn restore(&self) {
        self.up.store(true, Ordering::SeqCst);
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.cut();
        let _ = TcpStream::connect(("127.0.0.1", self.port)); // wake the accept loop
    }
}

fn pipe(mut from: TcpStream, mut to: TcpStream) {
    let mut buffer = [0u8; 16 * 1024];
    while let Ok(n) = from.read(&mut buffer) {
        if n == 0 || to.write_all(&buffer[..n]).is_err() {
            break;
        }
    }
    let _ = to.shutdown(Shutdown::Both);
    let _ = from.shutdown(Shutdown::Both);
}
