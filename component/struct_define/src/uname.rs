//! 系统信息(UTSname)结构体定义
//!
//! 用于实现uname等系统调用，保存系统标识信息。

/// UTSname 结构体，保存系统名称、节点名、版本等信息。
#[derive(Debug, Copy, Clone)]   
pub struct UTSname {
    /// 操作系统名称
    pub sysname: [u8; 65],
    /// 网络节点名称
    pub nodename: [u8; 65],
    /// 操作系统发行版本
    pub release: [u8; 65],
    /// 操作系统版本
    pub version: [u8; 65],
    /// 机器硬件名称
    pub machine: [u8; 65],
    /// 域名
    pub domainname: [u8; 65],
}

impl core::fmt::Display for UTSname {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "System: {}", core::str::from_utf8(&self.sysname).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))?;
        writeln!(f, "Node: {}", core::str::from_utf8(&self.nodename).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))?;
        writeln!(f, "Release: {}", core::str::from_utf8(&self.release).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))?;
        writeln!(f, "Version: {}", core::str::from_utf8(&self.version).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))?;
        writeln!(f, "Machine: {}", core::str::from_utf8(&self.machine).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))?;
        write!(f, "Domain: {}", core::str::from_utf8(&self.domainname).unwrap_or("Invalid UTF-8").trim_end_matches('\0'))
    }
}

impl UTSname {
    /// 创建一个空的UTSname结构体。
    pub fn new() -> Self {
        UTSname {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}