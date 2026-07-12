# EPM1 MIDI2 v0.1 — ALSA UMP Spike Evidence (SET4-1)

Date: 2026-07-06

Slice: `SET4-1` of `docs/EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md` — ALSA
UMP feasibility spike: the Rust path to UMP endpoints.

Evidence class: **synthetic development evidence**. All runs are
laptop-local against the ALSA sequencer with virtual clients; no hardware
UMP device exists in the rig. The spike binary is throwaway by design
(scratch, not a workspace member, not committed); its full source is
preserved in the appendix so every run here is reproducible.

## Environment

- Host: Intel Core i7-4600U @ 2.10 GHz (4 threads), 8 GiB RAM
- Kernel: Linux 7.0.14-201.fc44.x86_64 (Fedora); `snd_seq`, `snd_ump`,
  `snd_seq_dummy` modules loaded; `/dev/snd/seq` present and accessible
- ALSA: alsa-lib 1.2.16.1 (`/usr/include/alsa/ump.h`, `ump_msg.h` present);
  alsa-utils 1.2.16 (`aseqdump` supports `-u/--ump=<0|1|2>`)
- Toolchain: rustc 1.96.0 (stable), cargo 1.96.0
- Workspace crates under test for the gap claim: `alsa-sys 0.3.1`,
  `alsa 0.9.1`, `midir 0.10.3` (all as pinned in `Cargo.lock`)

## The gap, re-verified on this host

The backlog's assessment holds exactly as written:

- `alsa-sys 0.3.1` (registry sources): **zero** matches for
  `snd_ump`/`snd_seq_ump`/`set_client_midi_version` in `src/lib.rs`.
- `alsa 0.9.1`: no sequencer UMP surface; its only `ump` hits are unrelated
  (`pcm.rs` docs, ioctl constants).
- `midir 0.10.3`: MIDI 1.0 byte-stream only.
- alsa-lib headers on the same host carry the full needed surface:
  `snd_seq_set_client_midi_version`, `SND_SEQ_CLIENT_UMP_MIDI_2_0`,
  `snd_seq_ump_event_t`, `snd_seq_ump_event_input/output`,
  `snd_seq_create_ump_endpoint`, `snd_ump_endpoint_info_*`.

So the host stack is ready and the Rust binding layer is the only missing
piece — confirming the backlog's "largest external risk" framing.

## What was verified

### 1. Native UMP MIDI 2.0 client negotiation, checked from Rust

The spike opens a sequencer client, calls
`snd_seq_set_client_midi_version(SND_SEQ_CLIENT_UMP_MIDI_2_0)`, then reads
the mode back through `snd_seq_get_client_info` +
`snd_seq_client_info_get_midi_version` — the *negotiated* value, not the
requested one:

```
$ ump-spike mode
client 128 negotiated midi_version=2 (0=legacy,1=UMP1.0,2=UMP2.0)
```

This read-back is the evidence machinery the backlog demands: any future
UMP ingress evidence must print the negotiated client mode, or a silently
legacy client can pass every test at 7-bit resolution (see §4).

### 2. Virtual UMP endpoint visible to ALSA tooling

`snd_seq_create_ump_endpoint` (protocol caps and protocol both
`SND_UMP_EP_INFO_PROTO_MIDI2`, 1 group) creates an endpoint that ALSA
tooling identifies as a UMP MIDI 2.0 client, with the endpoint port and a
group port:

```
$ aconnect -l
client 128: 'ump-spike-recv' [type=user,UMP-MIDI2,pid=27340]
    0 'MIDI 2.0        '
    1 'Group 1         '
    2 'in              '
```

The same call in the sender direction (`ump-spike send`) is the virtual
UMP **output** client shape `mamut-seq` needs in `SET4-7`, captured by
`aconnect -l` while a send was in flight:

```
client 129: 'ump-spike-send' [type=user,UMP-MIDI2,pid=29907]
    0 'MIDI 2.0        '
    1 'Group 1         '
```

`aseqdump` also received from it as an ordinary destination (§4) — the
"visible to ALSA tooling" acceptance line satisfied in both directions.

### 3. MIDI 2.0 NoteOn with 16-bit velocity, decoded in Rust end to end

Sender (client 129) → receiver (client 128), both native UMP MIDI 2.0
clients. The packet is a MIDI 2.0 Channel Voice NoteOn, note 60, velocity
`0xABCD`, attribute type 3 (Pitch 7.9) with value `0x7900` (= 60.5
semitones):

