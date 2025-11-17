
#![allow(dead_code)]
// Auto-generated version information file, do not modify
// Generated at: 2025-11-15 11:24:24 UTC

#[derive(Debug, Clone, Copy)]
pub struct KernelBuildInfo {
    pub version: &'static str,
    pub release: &'static str,
    pub build_user: &'static str,
    pub build_host: &'static str,
    pub build_time: &'static str,
    pub compiler_info: &'static str,
    pub linker_info: &'static str,
    pub git_commit: &'static str,
    pub git_branch: &'static str,
    pub config_flags: &'static str,
}

pub static KERNEL_BUILD_INFO: KernelBuildInfo = KernelBuildInfo {
    version: "#41-dragonos-0.2.0 Sat Nov 15 11:24:24 UTC 2025",
    release: "6.6.21-dragonos",
    build_user: "godones",
    build_host: "Alien",
    build_time: "Sat Nov 15 11:24:24 UTC 2025",
    compiler_info: "rustc 1.91.0-nightly (ca7750494 2025-08-09)",
    linker_info: "GNU ld (GNU Binutils for Ubuntu) 2.38",
    git_commit: "5ca5c0cf-dirty",
    git_branch: "patch-ssh",
    config_flags: "",
};

pub const fn get_kernel_build_info() -> &'static KernelBuildInfo {
    &KERNEL_BUILD_INFO
}
