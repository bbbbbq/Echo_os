

#[derive(Debug, Clone, Copy)]
pub struct UserBuf<T> {
    pub ptr: *mut T
}


unsafe impl<T> Send for UserBuf<T> {}
unsafe impl<T> Sync for UserBuf<T> {}

use crate::alloc::string::String;
use crate::alloc::vec::Vec;

const MAX_PATH: usize = 256;

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
}



