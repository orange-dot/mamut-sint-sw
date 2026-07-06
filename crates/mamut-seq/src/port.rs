//! Virtual ALSA MIDI output port with built-in panic hygiene.
//!
//! On ALSA, `midir` exposes virtual ports through the `os::unix::VirtualOutput`
//! trait (no cargo feature required). The port tracks held notes and the
//! channels it has touched so it can send an *effective* panic on quit and on
//! `Drop`: explicit note-offs plus sustain clear are what actually silence the
//! Mamut receiver (which ignores CC 120/121/123); the channel-mode messages are
//! emitted for any other well-behaved receiver.

use std::collections::BTreeSet;

use anyhow::{Result, anyhow};
use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};

use crate::midi::{self, MidiChannel};

/// A virtual MIDI output port `mamut-seq` publishes on.
pub struct VirtualPort {
    connection: MidiOutputConnection,
    port_name: String,
    /// Notes believed to be sounding, keyed by `(channel_number, note)`.
    held_notes: BTreeSet<(u8, u8)>,
    /// Channels this port has emitted channel-voice messages on.
    active_channels: BTreeSet<u8>,
    panic_cc: Option<u8>,
    reset_controllers_cc: Option<u8>,
}

impl VirtualPort {
    /// Open a virtual output port. On a name collision (or other create error)
    /// the port name is suffixed with the process id and retried once.
    pub fn open(port_name: &str) -> Result<Self> {
        let output = new_output()?;
        match output.create_virtual(port_name) {
            Ok(connection) => Ok(Self::from_connection(connection, port_name.to_string())),
            Err(error) => {
                let fallback = format!("{port_name}-{}", std::process::id());
                let retry_output = new_output()?;
                retry_output
                    .create_virtual(&fallback)
                    .map(|connection| Self::from_connection(connection, fallback.clone()))
                    .map_err(|retry_error| {
                        anyhow!(
                            "failed to open virtual MIDI port `{port_name}` ({error}) and \
                             fallback `{fallback}` ({retry_error})"
                        )
                    })
            }
        }
    }

    fn from_connection(connection: MidiOutputConnection, port_name: String) -> Self {
        Self {
            connection,
            port_name,
            held_notes: BTreeSet::new(),
            active_channels: BTreeSet::new(),
            panic_cc: None,
            reset_controllers_cc: None,
        }
    }

    /// The name the port was actually opened under (post collision suffix).
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Configure the receiver-side panic/reset CCs used during panic hygiene.
    pub fn configure_panic(&mut self, panic_cc: Option<u8>, reset_controllers_cc: Option<u8>) {
        self.panic_cc = panic_cc;
        self.reset_controllers_cc = reset_controllers_cc;
    }

    /// Register a channel so panic resets it even if no note has been sent.
    pub fn mark_channel(&mut self, channel: MidiChannel) {
        self.active_channels.insert(channel.number());
    }

    /// Send a pre-encoded message, updating held-note / active-channel state
    /// from the outgoing status byte.
    pub fn send_tracked(&mut self, message: &[u8]) -> Result<()> {
        if let Some(&status) = message.first() {
            let kind = status & 0xF0;
            let channel = (status & 0x0F) + 1;
            match kind {
                0x90 if message.len() >= 3 => {
                    self.active_channels.insert(channel);
                    if message[2] == 0 {
                        self.held_notes.remove(&(channel, message[1] & 0x7F));
                    } else {
                        self.held_notes.insert((channel, message[1] & 0x7F));
                    }
                }
                0x80 if message.len() >= 2 => {
                    self.held_notes.remove(&(channel, message[1] & 0x7F));
                }
                _ if kind < 0xF0 => {
                    self.active_channels.insert(channel);
                }
                _ => {}
            }
        }
        self.send_raw(message)
    }

    /// Send an *effective* panic: note-off every tracked held note, clear
    /// sustain, drive the profile's panic/reset controls, then the conventional
    /// channel-mode messages.
    ///
    /// Best-effort **per message**: a failed send does not abort the remaining
    /// cleanup — this is the crate's primary safety property (never leave notes
    /// stuck), and a send failure is exactly when it matters most. The first
    /// error is returned for non-`Drop` callers; the `Drop` path ignores it.
    pub fn panic(&mut self) -> Result<()> {
        let mut outcome = Ok(());

        let held: Vec<(u8, u8)> = self.held_notes.iter().copied().collect();
        for (channel_number, note) in held {
            if let Some(channel) = MidiChannel::new(channel_number) {
                self.try_send(&midi::note_off(channel, note), &mut outcome);
            }
        }
        self.held_notes.clear();

        let channels: Vec<u8> = self.active_channels.iter().copied().collect();
        let panic_cc = self.panic_cc;
        let reset_controllers_cc = self.reset_controllers_cc;
        for channel_number in channels {
            let Some(channel) = MidiChannel::new(channel_number) else {
                continue;
            };
            // Sustain clear — honoured by the Mamut receiver.
            self.try_send(
                &midi::control_change(channel, midi::CC_SUSTAIN, 0),
                &mut outcome,
            );
            // Drive the receiver's own panic/reset if the profile maps them.
            if let Some(cc) = panic_cc {
                self.try_send(&midi::control_change(channel, cc, 127), &mut outcome);
            }
            if let Some(cc) = reset_controllers_cc {
                self.try_send(&midi::control_change(channel, cc, 127), &mut outcome);
            }
            // Conventional channel-mode messages: no-ops against Mamut, correct
            // citizenship for any other receiver.
            self.try_send(
                &midi::control_change(channel, midi::CC_ALL_SOUND_OFF, 0),
                &mut outcome,
            );
            self.try_send(
                &midi::control_change(channel, midi::CC_RESET_ALL_CONTROLLERS, 0),
                &mut outcome,
            );
            self.try_send(
                &midi::control_change(channel, midi::CC_ALL_NOTES_OFF, 0),
                &mut outcome,
            );
        }
        outcome
    }

    /// Send a message, recording the first error into `outcome` but never
    /// aborting the caller's cleanup sequence.
    fn try_send(&mut self, message: &[u8], outcome: &mut Result<()>) {
        if let Err(error) = self.send_raw(message) {
            if outcome.is_ok() {
                *outcome = Err(error);
            }
        }
    }

    fn send_raw(&mut self, message: &[u8]) -> Result<()> {
        self.connection
            .send(message)
            .map_err(|error| anyhow!("failed to send MIDI message: {error}"))
    }
}

impl Drop for VirtualPort {
    fn drop(&mut self) {
        // Best-effort cleanup on normal exit and on `?`/panic unwind. A hard
        // kill (SIGKILL/SIGTERM) bypasses this; that gap is documented.
        let _ = self.panic();
    }
}

fn new_output() -> Result<MidiOutput> {
    MidiOutput::new("mamut-seq").map_err(|error| anyhow!("failed to create MIDI output: {error}"))
}
