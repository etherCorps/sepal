//! The performance overlay: frame rate, frame time, memory, CPU, GPU and what
//! the app did, in a box in the top-right corner of the window.
//!
//! The host owns its nodes. They hang off `<html>`, after `<body>`, so the app
//! never sees them: they are not in the node map, no op names them, and
//! `body > *` does not find them. Every property that matters is set inline,
//! so the app's stylesheet cannot restyle the box, and `pointer-events: none`
//! keeps it out of hit testing.
//!
//! Hidden, it has no nodes and asks for no wake-ups. Shown, it refreshes once
//! a second, through a one-shot deadline that each refresh sets again (see
//! `SvelteDocument::settle`). Only `settle` calls [`Hud::update`], which is
//! where the tree changes.

use std::time::{Duration, Instant};

use blitz_dom::{BaseDocument, LocalName, NodeId, QualName, ns};

const EVERY: Duration = Duration::from_secs(1);

const STYLE: &str = "position: fixed; top: 8px; right: 8px; z-index: 2147483647; \
    pointer-events: none; display: block; box-sizing: border-box; width: auto; height: auto; \
    min-width: 0; max-width: none; margin: 0; padding: 6px 8px; border: 0; border-radius: 6px; \
    background: rgba(12, 12, 16, 0.82); color: #e6e6e6; opacity: 1; transform: none; \
    box-shadow: none; font-family: Menlo, ui-monospace, monospace; font-size: 11px; \
    font-weight: 400; font-style: normal; line-height: 15px; letter-spacing: 0; \
    text-align: left; text-transform: none; white-space: pre;";

const ENGINE: &str = if cfg!(feature = "engine-v8") {
    "V8"
} else {
    "QuickJS"
};

/// What the document knows that goes on the overlay.
pub(crate) struct AppNumbers {
    pub live_nodes: usize,
    /// Layouts done so far by the document itself (measurements, scrolls),
    /// not those the window does before painting: those are the frames.
    pub layouts: u64,
}

/// Counts since the last refresh.
struct Interval {
    began: Instant,
    frames: u32,
    frame_total: Duration,
    frame_max: Duration,
    cpu: Option<Duration>,
    layouts: u64,
}

impl Interval {
    fn start(layouts: u64) -> Self {
        Self {
            began: Instant::now(),
            frames: 0,
            frame_total: Duration::ZERO,
            frame_max: Duration::ZERO,
            cpu: cpu_time(),
            layouts,
        }
    }
}

#[derive(Default)]
pub(crate) struct Hud {
    shown: bool,
    /// The box and its text node, while they exist.
    nodes: Option<(NodeId, NodeId)>,
    /// When the numbers are next refreshed.
    due: Option<Instant>,
    interval: Option<Interval>,
    /// The last turn that changed anything: how long it took, and its ops.
    last_turn: Option<(Duration, usize)>,
}

impl Hud {
    pub fn show(&mut self, shown: bool) {
        self.shown = shown;
    }

    pub fn shown(&self) -> bool {
        self.shown
    }

    pub fn due(&self) -> Option<Instant> {
        self.due
    }

    /// The window painted a frame, which took this long on the UI thread.
    pub fn frame(&mut self, took: Duration) {
        if let Some(interval) = &mut self.interval {
            interval.frames += 1;
            interval.frame_total += took;
            interval.frame_max = interval.frame_max.max(took);
        }
    }

    /// A turn ended. Turns that did nothing (most of them) are not shown.
    pub fn turn(&mut self, took: Duration, ops: usize) {
        if self.shown && ops > 0 {
            self.last_turn = Some((took, ops));
        }
    }

