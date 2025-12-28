//! Chrome Tracing collector for internal job visualization.
//!
//! Provides a zero-contention tracer that records events into thread-local buffers.
//! Events can be exported to a JSON file compatible with chrome://tracing or ui.perfetto.dev.

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// A single trace event in Chrome Tracing format.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub name: &'static str,
    pub tid: usize,
    pub start_us: u64,
    pub duration_us: u64,
}

thread_local! {
    static TRACE_BUFFER: RefCell<Vec<TraceEvent>> = RefCell::new(Vec::with_capacity(10000));
}

lazy_static::lazy_static! {
    static ref GLOBAL_START: Instant = Instant::now();
    static ref EPOCH_START_US: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    static ref ALL_BUFFERS: Mutex<Vec<Vec<TraceEvent>>> = Mutex::new(Vec::new());
}

/// Registers the current thread's buffer for global export.
/// Should be called by each worker thread at startup.
pub fn register_thread() {
    // This is a bit tricky with thread-locals. 
    // We'll collect the buffers at the end instead.
}

/// Records a span of work.
pub fn record_event(name: &'static str, tid: usize, start: Instant, duration: std::time::Duration) {
    let start_us = (start.duration_since(*GLOBAL_START).as_micros() as u64) + *EPOCH_START_US;
    let duration_us = duration.as_micros() as u64;

    TRACE_BUFFER.with(|buf| {
        buf.borrow_mut().push(TraceEvent {
            name,
            tid,
            start_us,
            duration_us,
        });
    });
}

/// Collects all thread-local buffers into the global list.
/// MUST be called by EACH worker thread (e.g., at shutdown).
pub fn collect_local_trace() {
    TRACE_BUFFER.with(|buf| {
        let mut local_buf = buf.borrow_mut();
        if !local_buf.is_empty() {
            let mut global = ALL_BUFFERS.lock().unwrap();
            global.push(std::mem::take(&mut *local_buf));
        }
    });
}

/// Exports all collected trace events to a JSON file.
pub fn export_to_file(path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let buffers = ALL_BUFFERS.lock().unwrap();
    
    write!(writer, "[\n")?;
    let mut first = true;

    for buffer in buffers.iter() {
        for event in buffer {
            if !first {
                write!(writer, ",\n")?;
            }
            first = false;

            // ph: X is "Complete Event" (requires dur)
            write!(
                writer,
                "{{\"name\":\"{}\",\"ph\":\"X\",\"ts\":{},\"dur\":{},\"pid\":1,\"tid\":{}}}",
                event.name, event.start_us, event.duration_us, event.tid
            )?;
        }
    }

    write!(writer, "\n]\n")?;
    writer.flush()?;

    Ok(())
}

/// Helper for RAII tracing.
pub struct TraceGuard {
    name: &'static str,
    tid: usize,
    start: Instant,
}

impl TraceGuard {
    pub fn new(name: &'static str, tid: usize) -> Self {
        Self {
            name,
            tid,
            start: Instant::now(),
        }
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        record_event(self.name, self.tid, self.start, self.start.elapsed());
    }
}

/// RAII guard that collects the local trace when dropped.
pub struct CollectorGuard;

impl Drop for CollectorGuard {
    fn drop(&mut self) {
        collect_local_trace();
    }
}
