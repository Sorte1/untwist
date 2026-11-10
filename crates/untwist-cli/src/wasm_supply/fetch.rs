use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::time::{Duration, Instant};

use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

use super::address::{self, Filter};
use super::{printable, url};

const MEBIBYTE: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub references: usize,
    pub response_bytes: u64,
    pub total_bytes: u64,
    pub redirects: usize,
    pub phase: Duration,
    pub request: Duration,
}

pub const LIMITS: Limits = Limits {
    references: 16,
    response_bytes: untwist::MAX_WASM_BYTES as u64,
    total_bytes: 64 * MEBIBYTE,
    redirects: 3,
    phase: Duration::from_secs(120),
    request: Duration::from_secs(30),
};

#[derive(Debug)]
pub enum Failure {
    Stderr(io::Error),
    Fetch(String),
}

impl From<String> for Failure {
    fn from(reason: String) -> Self {
        Failure::Fetch(reason)
    }
}

#[derive(Debug)]
struct Refused {
    class: &'static str,
    address: IpAddr,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the host resolves to the {} address {}; pass --fetch-wasm-local to fetch from \
             such addresses",
            self.class, self.address
        )
    }
}

impl std::error::Error for Refused {}

#[derive(Debug)]
struct CheckedResolver {
    inner: DefaultResolver,
    filter: Filter,
}

impl Resolver for CheckedResolver {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let resolved = self.inner.resolve(uri, config, timeout)?;
        let refused = resolved.iter().find_map(|socket| {
            let address = socket.ip();
            (self.filter)(address).map(|class| Refused { class, address })
        });
        match refused {
            Some(refused) => Err(ureq::Error::Io(io::Error::new(
                io::ErrorKind::PermissionDenied,
                refused,
            ))),
            None => Ok(resolved),
        }
    }
}

fn reason(error: ureq::Error) -> String {
    if let ureq::Error::Io(inner) = &error {
        if let Some(refused) = inner.get_ref().and_then(|e| e.downcast_ref::<Refused>()) {
            return refused.to_string();
        }
    }
    error.to_string()
}

enum Reply {
    Body(Vec<u8>),
    Redirect(String),
}

pub struct Fetcher {
    agent: ureq::Agent,
    limits: Limits,
    started: Instant,
    attempts: usize,
    fetched_bytes: u64,
}

impl Fetcher {
    pub fn new(local: bool, limits: Limits) -> Self {
        let config = ureq::Agent::config_builder().max_redirects(0);
        match local {
            true => Self::with(config.build(), address::unfiltered, limits),
            false => Self::with(config.proxy(None).build(), address::refusal, limits),
        }
    }

    fn with(config: Config, filter: Filter, limits: Limits) -> Self {
        let resolver = CheckedResolver {
            inner: DefaultResolver::default(),
            filter,
        };
        Self {
            agent: ureq::Agent::with_parts(config, DefaultConnector::default(), resolver),
            limits,
            started: Instant::now(),
            attempts: 0,
            fetched_bytes: 0,
        }
    }

    fn remaining(&self) -> Option<Duration> {
        self.limits
            .phase
            .checked_sub(self.started.elapsed())
            .filter(|left| !left.is_zero())
    }

    fn budget(&self) -> u64 {
        self.limits.total_bytes.saturating_sub(self.fetched_bytes)
    }

    pub fn exhausted(&self) -> Option<String> {
        if self.attempts >= self.limits.references {
            return Some(format!(
                "--fetch-wasm fetches at most {} wasm modules in one run",
                self.limits.references
            ));
        }
        if self.budget() == 0 {
            return Some(format!(
                "--fetch-wasm fetches at most {} in one run",
                size(self.limits.total_bytes)
            ));
        }
        if self.remaining().is_none() {
            return Some(format!("--fetch-wasm stops after {:?}", self.limits.phase));
        }
        None
    }

    pub fn fetch(&mut self, target: &str, err: &mut dyn Write) -> Result<Vec<u8>, Failure> {
        self.attempts += 1;
        let mut next = target.to_string();
        for _ in 0..=self.limits.redirects {
            writeln!(err, "note: fetching {}", printable(&next)).map_err(Failure::Stderr)?;
            match self.request(&next)? {
                Reply::Body(bytes) => return Ok(bytes),
                Reply::Redirect(location) => {
                    next = url::joined(&next, &location).ok_or_else(|| {
                        format!("it redirects to {location}, which is not an http(s) URL")
                    })?;
                }
            }
        }
        Err(format!("more than {} redirects", self.limits.redirects).into())
    }

