//! Bounds for the tests of alice-markup, so a parser that stops advancing
//! fails its test instead of running on:
//!
//! - [`Budget`], a global allocator with a ceiling on the live bytes: a loop
//!   that keeps allocating fails at the ceiling (Rust stops the test process
//!   with `memory allocation of N bytes failed`) instead of filling the
//!   machine's memory.
//! - [`within`], a deadline on one call: a loop that spins without allocating
//!   fails its test at the deadline instead of running until an outer timeout.
//!
//! The alice-markup crate forbids `unsafe`, and an allocator needs it, so these
//! live here, as a dev-dependency that is never published.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

/// Runs `f` on its own thread and returns its result, or panics (failing the
/// calling test) when it has not returned within `limit`.
///
/// A thread that missed the deadline cannot be stopped: it keeps running (a
/// spinning loop keeps a core busy) until the test binary exits, which it does
/// once the remaining tests have run.
///
/// # Panics
///
/// When `f` panics (the panic is passed on) or does not return within `limit`.
pub fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    let call = std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(limit) {
        Ok(v) => v,
        Err(mpsc::RecvTimeoutError::Disconnected) => match call.join() {
            Err(e) => std::panic::resume_unwind(e),
            Ok(()) => unreachable!("the thread sends before it returns"),
        },
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("no result within {limit:?}"),
    }
}

/// The system allocator with a ceiling on the live bytes.
pub struct Budget {
    limit: usize,
    live: AtomicUsize,
    peak: AtomicUsize,
}

impl Budget {
    /// A budget of `limit` live bytes.
    #[must_use]
    pub const fn new(limit: usize) -> Self {
        Self {
            limit,
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }

    /// The most bytes held at once so far.
    #[must_use]
    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    /// Reserves `size` more bytes, or refuses when the total would pass the
    /// ceiling.
    fn take(&self, size: usize) -> bool {
        let mut live = self.live.load(Ordering::Relaxed);
        loop {
            let Some(next) = live.checked_add(size).filter(|&n| n <= self.limit) else {
                return false;
            };
            match self
                .live
                .compare_exchange_weak(live, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => {
                    self.peak.fetch_max(next, Ordering::Relaxed);
                    return true;
                }
                Err(now) => live = now,
            }
        }
    }

    fn give(&self, size: usize) {
        self.live.fetch_sub(size, Ordering::Relaxed);
    }
}

// SAFETY: every call goes to `System` with the caller's layout unchanged; the
// counters only decide whether to call it, and a refusal returns null, which
// the `GlobalAlloc` contract allows.
unsafe impl GlobalAlloc for Budget {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.take(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds `alloc`'s contract for `layout`.
        let p = unsafe { System.alloc(layout) };
        if p.is_null() {
            self.give(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !self.take(layout.size()) {
            return std::ptr::null_mut();
        }
        // SAFETY: as for `alloc`.
        let p = unsafe { System.alloc_zeroed(layout) };
        if p.is_null() {
            self.give(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator with `layout`, so from `System`.
        unsafe { System.dealloc(ptr, layout) };
        self.give(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old = layout.size();
        if new_size > old && !self.take(new_size - old) {
            return std::ptr::null_mut();
        }
        // SAFETY: the caller upholds `realloc`'s contract; `ptr` is from `System`.
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if p.is_null() {
            if new_size > old {
                self.give(new_size - old);
            }
        } else if new_size < old {
            self.give(old - new_size);
        }
        p
    }
}
