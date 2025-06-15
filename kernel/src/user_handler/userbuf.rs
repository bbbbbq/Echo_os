
#[derive(Debug, Clone, Copy)]
pub struct UserBuf<T> {
    pub ptr: *mut T
}

impl<T> core::fmt::Display for UserBuf<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UserBuf({:p})", self.ptr)
    }
}


unsafe impl<T> Send for UserBuf<T> {}
unsafe impl<T> Sync for UserBuf<T> {}

use crate::alloc::string::String;
use crate::alloc::vec::Vec;

const MAX_PATH: usize = 256;

impl<T: Copy> UserBuf<T> {
    pub fn read(&self) -> T {
        unsafe { self.ptr.read() }
    }
}

impl<T> UserBuf<T> {
    pub fn read_string(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let base_ptr = self.ptr as *const u8;
        for i in 0..MAX_PATH {
            let char_ptr = unsafe { base_ptr.add(i) };
            let char_val = unsafe { char_ptr.read_volatile() };
            if char_val == 0 {
                break;
            }
            buffer.push(char_val);
        }
        String::from_utf8(buffer).unwrap_or_default()
    }

    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }
    
    pub fn get_cstr(&self) -> &str {
        unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(self.ptr as *const u8, 64)).unwrap()
        }
    }

    pub fn write(&self, value: T) {
        unsafe {
            self.ptr.write_volatile(value);
        }
    }

    pub fn write_slice(&self, data: &[u8]) {
        unsafe {
            let len = data.len();
            let dst_slice = core::slice::from_raw_parts_mut(self.ptr as *mut u8, len);
            dst_slice.copy_from_slice(data);
        }
    }
    
    pub const fn is_valid(&self) -> bool {
        !self.ptr.is_null()
    }

    pub fn offset(&self, count: isize) -> Self {
        Self { ptr: unsafe { self.ptr.offset(count) } }
    }

    pub fn slice_mut_with_len(&self, len: usize) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, len) }
    }
}