```
$ ump-spike send 128 2
sent NoteOn  [40903C03, ABCD7900, 00000000, 00000000]
sent NoteOff [40803C00, 40000000, 00000000, 00000000]

# receiver output:
recv ready: client=128 port=2 midi_version=2
MIDI2 NoteOn group=0 ch=0 note=60 velocity16=43981 (0xABCD) attr_type=3 attr=0x7900
  attribute Pitch7.9 -> 60.5 semitones
MIDI2 NoteOff group=0 ch=0 note=60 velocity16=16384 (0x4000) attr_type=0 attr=0x0000
```

16-bit note-on velocity, the note attribute, and 16-bit release velocity
all survive the sequencer round-trip bit-exactly and decode correctly on
the Rust side. This satisfies the primary SET4-1 acceptance line (the UMP
source here is the spike's own virtual UMP output client rather than
`aseqdump`, which only dumps; `aseqdump -u 2` served as the independent
*receiver* cross-check below).

### 4. The auto-conversion trap, demonstrated

The same two packets sent to two `aseqdump` instances that differ only in
client MIDI version (a separate invocation from §3, so the sender's client
id is 130 here rather than 129):

```
=== aseqdump -u 0 (legacy MIDI 1.0 client) ===
130:0   Note on                 0, note 60, velocity 85
130:0   Note off                0, note 60, velocity 32

=== aseqdump -u 2 (native UMP MIDI 2.0 client) ===
130:0   Group  0, Note on       0, note 60, velocity 0xabcd, attr type = 3, data = 0x7900
130:0   Group  0, Note off      0, note 60, velocity 0x4000, attr type = 0, data = 0x0
```

The legacy client receives velocity 85 (= `0xABCD >> 9`) and the attribute
vanishes — with **no error anywhere**. This is exactly the trap the
backlog warned about: a legacy-registered client "works" while silently
running at 7-bit resolution. Consequence, now a standing evidence rule for
`SET4-6`/`SET4-7`: **every UMP evidence run must record the negotiated
client MIDI version** (the §1 read-back), and the runtime must verify the
mode at open and refuse to claim UMP otherwise.

The `-u 2` output doubles as an independent cross-check of the spike's
encoder: an unmodified ALSA tool decodes our emitted words to precisely
the intended fields.

### 5. Spike-scope observation: group ports refuse direct addressed writes

A directly-addressed send to the UMP endpoint's group port (`128:1`)
failed with `EPERM`:

```
send: client=129 midi_version=2 -> dest 128:1
error: drain_output: Operation not permitted (-1)
```

The same packet to a plain `CAP_WRITE|SUBS_WRITE` port on the same UMP
client (`128:2`) was delivered as native UMP. The group ports created by `snd_seq_create_ump_endpoint`
appear intended for the subscription model rather than direct addressing.
Not a blocker (both the subscription model and plain ports on a UMP client
work); flagged as a design input for the `SET4-6` ingress backend, which
should use subscriptions (as `midir` does today) rather than direct
addressing.

## Decisions

### FFI route: **B — dedicated minimal sys crate + thin safe wrapper** (both non-workspace-members)

- **(B) chosen.** The spike proves the needed surface is tiny and stable
  (20 symbols + 1 struct layout; inventory below). A purpose-built
  `-sys` crate binding exactly that, wrapped by a small safe API crate,
  quarantines the unsafe surface outside the workspace per the Shared
  Boundary Constraints, is auditable in one sitting, and carries no fork
  of a large third-party crate.
- **(A) extend `alsa-rs`** — not taken now. A vendored fork of `alsa`
  0.9.1 would drag the whole crate's maintenance surface for ~20 symbols.
  **Upstreaming the seq-UMP surface to `alsa-rs` remains the preferred
  long-term exit** and route B's binding is deliberately shaped so it can
  become that PR; if upstream lands first, route B collapses into a
  version bump.
- **(C) rawmidi UMP endpoints** — not needed. The sequencer route worked
  on the first attempt end to end; C stays documented as fallback only.

### Message model: **own, in `mamut-midi2`** (backlog default confirmed)

The external `midi2` crate was not adopted. The v1 surface `SET4-2` needs
(UMP framing, CVM1/CVM2, utility/JR, parse-and-ignore verdicts) is small,
table-testable, and must match the engine's event vocabulary exactly;
spec-derived vectors give equal confidence without a dependency. This
matches the zero-dep leaf philosophy of `mamut-dsp`/`mamut-params`. The
crate did not earn its place; revisit only if `SET4-2` uncovers spec
complexity materially beyond the v1 message set.

