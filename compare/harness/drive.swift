// drive: real OS input, window info and window capture for the comparison harness.
//
//   drive perm                         {"ax":bool,"screen":bool}; exit 1 unless both
//   drive clock                        mach_absolute_time in ns (same clock as every "t" below)
//   drive machine                      model, macOS, displays (mode, refresh)
//   drive windows <pid>                on-screen layer-0 windows of pid (CG coords, points)
//   drive ax <pid>                     AX frame of each window and its children (title bar check)
//   drive front <pid>                  is pid frontmost, is its window on screen
//   drive focus <pid> [ax]             NSRunningApplication.isActive and frontmost: is a key press
//                                      going to land. With `ax`, also through AX (50 ms timeout)
//                                      whether the app has a focused window and whether it is
//                                      main. AX is not free: the first AX client makes the app
//                                      build its accessibility tree (WebKit, AccessKit), which
//                                      delayed the next key press by ~15 ms in every build.
//   drive activate <pid>               bring pid to the front
//   drive resize <pid> <w> <h>         set the AX frame size of pid's first window, read it back
//   drive procs <pid>                  pid plus every process it is responsible for or parent of
//   drive record <windowid> <seconds> <out> [scale]
//                                      ScreenCaptureKit, one line per delivered frame
//   drive launch <exe> [args...]       exec exe in place (same pid, same stdio) as its own
//                                      responsible process, so the XPC helpers it starts
//                                      (WebKit's) name it, not the terminal, as responsible
//   drive ps                           every process: pid, ppid, responsible pid, path
//   drive usage <pid>...               proc_pid_rusage (V6) totals: CPU ns, wake-ups, footprint, energy
//   drive serve                        read commands from stdin (any of these but perm/record/
//                                      launch), one JSON reply per line
//   drive as <pid> <input command>     post only while pid is frontmost; else {"refused":...}
//   drive <input command>              one input command (see `input` below), then exit
//
// Input commands (all coordinates are global screen points, top-left origin):
//   move x y | click x y | down x y | up x y
//   key <keycode|name> [cmd,shift,alt,ctrl]
//   type <text> [interval_ms]          one keycode per char (US ANSI layout)
//   wheel x y <dy> <count> <hz>        pixel scroll events at a fixed rate
//   clock
// `click` and `key` take `hold=<ms>` anywhere after the command: how long the button or key
// stays down (default 30 ms for click, 8 ms for key; calibration probes with 300 ms to tell
// whether a control acts on the press or on the release).
// Every reply carries "t": mach ns of each event posted (taken right after CGEvent.post);
// click, key and type also name them: "move", "down", "up" (type: arrays, one per char).

import AppKit
import ApplicationServices
import CoreGraphics
import CoreMedia
import Foundation
import ScreenCaptureKit

// MARK: clock

let timebase: mach_timebase_info_data_t = {
	var info = mach_timebase_info_data_t()
	mach_timebase_info(&info)
	return info
}()
@inline(__always) func now_ns() -> UInt64 { mach_absolute_time() * UInt64(timebase.numer) / UInt64(timebase.denom) }
func ticks_to_ns(_ t: UInt64) -> UInt64 { t * UInt64(timebase.numer) / UInt64(timebase.denom) }

func json(_ v: Any) -> String {
	let data = try! JSONSerialization.data(withJSONObject: v, options: [.sortedKeys])
	return String(data: data, encoding: .utf8)!
}
func out(_ v: Any) {
	print(json(v))
	fflush(stdout)
}
func fail(_ msg: String) -> Never {
	FileHandle.standardError.write(("drive: " + msg + "\n").data(using: .utf8)!)
	exit(1)
}

// MARK: input

let source = CGEventSource(stateID: .hidSystemState)

