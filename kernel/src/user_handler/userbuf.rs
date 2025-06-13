pub struct UserBuf<T> {
    pub ptr: *mut T
}


impl<T> UserBuf<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }
}

impl UserBuf<u8> {
    pub fn get_cstr(&self) -> &str {
        unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(self.ptr as *const u8, 64)).unwrap()
        }
    }
}
