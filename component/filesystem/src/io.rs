use super::vfs::{Inode, VfsResult};
use alloc::sync::Arc;
use core::fmt;
use console::print;
pub struct Stdin {

}

pub struct Stdout {
}

pub struct Stderror
{

}

impl Inode for Stdout
{
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        use core::str;
        if let Ok(s) = str::from_utf8(buf) {
            print!("{}", s);
        }
        Ok(buf.len())
    }
}


impl Inode for Stderror
{
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        use core::str;
        if let Ok(s) = str::from_utf8(buf) {
            print!("{}", s);
        }
        Ok(buf.len())
    }
}