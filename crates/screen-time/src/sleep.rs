use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use zbus::blocking::{Connection, Proxy};

use crate::backoff::{FailureLog, next_delay};
use crate::capture::Event;
use crate::timeutil;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const BASE_RETRY_DELAY: Duration = Duration::from_secs(5);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5 * 60);

const LOGIN1: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER_INTERFACE: &str = "org.freedesktop.login1.Manager";
const PREPARE_FOR_SLEEP: &str = "PrepareForSleep";

/// Ignore ordinary scheduler jitter. Real suspend gaps are usually much
/// longer; shorter sleeps still arrive as logind events.
const SUSPEND_GAP_THRESHOLD: Duration = Duration::from_secs(30);

/// Fallback detector for a suspend that ended between two polls.
///
/// `CLOCK_BOOTTIME` keeps advancing during suspend while `CLOCK_MONOTONIC`
/// stops. The difference between their deltas is therefore the time the
/// machine was asleep, even if no logind signal was delivered.
#[derive(Debug, Default)]
pub struct SuspendClock {
    boottime: Option<Duration>,
    monotonic: Option<Duration>,
}

impl SuspendClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn poll(&mut self) -> Option<Duration> {
        let boottime = clock_now(libc::CLOCK_BOOTTIME)?;
        let monotonic = clock_now(libc::CLOCK_MONOTONIC)?;
        let previous = self.boottime.zip(self.monotonic);
        self.boottime = Some(boottime);
        self.monotonic = Some(monotonic);
        let (previous_boottime, previous_monotonic) = previous?;
        detect_gap(previous_boottime, boottime, previous_monotonic, monotonic)
    }
}

fn clock_now(clock: libc::clockid_t) -> Option<Duration> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `value` is a valid, writable `timespec`.
    if unsafe { libc::clock_gettime(clock, &mut value) } != 0 {
        return None;
    }
    let seconds = u64::try_from(value.tv_sec).ok()?;
    let nanos = u32::try_from(value.tv_nsec).ok()?.min(999_999_999);
    Some(Duration::new(seconds, nanos))
}

fn detect_gap(
    previous_boottime: Duration,
    boottime: Duration,
    previous_monotonic: Duration,
    monotonic: Duration,
) -> Option<Duration> {
    let boottime_delta = boottime.saturating_sub(previous_boottime);
    let monotonic_delta = monotonic.saturating_sub(previous_monotonic);
    let suspended = boottime_delta.saturating_sub(monotonic_delta);
    (suspended >= SUSPEND_GAP_THRESHOLD).then_some(suspended)
}

#[derive(Debug, Clone)]
pub struct SleepProbe {
    pub available: bool,
    pub error: Option<String>,
}

/// Check whether the login1 manager that emits `PrepareForSleep` is reachable.
pub fn probe() -> SleepProbe {
    match probe_inner() {
        Ok(()) => SleepProbe {
            available: true,
            error: None,
        },
        Err(error) => SleepProbe {
            available: false,
            error: Some(format!("{error:#}")),
        },
    }
}

fn probe_inner() -> Result<()> {
    let connection = Connection::system().context("cannot connect to the system D-Bus")?;
    let proxy = Proxy::new(&connection, LOGIN1, MANAGER_PATH, MANAGER_INTERFACE)
        .context("cannot open the login1 manager proxy")?;
    let _: String = proxy
        .call("CanSuspend", &())
        .context("cannot query login1 CanSuspend")?;
    Ok(())
}

fn emit_sleep(tx: &Sender<Event>, asleep: bool) {
    let _ = tx.send(Event::Sleep {
        asleep,
        ts: timeutil::now().timestamp(),
    });
}

pub fn run_listener(tx: Sender<Event>, stop: Arc<AtomicBool>) {
    if std::env::var("SCREEN_TIME_DISABLE_SLEEP").as_deref() == Ok("1") {
        return;
    }
    let mut failures = 0u32;
    let mut log = FailureLog::default();
    while !stop.load(Ordering::Relaxed) {
        match listen(&tx, &stop) {
            Ok(()) => {
                failures = 0;
                if log.recovered() {
                    eprintln!("screen-time: sleep capture: recovered");
                }
                if !stop.load(Ordering::Relaxed) {
                    thread::sleep(RECONNECT_DELAY);
                }
            }
            Err(error) => {
                failures = failures.saturating_add(1);
                let message = format!("{error:#}");
                if log.should_log(&message) {
                    eprintln!("screen-time: sleep capture: {message}");
                }
                thread::sleep(next_delay(failures, BASE_RETRY_DELAY, MAX_RETRY_DELAY));
            }
        }
    }
}

fn listen(tx: &Sender<Event>, stop: &Arc<AtomicBool>) -> Result<()> {
    let connection = Connection::system().context("cannot connect to the system D-Bus")?;
    let proxy = Proxy::new(&connection, LOGIN1, MANAGER_PATH, MANAGER_INTERFACE)
        .context("cannot open the login1 manager proxy")?;
    let signals = proxy
        .receive_signal(PREPARE_FOR_SLEEP)
        .context("cannot subscribe to login1 PrepareForSleep")?;

    // A running daemon is awake. Setting the state explicitly on every
    // reconnect keeps focus tracking enabled after a missed wake signal.
    emit_sleep(tx, false);
    for message in signals {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let asleep: bool = message
            .body()
            .deserialize()
            .context("cannot decode login1 PrepareForSleep")?;
        emit_sleep(tx, asleep);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_non_panicking() {
        let probe = probe();
        assert!(probe.available || probe.error.is_some());
    }

    #[test]
    fn suspend_gap_is_the_boottime_minus_monotonic_delta() {
        let base = Duration::from_secs(1_000);
        assert_eq!(
            detect_gap(
                base,
                base + Duration::from_secs(3_600),
                base,
                base + Duration::from_secs(60)
            ),
            Some(Duration::from_secs(3_540))
        );
        assert_eq!(
            detect_gap(
                base,
                base + Duration::from_secs(60),
                base,
                base + Duration::from_secs(60)
            ),
            None
        );
        assert_eq!(
            detect_gap(
                base,
                base + Duration::from_secs(10),
                base,
                base + Duration::from_secs(40)
            ),
            None,
            "a monotonic jump must not be treated as suspend"
        );
    }

    #[test]
    fn resync_emits_the_awake_state() {
        let (tx, rx) = std::sync::mpsc::channel();
        emit_sleep(&tx, false);
        emit_sleep(&tx, true);
        assert!(matches!(
            rx.recv().unwrap(),
            Event::Sleep { asleep: false, .. }
        ));
        assert!(matches!(
            rx.recv().unwrap(),
            Event::Sleep { asleep: true, .. }
        ));
    }
}
