#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Statfs {
    pub f_type: u64,       // 文件系统类型
    pub f_bsize: u64,      // 最佳传输块大小
    pub f_blocks: u64,     // 总数据块（以f_bsize为单位）
    pub f_bfree: u64,      // 可用数据块（对超级用户）
    pub f_bavail: u64,     // 可用数据块（对普通用户）
    pub f_files: u64,      // 总文件结点数
    pub f_ffree: u64,      // 可用文件结点数
    pub f_fsid: [u32; 2],  // 文件系统ID
    pub f_namelen: u64,    // 最大文件名长度
    pub f_frsize: u64,     // 碎片大小
    pub f_flags: u64,      // 挂载标志
    pub f_spare: [u64; 4], // 备用
}

impl Statfs {
    pub fn new() -> Self {
        Self {
            f_type: 0,
            f_bsize: 0,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_fsid: [0; 2],
            f_namelen: 0,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        }
    }
} 