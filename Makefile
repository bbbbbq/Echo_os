# Echo OS Makefile

# 配置变量
RUST_TARGET := riscv64gc-unknown-none-elf
KERNEL_ELF := target/$(RUST_TARGET)/debug/kernel
KERNEL_BIN := $(KERNEL_ELF).bin

# QEMU 参数
QEMU := qemu-system-riscv64
QEMU_MACHINE := virt
QEMU_CPU := rv64
QEMU_MEMORY := 1G
QEMU_DRIVE := -drive file=fs.img,if=none,format=raw,id=x0 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

QEMU_ARGS := -machine $(QEMU_MACHINE)\
	-nographic \
	-cpu $(QEMU_CPU) \
	-m $(QEMU_MEMORY) \
	-bios default

# 构建所有目标
.PHONY: all
all: kernel

# 构建内核
.PHONY: kernel
kernel: cargo-build
	@echo "\n=== 生成内核二进制文件 ==="
	riscv64-unknown-elf-objcopy $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

# 编译 Rust 代码
.PHONY: cargo-build
cargo-build:
	@echo "=== 编译 Rust 代码 ==="
	cargo build --target $(RUST_TARGET)

# 运行 QEMU
.PHONY: run
run: kernel fs-img
	@echo "\n=== 运行 QEMU ==="
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN) $(QEMU_DRIVE)

# 运行 QEMU 并保存日志
.PHONY: runlog
runlog: kernel fs-img
	rm -rf qemu.log
	@echo "\n=== 运行 QEMU 并保存日志到 qemu.log ==="
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN) $(QEMU_DRIVE) -D qemu.log -d int,in_asm,page,mmu,guest_errors

# 运行 QEMU 并捕获详细的 CPU 异常信息
.PHONY: rundbg
rundbg: kernel fs-img
	@echo "\n=== 运行 QEMU 并记录详细的 CPU 异常信息到 qemu.log ==="
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN) $(QEMU_DRIVE) -D qemu.log -d guest_errors,cpu,in_asm,page

# 在 QEMU 中调试
.PHONY: debug
debug: kernel fs-img
	@echo "\n=== 在 QEMU 中调试 ==="
	$(QEMU) $(QEMU_ARGS) -kernel $(KERNEL_BIN) $(QEMU_DRIVE) -s -S

# GDB 连接
.PHONY: gdb
gdb:
	gdb-multiarch \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'

# 使用 GDB 自动运行
.PHONY: gdb-run
gdb-run:
	gdb-multiarch \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234' \
		-ex 'continue'

# 清理编译产物
.PHONY: clean
clean:
	@echo "=== 清理编译产物 ==="
	cargo clean
	-rm -f $(KERNEL_BIN)

# 创建并填充文件系统镜像
.PHONY: fs-img
fs-img:
	@echo "=== 创建并填充文件系统镜像 ==="
	@echo "This will require sudo privileges."
	sudo ./populate_fs_image.sh
	sudo chown $(USER):$(USER) fs.img

# 杀死 tmux 会话
.PHONY: tmux_kill
tmux_kill:
	@echo "=== 关闭已存在的 tmux 调试会话 ==="
	@tmux kill-session -t echo_os_debug 2>/dev/null || true

# 启动 tmux 调试会话
.PHONY: gdb_tmux
gdb_tmux: tmux_kill
	@echo "=== 启动 tmux 调试会话 ==="
	@tmux new-session -d -s echo_os_debug
	@tmux split-window -h -t echo_os_debug
	@tmux send-keys -t echo_os_debug:0.0 "make debug" C-m
	@sleep 1
	@tmux send-keys -t echo_os_debug:0.1 "gdb-multiarch -ex 'file $(KERNEL_ELF)' -ex 'target remote localhost:1234' -ex 'layout split'" C-m
	@tmux attach -t echo_os_debug

asm:
	@echo "=== 生成汇编输出 ==="
	@mkdir -p target/asm
	riscv64-unknown-elf-objdump -d $(KERNEL_ELF) | less 

symbols:
	@echo "=== 显示符号表 ==="
	riscv64-unknown-elf-nm -a $(KERNEL_ELF) | less

sizes:
	@echo "=== 显示各个段的大小 ==="
	riscv64-unknown-elf-size -A $(KERNEL_ELF)
	@echo "\n=== 显示详细的段信息 ==="
	riscv64-unknown-elf-objdump -h $(KERNEL_ELF)

.PHONY: symbols sizes

# 打印帮助信息
.PHONY: help
help:
	@echo "Echo OS 构建系统"
	@echo ""
	@echo "make all         - 构建全部"
	@echo "make kernel      - 仅构建内核"
	@echo "make run         - 构建并运行内核"
	@echo "make debug       - 构建并在调试模式下运行内核"
	@echo "make gdb         - 启动 GDB 并连接到等待的 QEMU 实例"
	@echo "make gdb_tmux    - 启动 tmux 调试会话，包含 QEMU 和 GDB"
	@echo "make asm         - 查看内核汇编代码"
	@echo "make symbols     - 查看内核符号表"
	@echo "make clean       - 清理编译产物"
	@echo "make fs-img      - 创建文件系统镜像"
	@echo "make help        - 打印这个帮助信息"
