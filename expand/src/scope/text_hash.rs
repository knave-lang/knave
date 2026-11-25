pub(super) struct TextHash;

impl TextHash {
    const FACTOR: u64 = 0x517c_c1b7_2722_0a95;
    const ROTATION: u32 = 5;
    const FINAL_ROTATION: u32 = 26;
    const CHUNK: usize = size_of::<u64>();
    const UNTAGGED: u32 = u32::MAX;

    pub(super) fn of(text: &str, tag: u32) -> u64 {
        let chunks = text.as_bytes().chunks(Self::CHUNK);
        let hash = chunks.fold(0, |hash, chunk| Self::mix(hash, Self::chunk_word(chunk)));
        Self::mix(hash, Self::closing_word(text.len(), tag)).rotate_left(Self::FINAL_ROTATION)
    }

    pub(super) fn untagged(text: &str) -> u64 {
        Self::of(text, Self::UNTAGGED)
    }

    pub(super) fn first_slot(hash: u64, mask: usize) -> usize {
        let wide_mask = u64::try_from(mask).expect("a table's mask fits 64 bits");
        usize::try_from(hash & wide_mask).expect("a slot fits its mask")
    }

    fn closing_word(length: usize, tag: u32) -> u64 {
        (length as u64) << u32::BITS | u64::from(tag)
    }

    fn mix(hash: u64, word: u64) -> u64 {
        (hash.rotate_left(Self::ROTATION) ^ word).wrapping_mul(Self::FACTOR)
    }

    fn chunk_word(chunk: &[u8]) -> u64 {
        let mut word = [0; Self::CHUNK];
        word[..chunk.len()].copy_from_slice(chunk);
        u64::from_le_bytes(word)
    }
}
