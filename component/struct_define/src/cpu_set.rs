//! CPU集合的数据结构，用于设置CPU亲和性

/// CPU掩码结构，兼容Linux的cpu_set_t
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CpuSet {
    /// 使用位掩码表示CPU核心
    /// 每一位对应一个CPU核心，1表示可用，0表示不可用
    pub bits: [u64; 16],  // 支持最多1024个CPU核心
}

impl CpuSet {
    /// 创建一个新的CPU掩码，默认所有CPU都不可用
    pub fn new() -> Self {
        Self { bits: [0; 16] }
    }
    
    /// 创建一个只有0号CPU可用的掩码
    pub fn only_cpu0() -> Self {
        let mut set = Self::new();
        set.set(0);
        set
    }
    
    /// 设置指定的CPU为可用
    pub fn set(&mut self, cpu: usize) {
        let word = cpu / 64;
        let bit = cpu % 64;
        
        if word < self.bits.len() {
            self.bits[word] |= 1 << bit;
        }
    }
    
    /// 清除指定的CPU（设为不可用）
    pub fn clear(&mut self, cpu: usize) {
        let word = cpu / 64;
        let bit = cpu % 64;
        
        if word < self.bits.len() {
            self.bits[word] &= !(1 << bit);
        }
    }
    
    /// 检查指定的CPU是否可用
    pub fn is_set(&self, cpu: usize) -> bool {
        let word = cpu / 64;
        let bit = cpu % 64;
        
        if word < self.bits.len() {
            (self.bits[word] & (1 << bit)) != 0
        } else {
            false
        }
    }
} 