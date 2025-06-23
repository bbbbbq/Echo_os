#![no_std]

//! 位图(bitmap)模块
//!
//! 提供高效的位操作与管理，常用于内存分配、资源跟踪等场景。

extern crate alloc;

use alloc::vec::Vec;
use core::ops::{BitAnd, BitOr, BitXor, Not};

/// 位图结构体，用于高效管理和操作大量二进制位。
///
/// # 字段
/// * `bits` - 以u64为单位的位存储向量。
/// * `size` - 位图的总位数。
pub struct Bitmap {
    /// The underlying storage as a vector of u64 words
    bits: Vec<u64>,
    /// The total number of bits in the bitmap
    size: usize,
}

impl Bitmap {
    /// 创建一个指定大小、所有位为0的位图。
    ///
    /// # 参数
    /// * `size` - 位图的总位数。
    /// # 返回
    /// 新的Bitmap实例。
    pub fn new(size: usize) -> Self {
        let word_count = (size + 63) / 64; // Round up to nearest multiple of 64
        let mut bits = Vec::with_capacity(word_count);
        bits.resize(word_count, 0);

        Self { bits, size }
    }

    /// 创建一个所有位为1的位图。
    ///
    /// # 参数
    /// * `size` - 位图的总位数。
    /// # 返回
    /// 新的Bitmap实例。
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

    /// 获取位图的总位数。
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// 判断位图是否为空。
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// 获取指定下标的位。
    ///
    /// # 参数
    /// * `index` - 位的下标。
    /// # 返回
    /// `Some(true)`表示该位为1，`Some(false)`为0，越界返回None。
    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.size {
            return None;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        Some((self.bits[word_idx] & (1u64 << bit_idx)) != 0)
    }

    /// 将指定下标的位设置为1。
    ///
    /// # 参数
    /// * `index` - 位的下标。
    /// # 返回
    /// 设置成功返回true，越界返回false。
    pub fn set(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] |= 1u64 << bit_idx;
        true
    }

    /// 将指定下标的位清零。
    ///
    /// # 参数
    /// * `index` - 位的下标。
    /// # 返回
    /// 清零成功返回true，越界返回false。
    pub fn clear(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] &= !(1u64 << bit_idx);
        true
    }

    /// 翻转指定下标的位。
    ///
    /// # 参数
    /// * `index` - 位的下标。
    /// # 返回
    /// 翻转成功返回true，越界返回false。
    pub fn toggle(&mut self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }

        let word_idx = index / 64;
        let bit_idx = index % 64;

        self.bits[word_idx] ^= 1u64 << bit_idx;
        true
    }

    /// 判断所有位是否都为1。
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

    /// 判断是否存在至少一位为1。
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

    /// 统计所有为1的位数。
    pub fn count_ones(&self) -> usize {
        let mut count = 0;

        for word in &self.bits {
            count += word.count_ones() as usize;
        }

        count
    }

    /// 统计所有为0的位数。
    pub fn count_zeros(&self) -> usize {
        self.size - self.count_ones()
    }

    /// 查找第一个为1的位的下标。
    ///
    /// # 返回
    /// 若存在，返回Some(index)，否则返回None。
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

    /// 查找第一个为0的位的下标。
    ///
    /// # 返回
    /// 若存在，返回Some(index)，否则返回None。
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

    /// 将所有位设置为1。
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

    /// 将所有位清零。
    pub fn clear_all(&mut self) {
        for word in &mut self.bits {
            *word = 0;
        }
    }

    /// 调整位图大小。
    ///
    /// # 参数
    /// * `new_size` - 新的位图大小。
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

    /// 位与操作。
    ///
    /// # 参数
    /// * `other` - 另一个位图。
    /// # 返回
    /// 新的位图，结果为self & other。
    pub fn bitand(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] &= other.bits[i];
        }

        result
    }

    /// 位或操作。
    ///
    /// # 参数
    /// * `other` - 另一个位图。
    /// # 返回
    /// 新的位图，结果为self | other。
    pub fn bitor(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] |= other.bits[i];
        }

        result
    }

    /// 位异或操作。
    ///
    /// # 参数
    /// * `other` - 另一个位图。
    /// # 返回
    /// 新的位图，结果为self ^ other。
    pub fn bitxor(&self, other: &Self) -> Self {
        let mut result = self.clone();
        let min_len = core::cmp::min(self.bits.len(), other.bits.len());

        for i in 0..min_len {
            result.bits[i] ^= other.bits[i];
        }

        result
    }

    /// 位取反操作。
    ///
    /// # 返回
    /// 新的位图，结果为!self。
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

// trait实现

impl Clone for Bitmap {
    /// 克隆位图。
    fn clone(&self) -> Self {
        Self {
            bits: self.bits.clone(),
            size: self.size,
        }
    }
}

impl BitAnd for &Bitmap {
    type Output = Bitmap;
    /// 按位与操作符实现。
    fn bitand(self, other: &Bitmap) -> Bitmap {
        self.bitand(other)
    }
}

impl BitOr for &Bitmap {
    type Output = Bitmap;
    /// 按位或操作符实现。
    fn bitor(self, other: &Bitmap) -> Bitmap {
        self.bitor(other)
    }
}

impl BitXor for &Bitmap {
    type Output = Bitmap;
    /// 按位异或操作符实现。
    fn bitxor(self, other: &Bitmap) -> Bitmap {
        self.bitxor(other)
    }
}

impl Not for &Bitmap {
    type Output = Bitmap;
    /// 按位取反操作符实现。
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
