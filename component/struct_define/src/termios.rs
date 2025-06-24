// termios.rs - 终端 I/O 控制结构和常量
// 基于 Linux 内核的定义

// 终端 ioctl 请求码
// 来源：Linux 内核 include/uapi/asm-generic/ioctls.h

pub const TCGETS: usize = 0x5401;     // 获取终端属性
pub const TCSETS: usize = 0x5402;     // 设置终端属性
pub const TCSETSW: usize = 0x5403;    // 排空输出队列后设置终端属性
pub const TCSETSF: usize = 0x5404;    // 排空输入输出队列后设置终端属性
pub const TCGETA: usize = 0x5405;     // 获取终端属性 (old version)
pub const TCSETA: usize = 0x5406;     // 设置终端属性 (old version)
pub const TCSETAW: usize = 0x5407;    // 排空输出队列后设置终端属性 (old version)
pub const TCSETAF: usize = 0x5408;    // 排空输入输出队列后设置终端属性 (old version)
pub const TCSBRK: usize = 0x5409;     // 发送BREAK信号
pub const TCXONC: usize = 0x540A;     // 流控制
pub const TCFLSH: usize = 0x540B;     // 排空缓冲区
pub const TIOCEXCL: usize = 0x540C;   // 设置独占模式
pub const TIOCNXCL: usize = 0x540D;   // 取消独占模式
pub const TIOCSCTTY: usize = 0x540E;  // 设置为控制终端
pub const TIOCGPGRP: usize = 0x540F;  // 获取前台进程组
pub const TIOCSPGRP: usize = 0x5410;  // 设置前台进程组
pub const TIOCOUTQ: usize = 0x5411;   // 查询输出队列中的字节数
pub const TIOCSTI: usize = 0x5412;    // 模拟终端输入
pub const TIOCGWINSZ: usize = 0x5413; // 获取窗口大小
pub const TIOCSWINSZ: usize = 0x5414; // 设置窗口大小

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Termios {
    pub c_iflag: u32,  // 输入模式标志
    pub c_oflag: u32,  // 输出模式标志
    pub c_cflag: u32,  // 控制模式标志
    pub c_lflag: u32,  // 本地模式标志
    pub c_line: u8,    // 行规程
    pub c_cc: [u8; 32], // 控制字符
}

// 终端窗口大小
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Winsize {
    pub ws_row: u16,    // 行数
    pub ws_col: u16,    // 列数
    pub ws_xpixel: u16, // 每行像素数
    pub ws_ypixel: u16, // 每列像素数
} 