// US ANSI virtual keycodes.
let keycodes: [Character: CGKeyCode] = [
	"a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12,
	"w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23,
	"=": 24, "9": 25, "7": 26, "-": 27, "8": 28, "0": 29, "]": 30, "o": 31, "u": 32, "[": 33, "i": 34,
	"p": 35, "l": 37, "j": 38, "'": 39, "k": 40, ";": 41, "\\": 42, ",": 43, "/": 44, "n": 45, "m": 46,
	".": 47, " ": 49, "`": 50,
]
let shifted: [Character: Character] = [
	"!": "1", "@": "2", "#": "3", "$": "4", "%": "5", "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
	"_": "-", "+": "=", "{": "[", "}": "]", "|": "\\", ":": ";", "\"": "'", "<": ",", ">": ".", "?": "/", "~": "`",
]
let named: [String: CGKeyCode] = [
	"return": 36, "enter": 36, "tab": 48, "space": 49, "delete": 51, "backspace": 51, "escape": 53, "esc": 53,
	"left": 123, "right": 124, "down": 125, "up": 126, "home": 115, "end": 119, "pageup": 116, "pagedown": 121,
]

func flags(_ s: String?) -> CGEventFlags {
	var f = CGEventFlags()
	for m in (s ?? "").lowercased().split(separator: ",") {
		switch m {
		case "cmd", "command": f.insert(.maskCommand)
		case "shift": f.insert(.maskShift)
		case "alt", "option": f.insert(.maskAlternate)
		case "ctrl", "control": f.insert(.maskControl)
		default: fail("unknown modifier \(m)")
		}
	}
	return f
}

/// Post and stamp. The stamp is taken after post returns: the event is in the HID stream by then.
func post(_ e: CGEvent?) -> UInt64 {
	guard let e else { fail("could not create CGEvent") }
	e.post(tap: .cghidEventTap)
	return now_ns()
}

func mouse(_ type: CGEventType, _ x: Double, _ y: Double) -> UInt64 {
	let e = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left)
	if type == .leftMouseDown || type == .leftMouseUp { e?.setIntegerValueField(.mouseEventClickState, value: 1) }
	return post(e)
}

/// Down then up. Modifier keys get their own down/up so apps that track
/// modifier state (winit does, through flagsChanged) see them held.
func key(_ code: CGKeyCode, _ f: CGEventFlags, hold: UInt32 = 8_000) -> (all: [UInt64], down: UInt64, up: UInt64) {
	var t: [UInt64] = []
	var stamps: [UInt64] = []
	let mods: [(CGEventFlags, CGKeyCode)] = [(.maskCommand, 55), (.maskShift, 56), (.maskAlternate, 58), (.maskControl, 59)]
	var held = CGEventFlags()
	for (m, c) in mods where f.contains(m) {
		held.insert(m)
		let e = CGEvent(keyboardEventSource: source, virtualKey: c, keyDown: true)
		e?.flags = held
		t.append(post(e))
	}
	for down in [true, false] {
		let e = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)
		e?.flags = f
		let at = post(e)
		t.append(at)
		stamps.append(at)
		// Held briefly, as a finger does; a down and up 20 µs apart are not always both delivered.
		if down { usleep(hold) }
	}
	for (m, c) in mods.reversed() where f.contains(m) {
		held.remove(m)
		let e = CGEvent(keyboardEventSource: source, virtualKey: c, keyDown: false)
		e?.flags = held
		t.append(post(e))
	}
	return (t, stamps[0], stamps[1])
}

func keycode(_ s: String) -> CGKeyCode {
	if let n = named[s.lowercased()] { return n }
	if s.count == 1, let c = keycodes[Character(s.lowercased())] { return c }
	if let n = UInt16(s) { return n }
	fail("unknown key \(s)")
}

/// Sleep until an absolute mach deadline: fixed-rate sequences do not drift.
/// Waits most of the way in the kernel, then spins the last 2 ms: mach_wait_until
/// alone wakes up to ~4 ms late for a process the scheduler sees as background.
func sleep_until(_ ns: UInt64) {
	if ns > now_ns() + 2_000_000 {
		mach_wait_until((ns - 2_000_000) * UInt64(timebase.denom) / UInt64(timebase.numer))
	}
	while now_ns() < ns {}
}

