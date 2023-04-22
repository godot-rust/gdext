/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Validates is a [`u32`] slice contains only valid [unicode scalar values](https://www.unicode.org/glossary/#unicode_scalar_value)
pub fn validate_unicode_scalar_sequence(seq: &[u32]) -> Option<&[char]> {
    unsafe {
        let mut ptr = seq.as_ptr();
        let ptr_end = seq.as_ptr().add(seq.len());

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        loop {
            let ptr_next = ptr.add(4);
            if ptr_next > ptr_end {
                break;
            }

            let block = _mm_loadu_si128(ptr as *const __m128i);

            // check if has any character greater than `char::MAX` or less than 0, (SSE2 uses signed math)
            if _mm_movemask_epi8(_mm_and_si128(
                _mm_cmpgt_epi32(block, _mm_set1_epi32(-1)),
                _mm_cmplt_epi32(block, _mm_set1_epi32(char::MAX as i32 + 1)),
            )) != 0xFFFF
            {
                return None;
            }

            // check if has any high-surrogate and low-surrogate code points
            if _mm_testz_si128(
                _mm_cmpgt_epi32(block, _mm_set1_epi32(0xD7FF)),
                _mm_cmplt_epi32(block, _mm_set1_epi32(0xE000)),
            ) == 0
            {
                return None;
            }

            ptr = ptr_next;
        }

        #[cfg(target_arch = "aarch64")]
        loop {
            let ptr_next = ptr.add(4);
            if ptr_next > ptr_end {
                break;
            }

            let block = vld1q_u32(ptr as *const u32);

            // check if has any character bigger than `char::MAX`
            if vmaxvq_u32(block) >= char::MAX as u32 {
                return None;
            }

            // check if has any high-surrogate and low-surrogate code points
            // This is in the range `0xD800..0xE000`.
            if vminvq_u32(vsubq_u32(block, vdupq_n_u32(0xD800))) < (0xE000 - 0xD800) {
                return None;
            }

            ptr = ptr_next;
        }

        loop {
            if ptr >= ptr_end {
                break;
            }

            char::from_u32(*ptr)?;

            ptr = ptr.add(1);
        }

        Some(std::slice::from_raw_parts(
            seq.as_ptr() as *const char,
            seq.len(),
        ))
    }
}

#[cfg(test)]
mod tests {
    // simple random pseudorandom number generator using the linear congruential method
    struct Rand {
        state: u64,
    }

    impl Rand {
        const A: u64 = 6364136223846793005;
        const C: u64 = 1442695040888963407;

        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next(&mut self) -> u32 {
            self.state = Self::A.wrapping_mul(self.state).wrapping_add(Self::C);
            self.state as u32
        }
    }

    #[test]
    fn check_valid_unicode() {
        let mut rand = Rand::new(0xA102FE1);
        for _ in 0..16 {
            let len = (rand.next() % 128).min(80);
            let chars: Vec<u32> = (0..len)
                .map(|_| rand.next() % (char::MAX as u32))
                .filter_map(char::from_u32)
                .map(|x| x as u32)
                .collect();

            assert!(!chars.is_empty());

            assert!(super::validate_unicode_scalar_sequence(chars.as_slice()).is_some());
        }
    }

    #[test]
    fn check_unpaired_surrogate_unicode() {
        let mut rand = Rand::new(0xA102FE1);
        for _ in 0..16 {
            let len = (rand.next() % 128).min(80);
            let mut chars: Vec<u32> = (0..len)
                .map(|_| rand.next() % char::MAX as u32)
                .filter_map(char::from_u32)
                .map(|x| x as u32)
                .collect();

            assert!(!chars.is_empty());

            for _ in 0..4 {
                let surrogate = rand.next() % (0xE000 - 0xD800) + 0xD800;
                assert!(char::from_u32(surrogate).is_none());
                chars.insert(rand.next() as usize % chars.len(), surrogate);
            }

            assert!(super::validate_unicode_scalar_sequence(chars.as_slice()).is_none());
        }
    }

    #[test]
    fn check_out_of_range_unicode() {
        let mut rand = Rand::new(0xA102FE1);
        for _ in 0..16 {
            let len = (rand.next() % 128).min(80);
            let mut chars: Vec<u32> = (0..len)
                .map(|_| rand.next() % char::MAX as u32)
                .filter_map(char::from_u32)
                .map(|x| x as u32)
                .collect();

            assert!(!chars.is_empty());

            for _ in 0..4 {
                let out_of_range = rand.next() % (u32::MAX - char::MAX as u32) + char::MAX as u32;
                assert!(char::from_u32(out_of_range).is_none());
                chars.insert(rand.next() as usize % chars.len(), out_of_range);
            }

            assert!(super::validate_unicode_scalar_sequence(chars.as_slice()).is_none());
        }
    }
}
