/// 导出函数为内核入口点（符号名为 "kernel_main"）的宏。
///
/// # 用途
/// 用于将自定义函数导出为裸机内核的主入口。
///
/// # 示例
/// ```rust
/// #[kernel_main]
/// pub fn my_kernel_entry() -> ! {
///     // 内核初始化代码
/// }
/// ```
#[macro_export]
macro_rules! kernel_main {
    ($(#[$meta:meta])* $vis:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)? $body:block) => {
        $(#[$meta])*
        #[unsafe(export_name = "kernel_main")]
        #[unsafe(no_mangle)]
        $vis fn $name($($args)*) $(-> $ret)? $body
    };
}