/// `ok` is asked before every event a command posts; when it says no, the
/// command stops and reports what it posted so far.
struct Refused: Error { let posted: [UInt64]; let at: Int }

func input(_ args: [String], ok: (Int) -> Bool = { _ in true }) throws -> [String: Any] {
	let hold_ms = args.first { $0.hasPrefix("hold=") }.flatMap { Double($0.dropFirst(5)) }
	let a = args.filter { !$0.hasPrefix("hold=") }
	func d(_ i: Int) -> Double {
		guard i < a.count, let v = Double(a[i]) else { fail("\(a.first ?? ""): argument \(i) missing or not a number") }
		return v
	}
	switch a.first ?? "" {
	case "clock":
		return ["t": [now_ns()]]
	case "move":
		if !ok(0) { throw Refused(posted: [], at: 0) }
		return ["t": [mouse(.mouseMoved, d(1), d(2))]]
	case "down":
		if !ok(0) { throw Refused(posted: [], at: 0) }
		return ["t": [mouse(.leftMouseDown, d(1), d(2))]]
	case "up":
		if !ok(0) { throw Refused(posted: [], at: 0) }
		return ["t": [mouse(.leftMouseUp, d(1), d(2))]]
	case "click":
		// A move first: apps see hover before the press, as with a real mouse.
		if !ok(0) { throw Refused(posted: [], at: 0) }
		let m = mouse(.mouseMoved, d(1), d(2))
		usleep(16_000)
		let dn = mouse(.leftMouseDown, d(1), d(2))
		usleep(UInt32((hold_ms ?? 30) * 1000))
		let up = mouse(.leftMouseUp, d(1), d(2))
		return ["t": [m, dn, up], "move": m, "down": dn, "up": up]
	case "key":
		guard a.count >= 2 else { fail("key <code|name> [mods]") }
		if !ok(0) { throw Refused(posted: [], at: 0) }
		let k = key(keycode(a[1]), flags(a.count > 2 ? a[2] : nil), hold: UInt32((hold_ms ?? 8) * 1000))
		return ["t": k.all, "down": k.down, "up": k.up]
	case "type":
		guard a.count >= 2 else { fail("type <text> [interval_ms]") }
		// `type` from serve gets the text with spaces percent-encoded as %20.
		let text = a[1].removingPercentEncoding ?? a[1]
		let interval = UInt64((a.count > 2 ? Double(a[2]) ?? 0 : 0) * 1e6)
		var t: [UInt64] = [], ups: [UInt64] = []
		var at = now_ns()
		for (i, ch) in text.enumerated() {
			var f = CGEventFlags()
			var base = Character(ch.lowercased())
			if ch.isUppercase { f.insert(.maskShift) }
			if let s = shifted[ch] { base = s; f.insert(.maskShift) }
			guard let code = keycodes[base] else { fail("cannot type \(ch)") }
			if interval > 0 { sleep_until(at) }
			if !ok(i) { throw Refused(posted: t, at: i) }
			let k = key(code, f)
			t.append(k.down)
			ups.append(k.up)
			at += interval
		}
		return ["t": t, "down": t, "up": ups]
	case "wheel":
		let (x, y, dy, count, hz) = (d(1), d(2), Int32(d(3)), Int(d(4)), d(5))
		let step = UInt64(1e9 / hz)
		if !ok(0) { throw Refused(posted: [], at: 0) }
		var t: [UInt64] = [mouse(.mouseMoved, x, y)]
		var at = t[0] + step
		for i in 0..<count {
			sleep_until(at)
			// The full check costs a window-list copy: every 15th event (4 a second at 60 Hz).
			if i % 15 == 14 && !ok(i) { throw Refused(posted: t, at: i) }
			t.append(post(CGEvent(scrollWheelEvent2Source: source, units: .pixel, wheelCount: 1, wheel1: dy, wheel2: 0, wheel3: 0)))
			at += step
		}
		return ["t": t]
	default:
		fail("unknown command \(a.first ?? "")")
	}
}