    fn request(&mut self, target: &str) -> Result<Reply, String> {
        let left = self
            .remaining()
            .ok_or_else(|| format!("--fetch-wasm stops after {:?}", self.limits.phase))?;
        let response = self
            .agent
            .get(target)
            .config()
            .timeout_global(Some(left.min(self.limits.request)))
            .build()
            .call()
            .map_err(reason)?;
        let status = response.status();
        if status.is_redirection() {
            let location = response.headers().get("location");
            return location
                .and_then(|value| value.to_str().ok())
                .map(|location| Reply::Redirect(location.to_string()))
                .ok_or_else(|| format!("http status {status} without a usable Location"));
        }
        if !status.is_success() {
            return Err(format!("http status {status}"));
        }
        let declared = response
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());
        let allowed = self.limits.response_bytes.min(self.budget());
        if declared.is_some_and(|length| length > allowed) {
            return Err(self.too_large(allowed));
        }
        let mut bytes = Vec::new();
        let read = response
            .into_body()
            .into_reader()
            .take(allowed + 1)
            .read_to_end(&mut bytes);
        self.fetched_bytes = self.fetched_bytes.saturating_add(bytes.len() as u64);
        read.map_err(|error| error.to_string())?;
        if bytes.len() as u64 > allowed {
            return Err(self.too_large(allowed));
        }
        Ok(Reply::Body(bytes))
    }

    fn too_large(&self, allowed: u64) -> String {
        match allowed < self.limits.response_bytes {
            true => format!(
                "it would pass the {} that --fetch-wasm fetches in one run",
                size(self.limits.total_bytes)
            ),
            false => format!("it is larger than {}", size(self.limits.response_bytes)),
        }
    }
}