    /// Bring the tree in line with what is wanted, and refresh the numbers
    /// when they are due. Returns whether the page changed.
    pub fn update(&mut self, doc: &mut BaseDocument, app: &AppNumbers) -> bool {
        match (self.shown, self.nodes) {
            (false, None) => false,
            (false, Some((element, _))) => {
                doc.mutate().remove_and_drop_node(element);
                *self = Hud::default();
                true
            }
            (true, None) => {
                let html = doc.root_element().id;
                let mut m = doc.mutate();
                let element = m.create_element(
                    QualName::new(None, ns!(html), LocalName::from("div")),
                    Vec::new(),
                );
                m.set_attribute(element, attribute("style"), STYLE);
                m.set_attribute(element, attribute("data-sepal-hud"), "");
                let text = m.create_text_node("");
                m.append_children(element, &[text]);
                m.append_children(html, &[element]);
                drop(m);
                self.nodes = Some((element, text));
                self.refresh(doc, app);
                true
            }
            (true, Some(_)) => {
                // The wake-up comes a little after the deadline; never before.
                let due = self
                    .due
                    .is_none_or(|due| due <= Instant::now() + Duration::from_millis(5));
                if due {
                    self.refresh(doc, app);
                }
                due
            }
        }
    }

    fn refresh(&mut self, doc: &mut BaseDocument, app: &AppNumbers) {
        let text = self.text(app);
        // So a script driving a real window can read the numbers too.
        if crate::document::trace_enabled() {
            eprintln!(
                "[trace] hud: {}",
                text.split_whitespace().collect::<Vec<_>>().join(" ")
            );
        }
        if let Some((_, node)) = self.nodes {
            doc.mutate().set_node_text(node, &text);
        }
        self.interval = Some(Interval::start(app.layouts));
        self.due = Some(Instant::now() + EVERY);
    }

    fn text(&self, app: &AppNumbers) -> String {
        let mut lines = Vec::new();
        // The first refresh, right after the box appears, has nothing to count yet.
        match &self.interval {
            Some(interval) if interval.began.elapsed() >= Duration::from_millis(100) => {
                let seconds = interval.began.elapsed().as_secs_f64();
                let fps = interval.frames as f64 / seconds;
                let avg = ms(interval.frame_total) / interval.frames.max(1) as f64;
                lines.push(format!(
                    "{fps:>3.0} fps   frame {avg:.1} / {:.1} ms",
                    ms(interval.frame_max)
                ));
                let mut usage = String::new();
                if let (Some(then), Some(now)) = (interval.cpu, cpu_time()) {
                    let busy = now.saturating_sub(then).as_secs_f64() / seconds;
                    usage = format!("CPU {:.0}%   ", busy * 100.0);
                }
                if let Some(gpu) = gpu_utilisation() {
                    usage.push_str(&format!("GPU (system) {gpu:.0}%"));
                }
                if !usage.is_empty() {
                    lines.push(usage.trim_end().to_owned());
                }
                let layouts = app.layouts - interval.layouts;
                lines.push(format!("layouts {layouts}/s   {ENGINE}"));
            }
            _ => lines.push("  - fps   frame - / - ms".to_owned()),
        }
        if let Some((now, peak)) = memory() {
            lines.push(format!("mem {:.0} MB   peak {:.0} MB", mb(now), mb(peak)));
        }
        let turn = match self.last_turn {
            Some((took, ops)) => format!("{:.1} ms, {ops} ops", ms(took)),
            None => "-".to_owned(),
        };
        lines.push(format!("nodes {}   turn {turn}", app.live_nodes));
        lines.join("\n")
    }
}

fn attribute(name: &str) -> QualName {
    QualName::new(None, ns!(), LocalName::from(name))
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1048576.0
}

/// User plus system time of the whole process, all threads.
#[cfg(unix)]
fn cpu_time() -> Option<Duration> {
    let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return None;
    }
    let time = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    Some(time(usage.ru_utime) + time(usage.ru_stime))
}

#[cfg(not(unix))]
fn cpu_time() -> Option<Duration> {
    None
}