// MARK: windows

func windows(_ pid: pid_t) -> [[String: Any]] {
	let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
	return list.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }.map { w in
		let b = w[kCGWindowBounds as String] as? [String: Double] ?? [:]
		return [
			"id": w[kCGWindowNumber as String] as? Int ?? 0,
			"x": b["X"] ?? 0, "y": b["Y"] ?? 0, "w": b["Width"] ?? 0, "h": b["Height"] ?? 0,
			"name": w[kCGWindowName as String] as? String ?? "",
			"onscreen": w[kCGWindowIsOnscreen as String] as? Bool ?? false,
			"alpha": w[kCGWindowAlpha as String] as? Double ?? 0,
		]
	}
}

func ax_get<T>(_ el: AXUIElement, _ attr: String) -> T? {
	var v: CFTypeRef?
	guard AXUIElementCopyAttributeValue(el, attr as CFString, &v) == .success else { return nil }
	return v as? T
}
func ax_frame(_ el: AXUIElement) -> [String: Double]? {
	guard let p: AXValue = ax_get(el, kAXPositionAttribute), let s: AXValue = ax_get(el, kAXSizeAttribute) else { return nil }
	var pt = CGPoint.zero, sz = CGSize.zero
	AXValueGetValue(p, .cgPoint, &pt)
	AXValueGetValue(s, .cgSize, &sz)
	return ["x": pt.x, "y": pt.y, "w": sz.width, "h": sz.height]
}
func ax_windows(_ pid: pid_t) -> [AXUIElement] {
	ax_get(AXUIElementCreateApplication(pid), kAXWindowsAttribute) ?? []
}

// MARK: processes

typealias ResponsibleFn = @convention(c) (pid_t) -> pid_t
let responsible: ResponsibleFn? = {
	guard let h = dlopen(nil, RTLD_NOW), let s = dlsym(h, "responsibility_get_pid_responsible_for_pid") else { return nil }
	return unsafeBitCast(s, to: ResponsibleFn.self)
}()

func all_pids() -> [pid_t] {
	let n = proc_listallpids(nil, 0)
	var pids = [pid_t](repeating: 0, count: Int(n) * 2)
	let got = proc_listallpids(&pids, Int32(pids.count * MemoryLayout<pid_t>.size))
	return Array(pids.prefix(Int(got))).filter { $0 > 0 }
}
func ppid(_ pid: pid_t) -> pid_t {
	var info = proc_bsdinfo()
	let n = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, Int32(MemoryLayout<proc_bsdinfo>.size))
	return n > 0 ? pid_t(info.pbi_ppid) : -1
}
func pname(_ pid: pid_t) -> String {
	var buf = [CChar](repeating: 0, count: 4096)
	return proc_pidpath(pid, &buf, UInt32(buf.count)) > 0 ? String(cString: buf) : ""
}

/// The app and everything it owns: children (recursively), and XPC services
/// launchd started for it, which name it as their responsible process (WebKit's helpers).
func procs(_ root: pid_t) -> [[String: Any]] {
	var owned: Set<pid_t> = [root]
	let pids = all_pids()
	let parents = Dictionary(uniqueKeysWithValues: pids.map { ($0, ppid($0)) })
	var changed = true
	while changed {
		changed = false
		for p in pids where !owned.contains(p) {
			if let pp = parents[p], owned.contains(pp) { owned.insert(p); changed = true }
			else if let r = responsible?(p), r != p, owned.contains(r) { owned.insert(p); changed = true }
		}
	}
	return owned.sorted().map { p in
		["pid": p, "ppid": parents[p] ?? -1, "responsible": responsible?(p) ?? -1, "path": pname(p)]
	}
}

