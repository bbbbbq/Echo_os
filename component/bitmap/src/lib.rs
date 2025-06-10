#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr, BitXor, Not};

/// A bitmap for efficiently tracking bits
pub struct Bitmap {
    /// The underlying storage as a vector of u64 words
    bits: Vec<u64>,
    /// The total number of bits in the bitmap
    size: usize,
}

impl Bitmap {
    /// Create a new bitmap with the specified number of bits, all set to 0
    pub fn new(size: usize) -> Self {
        let word_count = (size + 63) / 64; // Round up to nearest multiple of 64
        let mut bits = Vec::with_capacity(word_count);
        bits.resize(word_count, 0);

        Self { bits, size }
    }

    /// Create a new bitmap with all bits set to 1
    pub fn new_filled(size: usize) -> Self {
        let word_count = (size + 63) / 64;
        let mut bits = Vec::with_capacity(word_count);
        bits.resize(word_count, u64::MAX);

        // Clear any excess bits in the last word
        if size % 64 != 0 {
            let last_idx = word_count - 1;
            let valid_bits = size % 64;
            bits[last_idx] = (1u64 << valid_bits) - 1;
        }

        Self { bits, size }
    }

    /// Get the total number of bits in the bitmap
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the bitmap is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get the value of a specific bit
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.size {
            return None;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        Some((self.bits[word_idx] & (1u64 << bit_idx)) != 0)
    }

    /// Set a specific bit to 1
    pub fn set(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] |= 1u64 << bit_idx;
        true
    }

    /// Clear a specific bit to 0
    pub fn clear(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] &= !(1u64 << bit_idx);
        true
    }

    /// Toggle a specific bit
    pub fn toggle(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] ^= 1u64 << bit_idx;
        true
    }

    /// Check if all bits are set to 1
    pub fn all(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        // Check all complete words
        let complete_words = self.size / 64;
        for i in 0..complete_words {
            if self.bits[i] != u64::MAX {
                return false;
            }
        }

        // Check remaining bits in the last word
        if self.size % 64 != 0 {
            let last_idx = complete_words;
            let valid_bits = self.size % 64;
            let mask = (1u64 << valid_bits) - 1;

            if (self.bits[last_idx] & mask) != mask {
                return false;
            }
        }

        true
    }

    /// Check if any bit is set to 1
    pub fn any(&self) -> bool {
        if self.is_empty() {
            return false;
        }

        for word in &self.bits {
            if *word != 0 {
                return true;
            }
        }

        false
    }

    /// Count the number of bits set to 1
    pub fn count_ones(&self) -> usize {
        let mut count = 0;

        for word in &self.bits {
            count += word.count_ones() as usize;
        }

        count
    }

    /// Count the number of bits set to 0
    pub fn count_zeros(&self) -> usize {
        self.size - self.count_ones()
    }

    /// Find the index of the first bit set to 1, or None if no bits are set
    pub fn first_set(&self) -> Option<usize> {
        for (word_idx, &word) in self.bits.iter().enumerate() {
            if word != 0 {
                let bit_idx = word.trailing_zeros() as usize;
                let index = word_idx * 64 + bit_idx;

                if index < self.size {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Find the index of the first bit set to 0, or None if all bits are set
    pub fn first_clear(&self) -> Option<usize> {
        for (word_idx, &word) in self.bits.iter().enumerate() {
            if word != u64::MAX {
                let bit_idx = (!word).trailing_zeros() as usize;
                let index = word_idx * 64 + bit_idx;

                if index < self.size {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Set all bits to 1
    pub fn set_all(&mut self) {
        for word in &mut self.bits {
            *word = u64::MAX;
        }

        // Clear any excess bits in the last word
        if self.size % 64 != 0 {
            let last_idx = self.bits.len() - 1;
            let valid_bits = self.size % 64;
            self.bits[last_idx] = (1u64 << valid_bits) - 1;
        }
    }

    /// Clear all bits to 0
    pub fn clear_all(&mut self) {
        for word in &mut self.bits {
            *word = 0;
        }
    }

    /// Resize the bitmap to the new size
    pub fn resize(&mut self, new_size: usize) {
        let new_word_count = (new_size + 63) / 64;

        if new_word_count > self.bits.len() {
            // Expanding the bitmap
            self.bits.resize(new_word_count, 0);
        } else if new_word_count < self.bits.len() {
            // Shrinking the bitmap
            self.bits.truncate(new_word_count);
        }

        // Clear any excess bits in the last word
        if new_size % 64 != 0 {
            let last_idx = new_word_count - 1;
            let valid_bits = new_size % 64;
            let mask = (1u64 << valid_bits) - 1;
            self.bits[last_idx] &= mask;
        }

        self.size = new_size;
    }

    /// Perform a bitwise AND operation with another bitmap
    /// Returns a new bitmap with the result
    pub fn bitand(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] &= other.bits[i];
        }

        result
    }

    /// Perform a bitwise OR operation with another bitmap
    /// Returns a new bitmap with the result
    pub fn bitor(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] |= other.bits[i];
        }

        result
    }

    /// Perform a bitwise XOR operation with another bitmap
    /// Returns a new bitmap with the result
    pub fn bitxor(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] ^= other.bits[i];
        }

        result
    }

    /// Perform a bitwise NOT operation
    /// Returns a new bitmap with the result
    pub fn bitnot(&self) -> Self {
        let mut result = self.clone();

        for i in 0..result.bits.len() {
            result.bits[i] = !result.bits[i];
        }

        // Clear any excess bits in the last word
        if self.size % 64 != 0 {
            let last_idx = result.bits.len() - 1;
            let valid_bits = self.size % 64;
            let mask = (1u64 << valid_bits) - 1;
            result.bits[last_idx] &= mask;
        }

        result
    }
}

// Implementation of common traits

impl Clone for Bitmap {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits.clone(),
            size: self.size,
        }
    }
}

