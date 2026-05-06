//! Deterministic friendly cube name generation.
//!
//! On first boot the cube derives a pleasant default name from its MAC address
//! so that multiple cubes on the same network are easily distinguishable.
//!
//! Format: `<adjective>-<noun>-<4 hex digits from MAC>`
//!
//! Example: `crimson-falcon-7a21`

/// Adjectives pool (kept small to fit in firmware flash).
pub const ADJECTIVES: &[&str] = &[
    "amber", "azure", "bold", "bright", "calm", "coral", "crimson", "dark",
    "dawn", "deep", "dusk", "ember", "faint", "fierce", "frost", "ghost",
    "golden", "grand", "hollow", "iron", "ivory", "jade", "keen", "lunar",
    "misty", "noble", "onyx", "pale", "proud", "quartz", "rapid", "regal",
    "ruby", "rusty", "sage", "scarlet", "shadow", "silent", "silver", "slate",
    "solar", "stark", "stone", "storm", "swift", "thorn", "titan", "umbra",
    "vivid", "wild", "zinc",
];

/// Nouns pool.
pub const NOUNS: &[&str] = &[
    "arrow", "atlas", "badge", "baron", "blade", "bolt", "claw", "comet",
    "crow", "dagger", "drake", "eagle", "falcon", "fang", "forge", "frost",
    "glyph", "hawk", "helm", "horn", "hydra", "knight", "lancer", "lynx",
    "mace", "nexus", "oracle", "pawn", "phoenix", "pike", "pulse", "raven",
    "rebel", "reef", "ridge", "rogue", "sage", "shard", "spark", "sphinx",
    "spire", "storm", "talon", "tiger", "titan", "torch", "viper", "warden",
    "wolf", "wraith", "zenith",
];

/// Derive a friendly cube name from a MAC address (6 bytes).
///
/// Deterministic: same MAC always produces the same name.
pub fn generate_name(mac: &[u8; 6]) -> ([u8; 64], usize) {
    // Use bytes 0-3 for word selection, bytes 4-5 for hex suffix.
    let adj_idx = u16::from_le_bytes([mac[0], mac[1]]) as usize % ADJECTIVES.len();
    let noun_idx = u16::from_le_bytes([mac[2], mac[3]]) as usize % NOUNS.len();
    let suffix = u16::from_le_bytes([mac[4], mac[5]]);

    let adj = ADJECTIVES[adj_idx];
    let noun = NOUNS[noun_idx];

    let mut buf = [0u8; 64];
    let mut pos = 0;

    for &b in adj.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }

    buf[pos] = b'-';
    pos += 1;
    for &b in noun.as_bytes() {
        buf[pos] = b;
        pos += 1;
    }

    buf[pos] = b'-';
    pos += 1;

    // 4-char hex suffix
    let hex_chars = hex_u16(suffix);
    for &b in &hex_chars {
        buf[pos] = b;
        pos += 1;
    }

    (buf, pos)
}

const fn hex_u16(val: u16) -> [u8; 4] {
    let nibbles = [
        (val >> 12) as u8 & 0xF,
        (val >> 8) as u8 & 0xF,
        (val >> 4) as u8 & 0xF,
        val as u8 & 0xF,
    ];
    [
        nibble_to_hex(nibbles[0]),
        nibble_to_hex(nibbles[1]),
        nibble_to_hex(nibbles[2]),
        nibble_to_hex(nibbles[3]),
    ]
}

const fn nibble_to_hex(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'a' + n - 10 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_deterministic() {
        let mac = [0xAB, 0x12, 0xCD, 0x34, 0x7A, 0x21];
        let (buf1, len1) = generate_name(&mac);
        let (buf2, len2) = generate_name(&mac);
        assert_eq!(&buf1[..len1], &buf2[..len2]);
        let name = core::str::from_utf8(&buf1[..len1]).unwrap();
        assert!(name.contains('-'));
        assert!(name.ends_with("217a"));
    }
}