// MARK: capture

final class Recorder: NSObject, SCStreamOutput, SCStreamDelegate {
	let fh: FileHandle
	var prev: [UInt8] = []
	var frames = 0, complete = 0
	init(_ fh: FileHandle) { self.fh = fh }

	func stream(_ stream: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType) {
		let arrival = now_ns()
		guard type == .screen,
			let atts = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
			let info = atts.first,
			let raw = info[.status] as? Int, let status = SCFrameStatus(rawValue: raw)
		else { return }
		frames += 1
		let display = (info[.displayTime] as? UInt64).map(ticks_to_ns) ?? 0
		var mad = -1.0
		var hash: UInt64 = 0
		if status == .complete, let px = CMSampleBufferGetImageBuffer(sb) {
			complete += 1
			CVPixelBufferLockBaseAddress(px, .readOnly)
			let w = CVPixelBufferGetWidth(px), h = CVPixelBufferGetHeight(px), row = CVPixelBufferGetBytesPerRow(px)
			let base = CVPixelBufferGetBaseAddress(px)!.assumingMemoryBound(to: UInt8.self)
			// Luma-ish: one byte per pixel (the green channel) keeps the diff cheap.
			var cur = [UInt8](repeating: 0, count: w * h)
			hash = 14695981039346656037
			for y in 0..<h {
				let r = base + y * row
				for x in 0..<w {
					let g = r[x * 4 + 1]
					cur[y * w + x] = g
					hash = (hash ^ UInt64(r[x * 4]) ^ (UInt64(g) << 8) ^ (UInt64(r[x * 4 + 2]) << 16)) &* 1099511628211
				}
			}
			CVPixelBufferUnlockBaseAddress(px, .readOnly)
			if prev.count == cur.count {
				var sum = 0
				for i in 0..<cur.count { sum += abs(Int(cur[i]) - Int(prev[i])) }
				mad = Double(sum) / Double(cur.count)
			} else {
				mad = 255 // first frame, or the window changed size
			}
			prev = cur
		}
		let names: [SCFrameStatus: String] = [.complete: "complete", .idle: "idle", .blank: "blank", .suspended: "suspended", .started: "started", .stopped: "stopped"]
		fh.write("\(display) \(arrival) \(names[status] ?? "\(raw)") \(String(format: "%.4f", mad)) \(String(hash, radix: 16))\n".data(using: .utf8)!)
	}

	func stream(_ stream: SCStream, didStopWithError error: Error) { fail("capture stopped: \(error)") }
}

func record(_ wid: Int, _ seconds: Double, _ path: String, _ scale: Double) async {
	guard CGPreflightScreenCaptureAccess() else { fail("no Screen Recording permission") }
	do {
		let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
		guard let win = content.windows.first(where: { $0.windowID == CGWindowID(wid) }) else { fail("window \(wid) not shareable/on screen") }
		let display = CGDisplayCopyDisplayMode(CGMainDisplayID())
		let hz = (display?.refreshRate ?? 0) > 0 ? display!.refreshRate : 60
		let cfg = SCStreamConfiguration()
		// Points * scale: the GPU downscales, so the per-frame work here stays small.
		cfg.width = max(16, Int(win.frame.width * scale))
		cfg.height = max(16, Int(win.frame.height * scale))
		cfg.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(hz.rounded()))
		cfg.queueDepth = 6
		cfg.pixelFormat = kCVPixelFormatType_32BGRA
		cfg.showsCursor = false
		FileManager.default.createFile(atPath: path, contents: nil)
		guard let fh = FileHandle(forWritingAtPath: path) else { fail("cannot write \(path)") }
		fh.write("# display_ns arrival_ns status mad hash  window=\(wid) hz=\(hz) size=\(cfg.width)x\(cfg.height)\n".data(using: .utf8)!)
		let rec = Recorder(fh)
		let stream = SCStream(filter: SCContentFilter(desktopIndependentWindow: win), configuration: cfg, delegate: rec)
		try stream.addStreamOutput(rec, type: .screen, sampleHandlerQueue: DispatchQueue(label: "capture", qos: .userInteractive))
		try await stream.startCapture()
		let started = now_ns()
		out(["started": started, "hz": hz, "width": cfg.width, "height": cfg.height])
		try await Task.sleep(nanoseconds: UInt64(seconds * 1e9))
		try await stream.stopCapture()
		try fh.close()
		out(["stopped": now_ns(), "frames": rec.frames, "complete": rec.complete])
	} catch {
		fail("capture: \(error)")
	}
}

