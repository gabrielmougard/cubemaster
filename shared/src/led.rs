//! LED pattern types and constants.

pub const LEDS_PER_FACE: usize = 256;
pub const FACES_COUNT: usize = 4;
pub const LEDS_TOTAL: usize = LEDS_PER_FACE * FACES_COUNT;
pub const BYTES_PER_LED: usize = 3;
pub const FRAME_SIZE: usize = LEDS_PER_FACE * BYTES_PER_LED;
pub const ALL_FACES_FRAME_SIZE: usize = FRAME_SIZE * FACES_COUNT;

pub type RgbColor = [u8; 3];
pub type FaceFrame = [RgbColor; LEDS_PER_FACE];
