// 定义系统调用号
#define SYS_WRITE 64
#define SYS_EXIT 93

// 自定义 strlen 函数
long my_strlen(const char *s) {
    long len = 0;
    while (s[len] != '\0') {
        len++;
    }
    return len;
}

// write 系统调用的封装
long sys_write(int fd, const void *buf, long count) {
    long ret;
    asm volatile (
        "li a7, %1\n"
        "mv a0, %2\n"
        "mv a1, %3\n"
        "mv a2, %4\n"
        "ecall\n"
        "mv %0, a0\n"
        : "=r"(ret)
        : "i"(SYS_WRITE), "r"(fd), "r"(buf), "r"(count)
        : "a0", "a1", "a2", "a7", "memory"
    );
    return ret;
}

// exit 系统调用的封装
void sys_exit(int code) {
    asm volatile (
        "li a7, %0\n"
        "mv a0, %1\n"
        "ecall"
        :
        : "i"(SYS_EXIT), "r"(code)
        : "a0", "a7"
    );
}

// 程序入口点
void _start() {
    const char msg[] = "Hello from raw syscall!\n";
    sys_write(1, msg, my_strlen(msg));
    sys_exit(0);
}
