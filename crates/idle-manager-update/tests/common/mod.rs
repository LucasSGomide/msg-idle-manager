//! A throwaway directory for the update crate's integration tests, removed
//! on drop — the same shape as `idle-manager-store`'s own `tests/common` —
//! and a throwaway HTTP server for the tests that must exercise
//! `VelopackChannel` against a release feed without ever reaching the real
//! GitHub.

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "idle-manager-update-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).expect("create the temporary directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}

/// Serves a fixed set of byte responses over plain HTTP on `127.0.0.1`, so a
/// test can point `sources::HttpSource` at a release feed and its assets
/// without a real network (CLAUDE.md: never the real GitHub in a test).
///
/// Runs its accept loop on a detached background thread for as long as the
/// test binary itself runs — there is no shutdown, since nothing here ever
/// starts more of these than a handful of short-lived tests need.
pub(crate) struct FixtureServer {
    addr: SocketAddr,
}

impl FixtureServer {
    /// Starts serving `routes` — each key the request path with no leading
    /// slash (`releases.linux.json`, an asset's file name) mapped to the
    /// bytes it answers with. Any other path answers `404`.
    pub(crate) fn start(routes: HashMap<String, Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind an ephemeral port");
        let addr = listener
            .local_addr()
            .expect("a bound listener reports its own address");

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                if let Err(error) = Self::serve_one(&mut stream, &routes) {
                    eprintln!("fixture server: {error}");
                }
            }
        });

        Self { addr }
    }

    /// The base URL to give `sources::HttpSource::new`.
    pub(crate) fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn serve_one(stream: &mut TcpStream, routes: &HashMap<String, Vec<u8>>) -> std::io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);

        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        loop {
            let mut header_line = String::new();
            if reader.read_line(&mut header_line)? == 0
                || header_line == "\r\n"
                || header_line == "\n"
            {
                break;
            }
        }

        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .trim_start_matches('/')
            .split('?')
            .next()
            .unwrap_or("");

        if let Some(body) = routes.get(path) {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(body)
        } else {
            let body = b"not found";
            write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(body)
        }
    }
}
