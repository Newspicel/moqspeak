//! Raising the security level. The level is the count of leading zero bits in the hash of the
//! public key and an offset, so a higher level is a search for a better offset. Each thread holds
//! a hasher that already covers the public key, and hashes the offset alone onto it. The search
//! runs on its own threads and reports what it has reached.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sha1::{Digest, Sha1};

/// How many offsets a thread tries before it reports them.
const FLUSH: u64 = 4096;

/// How many characters the decimal form of an offset takes at most.
const DIGITS: usize = 20;

/// What a running search has reached.
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    /// How many offsets the search tried.
    pub tried: u64,
    /// The best level it found.
    pub level: i32,
    /// Whether threads are still working.
    pub running: bool,
}

/// A search for an offset that raises the level of one identity.
pub struct Search {
    identity: ts3proto::Identity,
    start_level: i32,
    shared: Arc<Shared>,
}

struct Shared {
    stop: AtomicBool,
    workers: AtomicUsize,
    tried: AtomicU64,
    best: Mutex<(i32, i64)>,
}

impl Search {
    /// Starts the search for an offset that reaches `target`. A target the identity already
    /// reaches starts nothing.
    pub fn start(identity: ts3proto::Identity, target: i32) -> Self {
        let start_level = identity.security_level();
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            workers: AtomicUsize::new(0),
            tried: AtomicU64::new(0),
            best: Mutex::new((start_level, identity.offset)),
        });
        if target > start_level {
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            shared.workers.store(threads, Ordering::SeqCst);
            for lane in 0..threads {
                let prefix = identity.public_key_base64().to_owned();
                let shared = shared.clone();
                let first = identity.offset.saturating_add(1 + lane as i64);
                let stride = threads as i64;
                std::thread::Builder::new()
                    .name(format!("identity-level-{lane}"))
                    .spawn(move || hunt(&prefix, &shared, first, stride, target, start_level))
                    .expect("spawn an identity thread");
            }
        }
        Self {
            identity,
            start_level,
            shared,
        }
    }

    /// What the search has reached.
    pub fn progress(&self) -> Progress {
        let (level, _) = *self.shared.best.lock().expect("the best offset");
        Progress {
            tried: self.shared.tried.load(Ordering::Relaxed),
            level,
            running: self.shared.workers.load(Ordering::SeqCst) > 0,
        }
    }

    /// Ends the search. What it found is kept.
    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
    }

    /// The identity at the best offset found, if the search beat the level it started from.
    pub fn best(&self) -> Option<ts3proto::Identity> {
        let (level, offset) = *self.shared.best.lock().expect("the best offset");
        if level <= self.start_level {
            return None;
        }
        let mut better = self.identity.clone();
        better.offset = offset;
        Some(better)
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A hasher that already holds the public key, so a try hashes the offset alone.
fn primed(prefix: &str) -> Sha1 {
    let mut hasher = Sha1::new();
    hasher.update(prefix.as_bytes());
    hasher
}

/// The level the key reaches at `offset`.
fn level_at(primed: &Sha1, offset: i64) -> i32 {
    let mut buf = [0u8; DIGITS];
    let len = decimal(&mut buf, offset);
    let mut hasher = primed.clone();
    hasher.update(&buf[..len]);
    leading_zero_bits(&hasher.finalize())
}

/// The decimal form of `offset`, written into `buf`. Gives back how many characters it wrote.
fn decimal(buf: &mut [u8; DIGITS], offset: i64) -> usize {
    let mut rest = offset.unsigned_abs();
    let mut at = DIGITS;
    loop {
        at -= 1;
        buf[at] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if offset < 0 {
        at -= 1;
        buf[at] = b'-';
    }
    buf.copy_within(at.., 0);
    DIGITS - at
}

/// Tries every `stride`-th offset from `first` until one reaches `target`. `from` is the level the
/// identity already has, so only a better one is reported.
fn hunt(prefix: &str, shared: &Shared, first: i64, stride: i64, target: i32, from: i32) {
    let hasher = primed(prefix);
    let mut offset = first;
    let mut tried = 0u64;
    let mut seen = from;
    while !shared.stop.load(Ordering::Relaxed) {
        let level = level_at(&hasher, offset);
        if level > seen {
            let mut best = shared.best.lock().expect("the best offset");
            if level > best.0 {
                *best = (level, offset);
            }
            seen = best.0;
            drop(best);
            if seen >= target {
                shared.stop.store(true, Ordering::Relaxed);
                break;
            }
        }
        tried += 1;
        if tried.is_multiple_of(FLUSH) {
            shared.tried.fetch_add(FLUSH, Ordering::Relaxed);
        }
        offset = offset.saturating_add(stride);
    }
    shared.tried.fetch_add(tried % FLUSH, Ordering::Relaxed);
    shared.workers.fetch_sub(1, Ordering::SeqCst);
}

/// The count of leading zero bits, the way TeamSpeak counts them: from the lowest bit of a byte up.
fn leading_zero_bits(hash: &[u8]) -> i32 {
    let mut zeros = 0;
    for &byte in hash {
        if byte == 0 {
            zeros += 8;
            continue;
        }
        return zeros + byte.trailing_zeros() as i32;
    }
    zeros
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Waits for the search to finish, and gives up after `limit`.
    fn settle(search: &Search, limit: Duration) -> Progress {
        let until = Instant::now() + limit;
        loop {
            let progress = search.progress();
            if !progress.running || Instant::now() > until {
                return progress;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn writes_the_digits_of_an_offset() {
        for offset in [0i64, 7, 10, 99, 1_000_000, i64::MAX, -1, -42, i64::MIN] {
            let mut buf = [0u8; DIGITS];
            let len = decimal(&mut buf, offset);
            assert_eq!(
                std::str::from_utf8(&buf[..len]).expect("the digits are text"),
                offset.to_string()
            );
        }
    }

    #[test]
    fn reads_the_level_the_identity_reports() {
        let identity = ts3proto::generate_identity(0);
        let hasher = primed(identity.public_key_base64());
        for offset in 0..200i64 {
            let mut at = identity.clone();
            at.offset = offset;
            assert_eq!(
                level_at(&hasher, offset),
                at.security_level(),
                "the level at offset {offset}"
            );
        }
    }

    #[test]
    fn reaches_the_target_level() {
        let identity = ts3proto::generate_identity(0);
        let search = Search::start(identity.clone(), 12);
        let progress = settle(&search, Duration::from_secs(20));
        assert!(
            !progress.running,
            "the search ends once it reaches the target"
        );
        assert!(
            progress.level >= 12,
            "it reports the level it found: {}",
            progress.level
        );
        let better = search.best().expect("a better offset");
        assert!(better.security_level() >= 12);
        assert_eq!(better.public_key_base64(), identity.public_key_base64());
        assert_ne!(better.offset, identity.offset);
    }

    #[test]
    fn stops_when_asked() {
        let search = Search::start(ts3proto::generate_identity(0), 60);
        assert!(search.progress().running);
        search.stop();
        let progress = settle(&search, Duration::from_secs(5));
        assert!(
            !progress.running,
            "every thread leaves when the search stops"
        );
    }

    #[test]
    fn keeps_the_identity_when_the_level_is_already_reached() {
        let identity = ts3proto::generate_identity(12);
        let search = Search::start(identity, 8);
        assert!(!search.progress().running, "nothing to search for");
        assert!(search.best().is_none(), "the identity stays as it is");
    }
}
