//! 用户栈(StackRegion)管理模块
//!
//! 提供用户栈的分配、映射、数据压栈等功能。

use alloc::string::ToString;
use memory_addr::{align_up, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use core::mem::size_of;
use page_table_entry::MappingFlags;

use crate::{
    memregion::{MemRegion, MemRegionType},
    pagetable::PageTable,
};

/// 用户栈区域结构体。
#[derive(Debug, Clone)]
pub struct StackRegion {
    pub paddr_range: PhysAddrRange,
    pub vaddr_range: VirtAddrRange,
    pub is_mapped: bool,
    pub sp: usize,
}

impl StackRegion {
    /// 创建新的栈区域。
    pub fn new(paddr_range: PhysAddrRange, vaddr_range: VirtAddrRange) -> Self {
        Self {
            paddr_range,
            vaddr_range,
            is_mapped: false,
            sp: vaddr_range.end.as_usize(),
        }
    }

    /// 创建空栈区域。
    pub fn new_zero() -> Self {
        Self {
            paddr_range: PhysAddrRange::new(PhysAddr::from_usize(0), PhysAddr::from_usize(0)),
            vaddr_range: VirtAddrRange::new(VirtAddr::from_usize(0), VirtAddr::from_usize(0)),
            is_mapped: false,
            sp: 0,
        }
    }

    /// 获取栈顶虚拟地址。
    pub fn get_top(&self) -> usize {
        self.vaddr_range.end.as_usize()
    }

    /// 将栈区域映射到页表。
    pub fn map(&mut self, pagetable: &mut PageTable) {
        let mut mem_region = MemRegion::new_mapped(
            self.vaddr_range.start,
            self.vaddr_range.end,
            self.paddr_range.start,
            self.paddr_range.end,
            MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
            "user_stack".to_string(),
            MemRegionType::STACK,
        );
        let _ = pagetable.map_region_user(&mut mem_region);
        self.is_mapped = true;
    }

    /// 获取当前sp。
    pub fn get_sp(&self) -> usize {
        self.sp
    }

    /// 虚拟地址转物理地址。
    pub fn vaddr_to_paddr(&self, vaddr: VirtAddr) -> PhysAddr {
        if !self.vaddr_range.contains(vaddr) {
            panic!("Virtual address not in range");
        }
        let offset = vaddr.as_usize() - self.vaddr_range.start.as_usize();
        PhysAddr::from_usize(self.paddr_range.start.as_usize() + offset)
    }

    /// 压入usize数组。
    pub fn push_usizes(&mut self, buffer: &[usize]) -> usize {
        let len = buffer.len();
        let bytes_len = len * size_of::<usize>();
        if bytes_len == 0 {
            return self.sp;
        }
        let new_sp = self.sp - bytes_len;
        let dst_vaddr = VirtAddr::from_usize(new_sp);
        let dst_paddr = self.vaddr_to_paddr(dst_vaddr);
        let dst_kernel_vaddr = VirtAddr::from_usize(dst_paddr.as_usize());
        unsafe {
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), dst_kernel_vaddr.as_mut_ptr() as *mut usize, len);
        }
        self.sp = new_sp;
        new_sp
    }

    /// 压入字节数组。
    pub fn push_bytes(&mut self, bytes: &[u8]) -> usize {
        let len = bytes.len();
        if len == 0 {
            return self.sp;
        }
        let ulen = size_of::<usize>();
        let new_sp = self.sp - align_up(len + 1, ulen);
        let dst_vaddr = VirtAddr::from_usize(new_sp);
        let dst_paddr = self.vaddr_to_paddr(dst_vaddr);
        let dst_kernel_vaddr = VirtAddr::from_usize(dst_paddr.as_usize());
        unsafe {
            core::slice::from_raw_parts_mut(dst_kernel_vaddr.as_mut_ptr(), len).copy_from_slice(bytes);
        }
        self.sp = new_sp;
        new_sp
    }

    /// 压入字符串（带结尾0）。
    pub fn push_str(&mut self, str: &str) -> usize {
        let bytes = str.as_bytes();
        // +1 for null terminator
        let len = bytes.len() + 1;
        let ulen = size_of::<usize>();
        let new_sp = self.sp - align_up(len, ulen);
        let dst_vaddr = VirtAddr::from_usize(new_sp);
        let dst_paddr = self.vaddr_to_paddr(dst_vaddr);
        let dst_kernel_vaddr = VirtAddr::from_usize(dst_paddr.as_usize());
        unsafe {
            let ptr = dst_kernel_vaddr.as_mut_ptr();
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            // write null terminator
            core::ptr::write(ptr.add(bytes.len()), 0u8);
        }
        self.sp = new_sp;
        new_sp
    }

    /// 压入单个usize。
    pub fn push_num(&mut self, num: usize) -> usize {
        let ulen = size_of::<usize>();
        let new_sp = self.sp - ulen;
        let dst_vaddr = VirtAddr::from_usize(new_sp);
        let dst_paddr = self.vaddr_to_paddr(dst_vaddr);
        let dst_kernel_vaddr = VirtAddr::from_usize(dst_paddr.as_usize());
        unsafe {
            core::ptr::write(dst_kernel_vaddr.as_mut_ptr() as *mut usize, num);
        }
        self.sp = new_sp;
        new_sp
    }
}