/// The process's memory now and at its peak, in bytes. On macOS the physical
/// footprint, which is what Activity Monitor calls "Memory".
#[cfg(target_os = "macos")]
fn memory() -> Option<(u64, u64)> {
    let mut info = unsafe { std::mem::zeroed::<libc::rusage_info_v4>() };
    let pointer = (&mut info as *mut libc::rusage_info_v4).cast();
    let ok =
        unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, pointer) };
    (ok == 0).then_some((info.ri_phys_footprint, info.ri_lifetime_max_phys_footprint))
}

/// Resident set size, now and at its peak.
#[cfg(target_os = "linux")]
fn memory() -> Option<(u64, u64)> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let mut usage = unsafe { std::mem::zeroed::<libc::rusage>() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    // Kilobytes on Linux.
    Some((pages * page, usage.ru_maxrss as u64 * 1024))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn memory() -> Option<(u64, u64)> {
    None
}

/// How busy the GPU is, in percent, as the driver reports it to IOKit. This
/// is the whole machine's GPU, not this process's share: macOS has no
/// per-process figure short of Instruments.
#[cfg(target_os = "macos")]
fn gpu_utilisation() -> Option<f64> {
    use std::ffi::{c_char, c_void};
    use std::ptr::null;

    type CFTypeRef = *const c_void;
    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingServices(
            port: u32,
            matching: *mut c_void,
            iterator: *mut u32,
        ) -> i32;
        fn IOIteratorNext(iterator: u32) -> u32;
        fn IOObjectRelease(object: u32) -> i32;
        fn IORegistryEntryCreateCFProperty(
            entry: u32,
            key: CFTypeRef,
            allocator: CFTypeRef,
            options: u32,
        ) -> CFTypeRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            allocator: CFTypeRef,
            text: *const c_char,
            encoding: u32,
        ) -> CFTypeRef;
        fn CFDictionaryGetValue(dictionary: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(number: CFTypeRef, kind: isize, value: *mut c_void) -> u8;
        fn CFGetTypeID(object: CFTypeRef) -> usize;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFRelease(object: CFTypeRef);
    }
    const UTF8: u32 = 0x0800_0100;
    const SINT64: isize = 4;
    // kIOMainPortDefault
    const MAIN_PORT: u32 = 0;

    // SAFETY: the usual IOKit and CoreFoundation ownership rules. Matching
    // consumes the dictionary; everything created or copied here is released.
    unsafe {
        let mut iterator = 0;
        let matching = IOServiceMatching(c"IOAccelerator".as_ptr());
        if matching.is_null()
            || IOServiceGetMatchingServices(MAIN_PORT, matching, &mut iterator) != 0
        {
            return None;
        }
        let statistics = CFStringCreateWithCString(null(), c"PerformanceStatistics".as_ptr(), UTF8);
        let utilisation = CFStringCreateWithCString(null(), c"Device Utilization %".as_ptr(), UTF8);
        let mut busiest: Option<i64> = None;
        loop {
            let entry = IOIteratorNext(iterator);
            if entry == 0 {
                break;
            }
            let stats = IORegistryEntryCreateCFProperty(entry, statistics, null(), 0);
            if !stats.is_null() {
                if CFGetTypeID(stats) == CFDictionaryGetTypeID() {
                    let value = CFDictionaryGetValue(stats, utilisation);
                    let mut percent: i64 = 0;
                    if !value.is_null()
                        && CFGetTypeID(value) == CFNumberGetTypeID()
                        && CFNumberGetValue(value, SINT64, (&mut percent as *mut i64).cast()) != 0
                    {
                        // Two GPUs (an Intel Mac): show the busier.
                        busiest = Some(busiest.map_or(percent, |b| b.max(percent)));
                    }
                }
                CFRelease(stats);
            }
            IOObjectRelease(entry);
        }
        IOObjectRelease(iterator);
        CFRelease(statistics);
        CFRelease(utilisation);
        busiest.map(|percent| percent as f64)
    }
}

#[cfg(not(target_os = "macos"))]
fn gpu_utilisation() -> Option<f64> {
    None
}
