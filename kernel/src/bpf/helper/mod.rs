mod print;
use crate::bpf::helper::print::trace_printf;
use crate::bpf::map::BpfMap;
use crate::bpf::map::{PerCpuInfo, PerCpuInfoImpl};
use crate::include::bindings::linux_bpf::BPF_F_CURRENT_CPU;
use crate::libs::lazy_init::Lazy;
use alloc::{collections::BTreeMap, sync::Arc};
use core::ffi::c_void;
use system_error::SystemError;

type RawBPFHelperFn = fn(u64, u64, u64, u64, u64) -> u64;
type Result<T> = core::result::Result<T, SystemError>;
macro_rules! define_func {
    ($name:ident) => {
        core::mem::transmute::<usize, RawBPFHelperFn>($name as usize)
    };
}

/// See https://ebpf-docs.dylanreimerink.nl/linux/helper-function/bpf_map_lookup_elem/
pub unsafe fn raw_map_lookup_elem(map: *mut c_void, key: *const c_void) -> *const c_void {
    let map = Arc::from_raw(map as *const BpfMap);
    let key_size = map.key_size();
    let key = core::slice::from_raw_parts(key as *const u8, key_size);
    let value = map_lookup_elem(&map, key);
    // log::info!("<raw_map_lookup_elem>: {:x?}", value);
    // warning: We need to keep the map alive, so we don't drop it here.
    let _ = Arc::into_raw(map);
    match value {
        Ok(Some(value)) => value as *const c_void,
        _ => core::ptr::null_mut(),
    }
}

pub fn map_lookup_elem(map: &Arc<BpfMap>, key: &[u8]) -> Result<Option<*const u8>> {
    let binding = map.inner_map().lock();
    // let key_value = u32::from_ne_bytes(key[0..4].try_into().unwrap());
    // log::info!("<map_lookup_elem> key_value: {:?}", key_value);
    let value = binding.lookup_elem(key);
    match value {
        Ok(Some(value)) => Ok(Some(value.as_ptr())),
        _ => Ok(None),
    }
}

/// See https://ebpf-docs.dylanreimerink.nl/linux/helper-function/bpf_perf_event_output/
///
/// See https://man7.org/linux/man-pages/man7/bpf-helpers.7.html
pub unsafe fn raw_perf_event_output(
    ctx: *mut c_void,
    map: *mut c_void,
    flags: u64,
    data: *mut c_void,
    size: u64,
) -> i64 {
    // log::info!("<raw_perf_event_output>: {:x?}", data);
    let map = Arc::from_raw(map as *const BpfMap);
    let data = core::slice::from_raw_parts(data as *const u8, size as usize);
    let res = perf_event_output(ctx, &map, flags, data);
    // warning: We need to keep the map alive, so we don't drop it here.
    let _ = Arc::into_raw(map);
    match res {
        Ok(_) => 0,
        Err(e) => e as i64,
    }
}

pub fn perf_event_output(
    ctx: *mut c_void,
    map: &Arc<BpfMap>,
    flags: u64,
    data: &[u8],
) -> Result<()> {
    let binding = map.inner_map().lock();
    let index = flags as u32;
    let flags = (flags >> 32) as u32;
    let key = if index == BPF_F_CURRENT_CPU as u32 {
        let cpu_id = PerCpuInfoImpl::cpu_id();
        cpu_id
    } else {
        index
    };
    let fd = binding.lookup_elem(&key.to_ne_bytes()).unwrap().unwrap();
    let fd = u32::from_ne_bytes(fd.try_into().unwrap());
    // log::info!(
    //     "<perf_event_output>: flags: {:x?}, index: {:x?}, fd: {:x?}",
    //     flags,
    //     index,
    //     fd
    // );
    crate::perf::perf_event_output(ctx, fd as usize, flags, data)?;
    Ok(())
}

/// See https://ebpf-docs.dylanreimerink.nl/linux/helper-function/bpf_probe_read/
pub fn raw_bpf_probe_read(dst: *mut c_void, size: u32, unsafe_ptr: *const c_void) -> i64 {
    log::info!(
        "raw_bpf_probe_read, dst:{:x}, size:{}, unsafe_ptr: {:x}",
        dst as usize,
        size,
        unsafe_ptr as usize
    );
    let (dst, src) = unsafe {
        let dst = core::slice::from_raw_parts_mut(dst as *mut u8, size as usize);
        let src = core::slice::from_raw_parts(unsafe_ptr as *const u8, size as usize);
        (dst, src)
    };
    let res = bpf_probe_read(dst, src);
    match res {
        Ok(_) => 0,
        Err(e) => e as i64,
    }
}

/// For tracing programs, safely attempt to read size
/// bytes from kernel space address unsafe_ptr and
/// store the data in dst.
pub fn bpf_probe_read(dst: &mut [u8], src: &[u8]) -> Result<()> {
    log::info!("bpf_probe_read: len: {}", dst.len());
    dst.copy_from_slice(src);
    Ok(())
}

pub unsafe fn raw_map_update_elem(
    map: *mut c_void,
    key: *const c_void,
    value: *const c_void,
    flags: u64,
) -> i64 {
    let map = Arc::from_raw(map as *const BpfMap);
    let key_size = map.key_size();
    let value_size = map.value_size();
    // log::info!("<raw_map_update_elem>: flags: {:x?}", flags);
    let key = core::slice::from_raw_parts(key as *const u8, key_size);
    let value = core::slice::from_raw_parts(value as *const u8, value_size);
    let res = map_update_elem(&map, key, value, flags);
    let _ = Arc::into_raw(map);
    match res {
        Ok(_) => 0,
        Err(e) => e as _,
    }
}

pub fn map_update_elem(map: &Arc<BpfMap>, key: &[u8], value: &[u8], flags: u64) -> Result<()> {
    let mut binding = map.inner_map().lock();
    let value = binding.update_elem(key, value, flags);
    value
}

pub static BPF_HELPER_FUN_SET: Lazy<BTreeMap<u32, RawBPFHelperFn>> = Lazy::new();

/// Initialize the helper functions.
pub fn init_helper_functions() {
    let mut map = BTreeMap::new();
    unsafe {
        map.insert(1, define_func!(raw_map_lookup_elem));
        map.insert(2, define_func!(raw_map_update_elem));
        map.insert(25, define_func!(raw_perf_event_output));
        map.insert(6, define_func!(trace_printf));
        map.insert(4, define_func!(raw_bpf_probe_read));
    }
    BPF_HELPER_FUN_SET.init(map);
}
