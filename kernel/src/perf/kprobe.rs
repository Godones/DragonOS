use super::Result;
use crate::arch::interrupt::TrapFrame;
use crate::bpf::helper::BPF_HELPER_FUN_SET;
use crate::bpf::prog::BpfProg;
use crate::debug::kprobe::args::KprobeInfo;
use crate::debug::kprobe::{register_kprobe, unregister_kprobe, LockKprobe};
use crate::filesystem::vfs::file::File;
use crate::libs::casting::DowncastArc;
use crate::perf::util::PerfProbeArgs;
use crate::perf::PerfEventOps;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt::Debug;
use kprobe::{CallBackFunc, ProbeArgs};
use rbpf::EbpfVmRawOwned;
use system_error::SystemError;

#[derive(Debug)]
pub struct KprobePerfEvent {
    args: PerfProbeArgs,
    kprobe: LockKprobe,
}

impl Drop for KprobePerfEvent {
    fn drop(&mut self) {
        unregister_kprobe(self.kprobe.clone()).unwrap();
    }
}

impl KprobePerfEvent {
    pub fn do_set_bpf_prog(&self, prog_file: Arc<File>) -> Result<()> {
        let file = prog_file
            .inode()
            .downcast_arc::<BpfProg>()
            .ok_or(SystemError::EINVAL)?;
        let prog_slice = file.insns();
        let mut vm = EbpfVmRawOwned::new(Some(prog_slice.to_vec())).unwrap();
        vm.register_helper_set(BPF_HELPER_FUN_SET.get()).unwrap();

        // create a callback to execute the ebpf prog
        let callback = Box::new(KprobePerfCallBack::new(file, vm));
        // update callback for kprobe
        self.kprobe.write().update_event_callback(callback);
        Ok(())
    }
}

pub struct KprobePerfCallBack {
    bpf_prog_file: Arc<BpfProg>,
    vm: EbpfVmRawOwned,
}

impl KprobePerfCallBack {
    fn new(bpf_prog_file: Arc<BpfProg>, vm: EbpfVmRawOwned) -> Self {
        Self { bpf_prog_file, vm }
    }
}

impl CallBackFunc for KprobePerfCallBack {
    fn call(&self, trap_frame: &dyn ProbeArgs) {
        let trap_frame = trap_frame.as_any().downcast_ref::<TrapFrame>().unwrap();
        // log::info!("CallBackFunc {:#x?}", trap_frame);
        let pt_regs = pt_regs::from(trap_frame);
        let probe_context = unsafe {
            core::slice::from_raw_parts_mut(
                &pt_regs as *const pt_regs as *mut u8,
                size_of::<pt_regs>(),
            )
        };
        // log::info!("The pt_regs ptr: {:#x}", pt_regs as *const TrapFrame as *mut u8 as usize);
        // log::info!("---------------------Running probe---------------------");
        let _res = self.vm.execute_program(probe_context).unwrap();
        // log::info!("Program returned: {res:?} ({res:#x})");
        // log::info!("---------------------Probe finished---------------------");
    }
}

impl PerfEventOps for KprobePerfEvent {
    fn set_bpf_prog(&self, bpf_prog: Arc<File>) -> Result<()> {
        self.do_set_bpf_prog(bpf_prog)
    }
    fn enable(&self) -> Result<()> {
        self.kprobe.write().enable();
        Ok(())
    }
    fn disable(&self) -> Result<()> {
        self.kprobe.write().disable();
        Ok(())
    }
}

pub fn perf_event_open_kprobe(args: PerfProbeArgs) -> KprobePerfEvent {
    let symbol = args.name.clone();
    log::info!("create kprobe for symbol: {symbol}");
    let kprobe_info = KprobeInfo {
        pre_handler: |_| {
            // log::info!("pre_handler:kprobe for perf_event_open_kprobe")
        },
        post_handler: |_| {
            // log::info!("post_handler:kprobe for perf_event_open_kprobe")
        },
        fault_handler: None,
        event_callback: None,
        symbol: Some(symbol),
        addr: None,
        offset: 0,
        enable: false,
    };
    let kprobe = register_kprobe(kprobe_info).expect("create kprobe failed");
    KprobePerfEvent { args, kprobe }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pt_regs {
    pub r15: ::core::ffi::c_ulong,
    pub r14: ::core::ffi::c_ulong,
    pub r13: ::core::ffi::c_ulong,
    pub r12: ::core::ffi::c_ulong,
    pub rbp: ::core::ffi::c_ulong,
    pub rbx: ::core::ffi::c_ulong,
    pub r11: ::core::ffi::c_ulong,
    pub r10: ::core::ffi::c_ulong,
    pub r9: ::core::ffi::c_ulong,
    pub r8: ::core::ffi::c_ulong,
    pub rax: ::core::ffi::c_ulong,
    pub rcx: ::core::ffi::c_ulong,
    pub rdx: ::core::ffi::c_ulong,
    pub rsi: ::core::ffi::c_ulong,
    pub rdi: ::core::ffi::c_ulong,
    pub orig_rax: ::core::ffi::c_ulong,
    pub rip: ::core::ffi::c_ulong,
    pub cs: ::core::ffi::c_ulong,
    pub eflags: ::core::ffi::c_ulong,
    pub rsp: ::core::ffi::c_ulong,
    pub ss: ::core::ffi::c_ulong,
}

impl From<&TrapFrame> for pt_regs {
    fn from(trap_frame: &TrapFrame) -> Self {
        Self {
            r15: trap_frame.r15,
            r14: trap_frame.r14,
            r13: trap_frame.r13,
            r12: trap_frame.r12,
            rbp: trap_frame.rbp,
            rbx: trap_frame.rbx,
            r11: trap_frame.r11,
            r10: trap_frame.r10,
            r9: trap_frame.r9,
            r8: trap_frame.r8,
            rax: trap_frame.rax,
            rcx: trap_frame.rcx,
            rdx: trap_frame.rdx,
            rsi: trap_frame.rsi,
            rdi: trap_frame.rdi,
            orig_rax: 0,
            rip: trap_frame.rip,
            cs: trap_frame.cs,
            eflags: trap_frame.rflags,
            rsp: trap_frame.rsp,
            ss: trap_frame.ss,
        }
    }
}