fn size(bytes: u64) -> String {
    match bytes % MEBIBYTE {
        0 => format!("{} MiB", bytes / MEBIBYTE),
        _ => format!("{bytes} bytes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Server {
        origin: String,
        hits: Arc<AtomicUsize>,
    }

    fn serve(reply: impl Fn(&mut TcpStream) + Send + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counted = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                counted.fetch_add(1, Ordering::SeqCst);
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                reply(&mut stream);
            }
        });
        Server { origin, hits }
    }

    fn head(status: &str, length: usize, extra: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\nConnection: close\r\n{extra}\r\n")
    }

    fn serve_body(body: &'static [u8]) -> Server {
        serve(move |stream| {
            let _ = stream.write_all(head("200 OK", body.len(), "").as_bytes());
            let _ = stream.write_all(body);
        })
    }

    fn limits() -> Limits {
        Limits {
            request: Duration::from_secs(10),
            phase: Duration::from_secs(10),
            ..LIMITS
        }
    }

    fn local(limits: Limits) -> Fetcher {
        Fetcher::with(
            ureq::Agent::config_builder()
                .max_redirects(0)
                .proxy(None)
                .build(),
            address::unfiltered,
            limits,
        )
    }

    fn reason_of(outcome: Result<Vec<u8>, Failure>) -> String {
        match outcome {
            Err(Failure::Fetch(reason)) => reason,
            other => panic!("expected a fetch failure, got {other:?}"),
        }
    }

    #[test]
    fn a_body_within_the_limits_is_returned() {
        let server = serve_body(b"12345678");
        let mut fetcher = local(limits());
        let mut err = Vec::new();

        let bytes = fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut err);

        assert_eq!(bytes.unwrap(), b"12345678");
        assert_eq!(
            String::from_utf8(err).unwrap(),
            format!("note: fetching {}/a.wasm\n", server.origin)
        );
    }

    #[test]
    fn the_default_filter_refuses_a_loopback_host_before_connecting() {
        let server = serve_body(b"12345678");
        let port = server.origin.rsplit(':').next().unwrap();
        let mut fetcher = Fetcher::new(false, limits());

        for host in ["127.0.0.1", "localhost"] {
            let target = format!("http://{host}:{port}/a.wasm");
            let reason = reason_of(fetcher.fetch(&target, &mut Vec::new()));
            assert!(
                reason.contains("the host resolves to the loopback address"),
                "{reason}"
            );
            assert!(reason.contains("--fetch-wasm-local"), "{reason}");
        }
        assert_eq!(server.hits.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_redirect_to_a_refused_address_is_not_followed() {
        fn only_the_first(address: IpAddr) -> Option<&'static str> {
            (address != IpAddr::from([127, 0, 0, 1])).then_some("test")
        }
        let server = serve(|stream| {
            let location = "Location: http://127.0.0.2:9/a.wasm\r\n";
            let _ = stream.write_all(head("302 Found", 0, location).as_bytes());
        });
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .proxy(None)
            .build();
        let mut fetcher = Fetcher::with(config, only_the_first, limits());
        let mut err = Vec::new();

        let reason = reason_of(fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut err));

        assert!(
            reason.contains("the host resolves to the test address 127.0.0.2"),
            "{reason}"
        );
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
        assert!(String::from_utf8(err)
            .unwrap()
            .ends_with("note: fetching http://127.0.0.2:9/a.wasm\n"));
    }

    #[test]
    fn a_body_longer_than_one_response_may_be_is_refused() {
        let server = serve_body(b"123456789");
        let mut fetcher = local(Limits {
            response_bytes: 8,
            ..limits()
        });

        let reason =
            reason_of(fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut Vec::new()));

        assert_eq!(reason, "it is larger than 8 bytes");
    }

    #[test]
    fn a_declared_length_over_the_limit_is_refused_without_reading_the_body() {
        let server = serve(|stream| {
            let _ = stream.write_all(head("200 OK", 1_000_000, "").as_bytes());
            std::thread::sleep(Duration::from_secs(30));
        });
        let mut fetcher = local(Limits {
            response_bytes: 8,
            ..limits()
        });
        let started = Instant::now();

        let reason =
            reason_of(fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut Vec::new()));

        assert_eq!(reason, "it is larger than 8 bytes");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(fetcher.fetched_bytes, 0);
    }

    #[test]
    fn the_total_of_all_bodies_is_capped() {
        let server = serve_body(b"12345678");
        let target = format!("{}/a.wasm", server.origin);
        let mut fetcher = local(Limits {
            total_bytes: 12,
            ..limits()
        });

        let first = fetcher.fetch(&target, &mut Vec::new());
        let second = reason_of(fetcher.fetch(&target, &mut Vec::new()));

        assert_eq!(first.unwrap().len(), 8);
        assert_eq!(
            second,
            "it would pass the 12 bytes that --fetch-wasm fetches in one run"
        );
        assert_eq!(fetcher.exhausted(), None);
    }

    #[test]
    fn nothing_more_is_fetched_once_the_total_is_used_up() {
        let server = serve_body(b"12345678");
        let mut fetcher = local(Limits {
            total_bytes: 8,
            ..limits()
        });

        let first = fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut Vec::new());

        assert_eq!(first.unwrap().len(), 8);
        assert_eq!(
            fetcher.exhausted().as_deref(),
            Some("--fetch-wasm fetches at most 8 bytes in one run")
        );
    }

    #[test]
    fn the_number_of_fetched_references_is_capped() {
        let server = serve_body(b"12345678");
        let target = format!("{}/a.wasm", server.origin);
        let mut fetcher = local(Limits {
            references: 2,
            ..limits()
        });

        let before = fetcher.exhausted();
        let _ = fetcher.fetch(&target, &mut Vec::new());
        let _ = fetcher.fetch(&target, &mut Vec::new());

        assert_eq!(before, None);
        assert_eq!(
            fetcher.exhausted().as_deref(),
            Some("--fetch-wasm fetches at most 2 wasm modules in one run")
        );
    }

    #[test]
    fn a_server_that_never_answers_is_dropped_at_the_deadline() {
        let server = serve(|_| std::thread::sleep(Duration::from_secs(30)));
        let mut fetcher = local(Limits {
            phase: Duration::from_millis(300),
            ..limits()
        });
        let started = Instant::now();

        let first = fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut Vec::new());

        assert!(matches!(first, Err(Failure::Fetch(_))), "{first:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            fetcher.exhausted().as_deref(),
            Some("--fetch-wasm stops after 300ms")
        );
    }

    #[test]
    fn an_error_status_is_a_failure() {
        let server = serve(|stream| {
            let _ = stream.write_all(head("404 Not Found", 0, "").as_bytes());
        });
        let mut fetcher = local(limits());

        let reason =
            reason_of(fetcher.fetch(&format!("{}/a.wasm", server.origin), &mut Vec::new()));

        assert!(reason.contains("404"), "{reason}");
    }
}
