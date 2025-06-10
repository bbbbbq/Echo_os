/// Macro to export a function as the kernel entry point with name "kernel_main"
///
/// # Example
/// ```
/// #[kernel_main]
/// pub fn my_kernel_entry() -> ! {
///     // Kernel initialization code
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
