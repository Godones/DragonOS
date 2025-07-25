# 导入环境变量
include ./env.mk

# export ARCH ?= x86_64
# 如果是x86_64, 则使用x86_64-unknown-none.json作为target
ifeq ($(ARCH), x86_64)
	export TARGET_JSON=arch/x86_64/x86_64-unknown-none.json
else ifeq ($(ARCH), riscv64)
	export TARGET_JSON=arch/riscv64/riscv64gc-unknown-none-elf.json
else ifeq ($(ARCH), loongarch64)
	export TARGET_JSON=arch/loongarch64/loongarch64-unknown-none.json
else
	$(error "Unsupported ARCH: $(ARCH)")
endif

export CARGO_ZBUILD=-Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem

.PHONY: ECHO
ECHO:
	@echo "$@"

all: 
	$(MAKE) -C ../build-scripts all
	$(MAKE) -C src all ARCH=$(ARCH) || (sh -c "echo 内核编译失败" && exit 1)
	

clean:
	$(MAKE) -C src clean ARCH=$(ARCH)

.PHONY: fmt
fmt:
	RUSTFLAGS="$(RUSTFLAGS)" cargo fmt --all $(FMT_CHECK)
ifeq ($(ARCH), x86_64)
	RUSTFLAGS="$(RUSTFLAGS)" cargo +nightly-2024-11-05 clippy --all-features --target x86_64-unknown-none
endif


.PHONY: check
check: ECHO
	$(MAKE) -C src check ARCH=$(ARCH)

test:
# 测试内核库
	RUSTFLAGS="$(RUSTFLAGS)" cargo +nightly-2024-11-05 test --workspace --exclude dragonos_kernel