### Unsafe-surface inventory (route B, as proven by the spike)

Functions (all `libasound`, all present in alsa-lib 1.2.16 headers):

```
snd_seq_open, snd_seq_close, snd_seq_set_client_name, snd_seq_client_id
snd_seq_set_client_midi_version
snd_seq_client_info_malloc, snd_seq_client_info_free
snd_seq_get_client_info, snd_seq_client_info_get_midi_version
snd_ump_endpoint_info_malloc, snd_ump_endpoint_info_free
snd_ump_endpoint_info_set_name, snd_ump_endpoint_info_set_protocol_caps
snd_ump_endpoint_info_set_protocol
snd_seq_create_ump_endpoint, snd_seq_create_simple_port
snd_seq_ump_event_input, snd_seq_ump_event_output, snd_seq_drain_output
snd_strerror
```

One non-opaque struct: `snd_seq_ump_event_t` — layout verified on this
host with a C `sizeof`/`offsetof` probe (size 32; `time` @4, `source` @12,
`dest` @14, `ump[4]` @16) and mirrored as a `#[repr(C)]` Rust struct. All
other types stay opaque behind pointers.

Known additions the production binding will need beyond the spike
(bounded, same character): poll-descriptor integration
(`snd_seq_poll_descriptors*`) for the ingress thread, port subscription
(`snd_seq_subscribe_port` family) per §5, port/client enumeration for
device listing, and `snd_seq_set_client_ump_conversion` if mixed-mode
delivery needs pinning. Estimated total remains ≈30 symbols.

### Kernel matrix

- Dev host: kernel 7.0.14 — far above the ≥ 6.5 ALSA seq UMP floor; verified
  live by every run above.
- **RPi3B headless path: open rig check.** `EPM1_RPI3B_MIOXM_HEADLESS.md`
  does not record a kernel version. Current Raspberry Pi OS (Bookworm)
  kernels are ≥ 6.6 and should satisfy the floor, but per lab discipline
  this is **checked on the actual rig, not assumed**: record `uname -r`,
  presence of `/sys/module/snd_ump`, and the image's alsa-lib version
  (needs ≥ 1.2.10 for the seq UMP client API) in that runbook before any
  UMP claim on the Pi path.

## Boundary compliance

- No workspace change of any kind: no new members, no `Cargo.lock` drift,
  no lint-posture change. The spike lives outside the repo and links
  `libasound` directly with its own hand-written FFI (`unsafe` exists only
  there, which is exactly the quarantine posture the backlog mandates for
  production code too).
- No runtime, engine, or transport code touched. ADR 0005 governance and
  ADR 0001 realtime rules unaffected by this slice.

## Acceptance mapping (SET4-1)

- *Spike binary receives a MIDI 2.0 NoteOn with 16-bit velocity from a UMP
  source and prints decoded fields* — §3 (spike-to-spike, both native UMP
  clients; `aseqdump -u 2` as independent decode cross-check).