// MARK: frontmost

/// Both have to agree: the app AppKit calls frontmost (fresh only while the
/// main run loop turns, which `serve` keeps doing between commands) and the
/// owner of the topmost normal window on screen (fresh always).
func front_state(_ pid: pid_t) -> [String: Any] {
	let front = NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1
	let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
	let top = list.first { ($0[kCGWindowLayer as String] as? Int) == 0 }
	let top_pid = top?[kCGWindowOwnerPID as String] as? Int32 ?? -1
	let mine = windows(pid)
	return [
		"front_pid": front, "top_window_pid": top_pid, "frontmost": front == pid && top_pid == pid,
		"onscreen": mine.contains { $0["onscreen"] as? Bool == true }, "windows": mine,
	]
}
func is_front(_ pid: pid_t) -> Bool { front_state(pid)["frontmost"] as? Bool ?? false }

/// Will a key press reach the app's window? AX calls are answered by the app's main thread,
/// so they wait while it is busy: 50 ms at most, and a timeout is reported as such.
func focus_state(_ pid: pid_t, ax: Bool) -> [String: Any] {
	let t = now_ns()
	var out: [String: Any] = ["t": t, "active": NSRunningApplication(processIdentifier: pid)?.isActive ?? false, "frontmost": is_front(pid)]
	if !ax {
		out["ms"] = Double(now_ns() - t) / 1e6
		return out
	}
	let app = AXUIElementCreateApplication(pid)
	AXUIElementSetMessagingTimeout(app, 0.05)
	var v: CFTypeRef?
	let err = AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute as CFString, &v)
	out["ax_error"] = err.rawValue
	if err == .success, let v {
		let win = v as! AXUIElement
		AXUIElementSetMessagingTimeout(win, 0.05)
		out["ax_focused_window"] = true
		out["ax_main"] = (ax_get(win, kAXMainAttribute) as Bool?) ?? NSNull()
	} else {
		// kAXErrorNoValue (-25212): no focused window; kAXErrorCannotComplete (-25204): timeout or no answer.
		out["ax_focused_window"] = false
		out["ax_main"] = NSNull()
	}
	out["ms"] = Double(now_ns() - t) / 1e6
	return out
}

// MARK: usage

/// Exact totals from the kernel, per process: CPU time, wake-ups, energy.
///
/// Energy: `ri_energy_nj` (V6) is the kernel's estimate of the energy this process's own
/// threads used, in nJ (`ri_penergy_nj`: the part on P-cores). `ri_billed_energy` is NOT
/// that: it is energy other processes spent on this one's behalf and billed to it through
/// vouchers (XNU bank ledger, `bank_billed_balance`), and `ri_serviced_energy` the reverse.
func usage(_ pid: pid_t) -> [String: Any] {
	var ri = rusage_info_v6()
	let r = withUnsafeMutablePointer(to: &ri) {
		$0.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) { proc_pid_rusage(pid, RUSAGE_INFO_V6, $0) }
	}
	if r != 0 { return ["pid": pid, "error": String(cString: strerror(errno))] }
	// User and system time are mach ticks on Apple silicon, not ns.
	return [
		"pid": pid, "t": now_ns(),
		"user_ns": ticks_to_ns(ri.ri_user_time), "system_ns": ticks_to_ns(ri.ri_system_time),
		"pkg_idle_wkups": ri.ri_pkg_idle_wkups, "interrupt_wkups": ri.ri_interrupt_wkups,
		"phys_footprint": ri.ri_phys_footprint, "lifetime_max_phys_footprint": ri.ri_lifetime_max_phys_footprint,
		"energy_nj": ri.ri_energy_nj, "penergy_nj": ri.ri_penergy_nj,
		"billed_energy": ri.ri_billed_energy, "serviced_energy": ri.ri_serviced_energy,
		"instructions": ri.ri_instructions, "cycles": ri.ri_cycles,
		"diskio_read": ri.ri_diskio_bytesread, "diskio_written": ri.ri_diskio_byteswritten,
	]
}

