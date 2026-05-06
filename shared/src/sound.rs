//! Sound format constants.

pub const SAMPLE_RATE: u32 = 16000;
pub const BIT_DEPTH: u8 = 16;
pub const CHANNELS: u8 = 1;
pub const BYTES_PER_SAMPLE: usize = (BIT_DEPTH as usize) / 8;
pub const BYTES_PER_SECOND: usize = SAMPLE_RATE as usize * BYTES_PER_SAMPLE * CHANNELS as usize;

/// Maximum sound duration in seconds (to prevent SD exhaustion).
pub const MAX_SOUND_DURATION_S: u32 = 60;
/// Maximum raw PCM file size.
pub const MAX_SOUND_BYTES: usize = BYTES_PER_SECOND * MAX_SOUND_DURATION_S as usize;