- *A virtual UMP output client is created and visible to ALSA tooling* —
  §2 (`aconnect -l` shows `UMP-MIDI2` client with endpoint + group ports;
  `aseqdump` receives from the sender's endpoint).
- *FFI route decision and message-model decision recorded with the
  unsafe-surface inventory* — Decisions section above.
- Backlog spike extras: auto-conversion trap verified and converted into a
  standing evidence rule (§4); virtual UMP output client proven for
  `SET4-7` (§2); kernel matrix recorded with the RPi3B check left
  explicitly open (Decisions).

## Appendix: spike source (throwaway, reproducible)

`Cargo.toml`:

```toml
# SET4-1 ALSA UMP feasibility spike — throwaway, NOT a mamut-sint-sw workspace
# member. Links libasound directly with a narrow hand-written FFI surface.
[package]
name = "ump-spike"
version = "0.1.0"
edition = "2021"

[workspace]
```

`build.rs`:

```rust
fn main() {
    println!("cargo:rustc-link-lib=dylib=asound");
}
```

`src/main.rs`:

```rust
//! SET4-1 ALSA UMP feasibility spike (throwaway).
//!
//! Proves, from Rust, against alsa-lib 1.2.16 / kernel >= 6.5 sequencer UMP:
//!   1. a native UMP MIDI 2.0 sequencer client can be created and its
//!      negotiated client MIDI version verified (guards the silent
//!      legacy-downconversion trap);
//!   2. a virtual UMP endpoint (with group ports) is visible to ALSA tooling;
//!   3. a MIDI 2.0 NoteOn with 16-bit velocity + attribute survives
//!      end-to-end and decodes correctly on the Rust side.
//!
//! Usage:
//!   ump-spike recv                 # UMP MIDI 2.0 endpoint, decode inbound CVM2
//!   ump-spike send <client> <port> # send MIDI 2.0 NoteOn/NoteOff to dest
//!   ump-spike mode                 # print negotiated client midi version

use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::process::ExitCode;

// ---- opaque handles ----
#[repr(C)]
struct SndSeq {
    _p: [u8; 0],
}
#[repr(C)]
struct SndSeqClientInfo {
    _p: [u8; 0],
}
#[repr(C)]
struct SndUmpEndpointInfo {
    _p: [u8; 0],
}

// ---- snd_seq_ump_event_t, layout verified by C probe on this host:
//      sizeof = 32, time @4, source @12, dest @14, ump @16 ----
#[repr(C)]
#[derive(Clone, Copy)]
struct SndSeqAddr {
    client: u8,
    port: u8,
}
#[repr(C)]
struct SndSeqUmpEvent {
    r#type: u8,
    flags: u8,
    tag: u8,
    queue: u8,
    time: [u32; 2], // snd_seq_timestamp_t union (tick | {sec,nsec})
    source: SndSeqAddr,
    dest: SndSeqAddr,
    ump: [u32; 4], // union with legacy snd_seq_event_data_t
}

const SND_SEQ_OPEN_DUPLEX: c_int = 3;
const SND_SEQ_CLIENT_UMP_MIDI_2_0: c_int = 2;
const SND_SEQ_EVENT_UMP: u8 = 1 << 5;
const SND_SEQ_QUEUE_DIRECT: u8 = 253;
const SND_SEQ_PORT_CAP_WRITE: c_uint = 1 << 1;
const SND_SEQ_PORT_CAP_SUBS_WRITE: c_uint = 1 << 6;
const SND_UMP_EP_INFO_PROTO_MIDI2: c_uint = 0x0200;

#[link(name = "asound")]
extern "C" {
    fn snd_seq_open(h: *mut *mut SndSeq, name: *const c_char, streams: c_int, mode: c_int) -> c_int;
    fn snd_seq_close(h: *mut SndSeq) -> c_int;
    fn snd_seq_set_client_name(h: *mut SndSeq, name: *const c_char) -> c_int;
    fn snd_seq_client_id(h: *mut SndSeq) -> c_int;
    fn snd_seq_set_client_midi_version(h: *mut SndSeq, v: c_int) -> c_int;
    fn snd_seq_client_info_malloc(p: *mut *mut SndSeqClientInfo) -> c_int;
    fn snd_seq_client_info_free(p: *mut SndSeqClientInfo);
    fn snd_seq_get_client_info(h: *mut SndSeq, i: *mut SndSeqClientInfo) -> c_int;
    fn snd_seq_client_info_get_midi_version(i: *const SndSeqClientInfo) -> c_int;
    fn snd_ump_endpoint_info_malloc(p: *mut *mut SndUmpEndpointInfo) -> c_int;
    fn snd_ump_endpoint_info_free(p: *mut SndUmpEndpointInfo);
    fn snd_ump_endpoint_info_set_name(i: *mut SndUmpEndpointInfo, n: *const c_char);
    fn snd_ump_endpoint_info_set_protocol_caps(i: *mut SndUmpEndpointInfo, c: c_uint);
    fn snd_ump_endpoint_info_set_protocol(i: *mut SndUmpEndpointInfo, p: c_uint);
    fn snd_seq_create_ump_endpoint(h: *mut SndSeq, i: *const SndUmpEndpointInfo, groups: c_uint) -> c_int;
    fn snd_seq_create_simple_port(h: *mut SndSeq, n: *const c_char, caps: c_uint, t: c_uint) -> c_int;
    fn snd_seq_ump_event_input(h: *mut SndSeq, ev: *mut *mut SndSeqUmpEvent) -> c_int;
    fn snd_seq_ump_event_output(h: *mut SndSeq, ev: *mut SndSeqUmpEvent) -> c_int;
    fn snd_seq_drain_output(h: *mut SndSeq) -> c_int;
    fn snd_strerror(e: c_int) -> *const c_char;
}

fn err(what: &str, code: c_int) -> String {
    let msg = unsafe { CStr::from_ptr(snd_strerror(code)) };
    format!("{what}: {} ({code})", msg.to_string_lossy())
}

fn check(what: &str, code: c_int) -> Result<c_int, String> {
    if code < 0 {
        Err(err(what, code))
    } else {
        Ok(code)
    }
}

struct Seq(*mut SndSeq);
impl Drop for Seq {
    fn drop(&mut self) {
        unsafe { snd_seq_close(self.0) };
    }
}

fn open_ump_client(name: &str) -> Result<Seq, String> {
    let mut h: *mut SndSeq = std::ptr::null_mut();
    let dev = CString::new("default").map_err(|e| e.to_string())?;
    check("snd_seq_open", unsafe {
        snd_seq_open(&mut h, dev.as_ptr(), SND_SEQ_OPEN_DUPLEX, 0)
    })?;
    let seq = Seq(h);
    let cname = CString::new(name).map_err(|e| e.to_string())?;
    check("set_client_name", unsafe {
        snd_seq_set_client_name(seq.0, cname.as_ptr())
    })?;
    // The load-bearing call: without this the client is legacy MIDI 1.0 and
    // the kernel silently downconverts UMP sources to 7-bit for it.
    check("set_client_midi_version(UMP_MIDI_2_0)", unsafe {
        snd_seq_set_client_midi_version(seq.0, SND_SEQ_CLIENT_UMP_MIDI_2_0)
    })?;
    Ok(seq)
}

/// Read back the *negotiated* client MIDI version from the sequencer — the
/// evidence check that guards against the silent-downconversion trap.
fn negotiated_midi_version(seq: &Seq) -> Result<c_int, String> {
    let mut info: *mut SndSeqClientInfo = std::ptr::null_mut();
    check("client_info_malloc", unsafe {
        snd_seq_client_info_malloc(&mut info)
    })?;
    let r = check("get_client_info", unsafe {
        snd_seq_get_client_info(seq.0, info)
    })
    .map(|_| unsafe { snd_seq_client_info_get_midi_version(info) });
    unsafe { snd_seq_client_info_free(info) };
    r
}

fn create_ump_endpoint(seq: &Seq, name: &str, groups: u32) -> Result<(), String> {
    let mut info: *mut SndUmpEndpointInfo = std::ptr::null_mut();
    check("ump_endpoint_info_malloc", unsafe {
        snd_ump_endpoint_info_malloc(&mut info)
    })?;
    let cname = CString::new(name).map_err(|e| e.to_string())?;
    let r = unsafe {
        snd_ump_endpoint_info_set_name(info, cname.as_ptr());
        snd_ump_endpoint_info_set_protocol_caps(info, SND_UMP_EP_INFO_PROTO_MIDI2);
        snd_ump_endpoint_info_set_protocol(info, SND_UMP_EP_INFO_PROTO_MIDI2);
        check("create_ump_endpoint", snd_seq_create_ump_endpoint(seq.0, info, groups))
    };
    unsafe { snd_ump_endpoint_info_free(info) };
    r.map(|_| ())
}

fn decode_cvm2(w: &[u32; 4]) {
    let mt = w[0] >> 28;
    let group = (w[0] >> 24) & 0xF;
    match mt {
        0x4 => {
            let opcode = (w[0] >> 20) & 0xF;
            let channel = (w[0] >> 16) & 0xF;
            match opcode {
                0x9 | 0x8 => {
                    let note = (w[0] >> 8) & 0x7F;
                    let attr_type = w[0] & 0xFF;
                    let velocity = w[1] >> 16;
                    let attr = w[1] & 0xFFFF;
                    let kind = if opcode == 0x9 { "NoteOn" } else { "NoteOff" };
                    println!(
                        "MIDI2 {kind} group={group} ch={channel} note={note} \
                         velocity16={velocity} (0x{velocity:04X}) attr_type={attr_type} attr=0x{attr:04X}"
                    );
                    if attr_type == 3 {
                        println!(
                            "  attribute Pitch7.9 -> {} semitones",
                            attr as f32 / 512.0
                        );
                    }
                }
                _ => println!(
                    "MIDI2 CVM group={group} ch={channel} opcode=0x{opcode:X} words={w:08X?}"
                ),
            }
        }
        _ => println!("UMP mt=0x{mt:X} group={group} words={w:08X?}"),
    }
}

fn cmd_mode() -> Result<(), String> {
    let seq = open_ump_client("ump-spike-mode")?;
    println!(
        "client {} negotiated midi_version={} (0=legacy,1=UMP1.0,2=UMP2.0)",
        unsafe { snd_seq_client_id(seq.0) },
        negotiated_midi_version(&seq)?
    );
    Ok(())
}

fn cmd_recv() -> Result<(), String> {
    let seq = open_ump_client("ump-spike-recv")?;
    let v = negotiated_midi_version(&seq)?;
    create_ump_endpoint(&seq, "ump-spike-recv", 1)?;
    // An extra plain writable port so senders can address us without
    // subscription ceremony.
    let pname = CString::new("in").map_err(|e| e.to_string())?;
    let port = check("create_simple_port", unsafe {
        snd_seq_create_simple_port(
            seq.0,
            pname.as_ptr(),
            SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE,
            (1 << 1) | (1 << 20), // MIDI_GENERIC | APPLICATION
        )
    })?;
    println!(
        "recv ready: client={} port={} midi_version={}",
        unsafe { snd_seq_client_id(seq.0) },
        port,
        v
    );
    loop {
        let mut ev: *mut SndSeqUmpEvent = std::ptr::null_mut();
        check("ump_event_input", unsafe {
            snd_seq_ump_event_input(seq.0, &mut ev)
        })?;
        let ev = unsafe { &*ev };
        if ev.flags & SND_SEQ_EVENT_UMP != 0 {
            decode_cvm2(&ev.ump);
        } else {
            println!("legacy event type={} (not UMP!)", ev.r#type);
        }
    }
}

fn cmd_send(client: u8, port: u8) -> Result<(), String> {
    let seq = open_ump_client("ump-spike-send")?;
    let v = negotiated_midi_version(&seq)?;
    // Virtual UMP *output* client — the mamut-seq SET4-7 shape.
    create_ump_endpoint(&seq, "ump-spike-send", 1)?;
    println!(
        "send: client={} midi_version={} -> dest {client}:{port}",
        unsafe { snd_seq_client_id(seq.0) },
        v
    );
    // MIDI 2.0 NoteOn, group 0, ch 0, note 60, velocity16 0xABCD,
    // attribute type 3 (Pitch 7.9) = 60.5 semitones (0x7900).
    let note_on: [u32; 4] = [0x4090_3C03, 0xABCD_7900, 0, 0];
    let note_off: [u32; 4] = [0x4080_3C00, 0x4000_0000, 0, 0];
    for (label, words) in [("NoteOn", note_on), ("NoteOff", note_off)] {
        let mut ev = SndSeqUmpEvent {
            r#type: 0,
            flags: SND_SEQ_EVENT_UMP,
            tag: 0,
            queue: SND_SEQ_QUEUE_DIRECT,
            time: [0; 2],
            source: SndSeqAddr { client: 0, port: 0 },
            dest: SndSeqAddr { client, port },
            ump: words,
        };
        check("ump_event_output", unsafe {
            snd_seq_ump_event_output(seq.0, &mut ev)
        })?;
        check("drain_output", unsafe { snd_seq_drain_output(seq.0) })?;
        println!("sent {label} {words:08X?}");
        std::thread::sleep(std::time::Duration::from_millis(120));
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let r = match args.get(1).map(String::as_str) {
        Some("mode") => cmd_mode(),
        Some("recv") => cmd_recv(),
        Some("send") if args.len() == 4 => {
            match (args[2].parse::<u8>(), args[3].parse::<u8>()) {
                (Ok(c), Ok(p)) => cmd_send(c, p),
                _ => Err("send: client/port must be u8".into()),
            }
        }
        _ => Err("usage: ump-spike mode | recv | send <client> <port>".into()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
```

Layout probe used to pin the `snd_seq_ump_event_t` mirror (C, compiled with
`gcc probe.c -lasound`):

```c
#include <alsa/asoundlib.h>
#include <stdio.h>
#include <stddef.h>
int main(void) {
    printf("sizeof(snd_seq_ump_event_t)=%zu\n", sizeof(snd_seq_ump_event_t));
    printf("offsetof ump=%zu time=%zu source=%zu dest=%zu\n",
           offsetof(snd_seq_ump_event_t, ump),
           offsetof(snd_seq_ump_event_t, time),
           offsetof(snd_seq_ump_event_t, source),
           offsetof(snd_seq_ump_event_t, dest));
    return 0;
}
// output on this host:
// sizeof(snd_seq_ump_event_t)=32
// offsetof ump=16 time=4 source=12 dest=14
```