// MARK: commands

func pid_at(_ a: [String], _ i: Int) -> pid_t {
	guard i < a.count, let p = Int32(a[i]) else { fail("\(a.first ?? ""): pid missing") }
	return p
}

/// Everything but `perm`, `record` and `serve`, for the command line and for `serve` alike.
func command(_ a: [String]) -> Any {
	switch a.first ?? "" {
	case "machine":
		var displays: [[String: Any]] = []
		var ids = [CGDirectDisplayID](repeating: 0, count: 16)
		var n: UInt32 = 0
		CGGetActiveDisplayList(16, &ids, &n)
		for id in ids.prefix(Int(n)) {
			guard let m = CGDisplayCopyDisplayMode(id) else { continue }
			displays.append(["id": id, "main": id == CGMainDisplayID(), "points": [m.width, m.height], "pixels": [m.pixelWidth, m.pixelHeight], "refresh_hz": m.refreshRate])
		}
		// What a standard titled window loses to its title bar on this macOS.
		let frame = NSRect(x: 0, y: 0, width: 1200, height: 800)
		let content = NSWindow.contentRect(forFrameRect: frame, styleMask: [.titled, .closable, .miniaturizable, .resizable])
		return ["displays": displays, "os": ProcessInfo.processInfo.operatingSystemVersionString, "titlebar": frame.height - content.height,
			"timebase": [timebase.numer, timebase.denom]]
	case "windows":
		return windows(pid_at(a, 1))
	case "ax":
		return ax_windows(pid_at(a, 1)).map { w -> [String: Any] in
			let kids: [AXUIElement] = ax_get(w, kAXChildrenAttribute) ?? []
			return [
				"frame": ax_frame(w) ?? [:], "title": (ax_get(w, kAXTitleAttribute) as String?) ?? "",
				"children": kids.map { ["role": (ax_get($0, kAXRoleAttribute) as String?) ?? "", "subrole": (ax_get($0, kAXSubroleAttribute) as String?) ?? "", "frame": ax_frame($0) ?? [:]] },
			]
		}
	case "front":
		return front_state(pid_at(a, 1))
	case "focus":
		return focus_state(pid_at(a, 1), ax: a.count > 2 && a[2] == "ax")
	case "activate":
		let pid = pid_at(a, 1)
		guard let app = NSRunningApplication(processIdentifier: pid) else { return ["error": "no app with pid \(pid)"] }
		let ok = app.activate(options: [.activateAllWindows])
		RunLoop.main.run(until: Date().addingTimeInterval(0.2))
		return ["requested": ok, "frontmost": is_front(pid)]
	case "resize":
		let pid = pid_at(a, 1)
		guard a.count >= 4, let w = Double(a[2]), let h = Double(a[3]) else { fail("resize <pid> <w> <h>") }
		// The largest: while the window is screen-captured, macOS adds a 66x20 AX window to
		// the app, listed first, which refuses resizing (-25200).
		let area = { (w: AXUIElement) -> Double in let f = ax_frame(w); return ((f?["w"] as? Double) ?? 0) * ((f?["h"] as? Double) ?? 0) }
		guard let win = ax_windows(pid).max(by: { area($0) < area($1) }) else { return ["error": "no AX window for \(pid)"] }
		var size = CGSize(width: w, height: h)
		let value = AXValueCreate(.cgSize, &size)!
		let t = now_ns()
		let err = AXUIElementSetAttributeValue(win, kAXSizeAttribute as CFString, value)
		let done = now_ns()
		return ["t": [t], "returned": done, "error": err.rawValue, "frame": ax_frame(win) ?? [:]]
	case "procs":
		return procs(pid_at(a, 1))
	case "ps":
		return all_pids().map { p in ["pid": p, "ppid": ppid(p), "responsible": responsible?(p) ?? -1, "path": pname(p)] }
	case "usage":
		return a.dropFirst().compactMap { Int32($0) }.map(usage)
	case "as":
		// `as <pid> <input command>`: post only while pid is frontmost.
		let pid = pid_at(a, 1)
		do {
			return try input(Array(a.dropFirst(2)), ok: { _ in is_front(pid) })
		} catch let r as Refused {
			return ["refused": "pid \(pid) is not frontmost", "at": r.at, "t": r.posted, "front": front_state(pid)]
		} catch { fail("\(error)") }
	default:
		guard AXIsProcessTrusted() else { fail("no Accessibility permission: CGEvents would be dropped silently") }
		return try! input(a)
	}
}