impl BitAnd for &Bitmap {
    type Output = Bitmap;

    fn bitand(self, other: &Bitmap) -> Bitmap {
        self.bitand(other)
    }
}

impl BitOr for &Bitmap {
    type Output = Bitmap;

    fn bitor(self, other: &Bitmap) -> Bitmap {
        self.bitor(other)
    }
}

impl BitXor for &Bitmap {
    type Output = Bitmap;

    fn bitxor(self, other: &Bitmap) -> Bitmap {
        self.bitxor(other)
    }
}

impl Not for &Bitmap {
    type Output = Bitmap;

    fn not(self) -> Bitmap {
        self.bitnot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bitmap = Bitmap::new(100);
        assert_eq!(bitmap.len(), 100);
        assert_eq!(bitmap.count_ones(), 0);
        assert_eq!(bitmap.count_zeros(), 100);
    }

    #[test]
    fn test_new_filled() {
        let bitmap = Bitmap::new_filled(100);
        assert_eq!(bitmap.len(), 100);
        assert_eq!(bitmap.count_ones(), 100);
        assert_eq!(bitmap.count_zeros(), 0);
    }

    #[test]
    fn test_set_clear() {
        let mut bitmap = Bitmap::new(100);
        assert_eq!(bitmap.get(10), Some(false));

        bitmap.set(10);
        assert_eq!(bitmap.get(10), Some(true));

        bitmap.clear(10);
        assert_eq!(bitmap.get(10), Some(false));
    }

    #[test]
    fn test_first_set_clear() {
        let mut bitmap = Bitmap::new(100);
        assert_eq!(bitmap.first_set(), None);
        assert_eq!(bitmap.first_clear(), Some(0));

        bitmap.set(42);
        assert_eq!(bitmap.first_set(), Some(42));

        bitmap.set_all();
        assert_eq!(bitmap.first_clear(), None);
    }
}