// MARK: main

let args = Array(CommandLine.arguments.dropFirst())
pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0)

switch args.first ?? "" {
case "perm":
	let ax = AXIsProcessTrusted(), screen = CGPreflightScreenCaptureAccess()
	out(["ax": ax, "screen": screen])
	exit(ax && screen ? 0 : 1)
case "record":
	guard args.count >= 4, let wid = Int(args[1]), let secs = Double(args[2]) else { fail("record <windowid> <seconds> <out> [scale]") }
	let scale = args.count > 4 ? Double(args[4]) ?? 0.25 : 0.25
	// SIGTERM ends the recording early, with the file complete up to the last frame.
	signal(SIGTERM) { _ in exit(0) }
	Task { await record(wid, secs, args[3], scale); exit(0) }
	RunLoop.main.run()
case "launch":
	guard args.count >= 2 else { fail("launch <exe> [args...]") }
	typealias Disclaim = @convention(c) (UnsafeMutablePointer<posix_spawnattr_t?>, Int32) -> Int32
	guard let sym = dlsym(dlopen(nil, RTLD_NOW), "responsibility_spawnattrs_setdisclaim") else { fail("no responsibility_spawnattrs_setdisclaim") }
	var attr: posix_spawnattr_t? = nil
	posix_spawnattr_init(&attr)
	guard unsafeBitCast(sym, to: Disclaim.self)(&attr, 1) == 0 else { fail("disclaim failed") }
	posix_spawnattr_setflags(&attr, Int16(POSIX_SPAWN_SETEXEC))
	let argv = Array(args.dropFirst())
	var cargs = argv.map { strdup($0) } + [nil]
	var pid: pid_t = 0
	let r = posix_spawn(&pid, argv[0], nil, &attr, &cargs, environ)
	fail("exec \(argv[0]): \(String(cString: strerror(r)))")
case "serve":
	guard AXIsProcessTrusted() else { fail("no Accessibility permission: CGEvents would be dropped silently") }
	// Commands run on the main thread; between them the run loop turns, which
	// keeps NSWorkspace's idea of the frontmost app current.
	Thread.detachNewThread {
		while let line = readLine() {
			let a = line.split(separator: " ").map(String.init)
			if a.isEmpty { continue }
			if a[0] == "quit" { exit(0) }
			DispatchQueue.main.sync { out(command(a)) }
		}
		exit(0)
	}
	out(["ready": now_ns()])
	RunLoop.main.run()
case "":
	fail("usage: see the top of drive.swift")
default:
	out(command(args))
	// Events still in flight when the process exits can be lost (seen with keys).
	usleep(50_000)
}
