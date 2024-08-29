#![allow(unused)]
mod bpf;
mod kprobe;
mod util;

use crate::filesystem::vfs::file::{File, FileMode};
use crate::filesystem::vfs::syscall::ModeType;
use crate::filesystem::vfs::{FilePrivateData, FileSystem, FileType, IndexNode, Metadata};
use crate::include::bindings::linux_bpf::{
    perf_event_attr, perf_event_sample_format, perf_sw_ids, perf_type_id,
};
use crate::libs::casting::DowncastArc;
use crate::libs::spinlock::{SpinLock, SpinLockGuard};
use crate::net::event_poll::{EPollEventType, EPollItem, EventPoll, KernelIoctlData};
use crate::perf::bpf::BpfPerfEvent;
use crate::perf::util::{PerfEventIoc, PerfEventOpenFlags, PerfProbeArgs};
use crate::process::ProcessManager;
use crate::syscall::user_access::UserBufferReader;
use crate::syscall::Syscall;
use alloc::boxed::Box;
use alloc::collections::LinkedList;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::any::Any;
use core::ffi::c_void;
use core::fmt::Debug;
use core::ops::Deref;
use intertrait::{CastFrom, CastFromSync};
use log::info;
use num_traits::FromPrimitive;
use system_error::SystemError;

type Result<T> = core::result::Result<T, SystemError>;

pub trait PerfEventOps: Send + Sync + Debug + CastFromSync + CastFrom {
    fn mmap(&self, _start: usize, _len: usize, _offset: usize) -> Result<()> {
        panic!("mmap not implemented for PerfEvent");
    }
    fn set_bpf_prog(&self, _bpf_prog: Arc<File>) -> Result<()> {
        panic!("set_bpf_prog not implemented for PerfEvent");
    }
    fn enable(&self) -> Result<()> {
        panic!("enable not implemented");
    }
    fn disable(&self) -> Result<()> {
        panic!("disable not implemented");
    }
    fn readable(&self) -> bool {
        panic!("readable not implemented");
    }
}

#[derive(Debug)]
pub struct PerfEventInode {
    event: Box<dyn PerfEventOps>,
    epitems: SpinLock<LinkedList<Arc<EPollItem>>>,
}

impl PerfEventInode {
    pub fn new(event: Box<dyn PerfEventOps>) -> Self {
        Self {
            event,
            epitems: SpinLock::new(LinkedList::new()),
        }
    }
    pub fn remove_epoll(
        &self,
        epoll: &Weak<SpinLock<EventPoll>>,
    ) -> core::result::Result<(), SystemError> {
        let is_remove = !self
            .epitems
            .lock_irqsave()
            .extract_if(|x| x.epoll().ptr_eq(epoll))
            .collect::<Vec<_>>()
            .is_empty();
        if is_remove {
            return Ok(());
        }
        Err(SystemError::ENOENT)
    }
    fn do_poll(&self) -> Result<usize> {
        let mut events = EPollEventType::empty();
        if self.event.readable() {
            events |= EPollEventType::EPOLLIN | EPollEventType::EPOLLRDNORM;
        }
        return Ok(events.bits() as usize);
    }
    fn epoll_callback(&self) -> Result<()> {
        let pollflag = EPollEventType::from_bits_truncate(self.do_poll()? as u32);
        // 唤醒epoll中等待的进程
        EventPoll::wakeup_epoll(&self.epitems, pollflag)
    }
}

impl Deref for PerfEventInode {
    type Target = Box<dyn PerfEventOps>;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl IndexNode for PerfEventInode {
    fn mmap(&self, start: usize, len: usize, offset: usize) -> Result<()> {
        self.event.mmap(start, len, offset)
    }
    fn open(&self, _data: SpinLockGuard<FilePrivateData>, _mode: &FileMode) -> Result<()> {
        Ok(())
    }
    fn close(&self, _data: SpinLockGuard<FilePrivateData>) -> Result<()> {
        Ok(())
    }
    fn read_at(
        &self,
        _offset: usize,
        _len: usize,
        _buf: &mut [u8],
        _data: SpinLockGuard<FilePrivateData>,
    ) -> Result<usize> {
        panic!("read_at not implemented for PerfEvent");
    }

    fn write_at(
        &self,
        _offset: usize,
        _len: usize,
        _buf: &[u8],
        _data: SpinLockGuard<FilePrivateData>,
    ) -> Result<usize> {
        panic!("write_at not implemented for PerfEvent");
    }

    fn poll(&self, _private_data: &FilePrivateData) -> Result<usize> {
        self.do_poll()
    }

    fn metadata(&self) -> Result<Metadata> {
        let meta = Metadata {
            mode: ModeType::from_bits_truncate(0o755),
            file_type: FileType::File,
            ..Default::default()
        };
        Ok(meta)
    }

    fn resize(&self, _len: usize) -> Result<()> {
        Ok(())
    }

    fn ioctl(&self, cmd: u32, data: usize, _private_data: &FilePrivateData) -> Result<usize> {
        let req = PerfEventIoc::from_u32(cmd).ok_or(SystemError::EINVAL)?;
        info!("perf_event_ioctl: request: {:?}, arg: {}", req, data);
        match req {
            PerfEventIoc::Enable => {
                self.event.enable()?;
                Ok(0)
            }
            PerfEventIoc::Disable => {
                self.event.disable()?;
                Ok(0)
            }
            PerfEventIoc::SetBpf => {
                info!("perf_event_ioctl: PERF_EVENT_IOC_SET_BPF, arg: {}", data);
                let bpf_prog_fd = data;
                let fd_table = ProcessManager::current_pcb().fd_table();
                let file = fd_table
                    .read()
                    .get_file_by_fd(bpf_prog_fd as _)
                    .ok_or(SystemError::EBADF)?;
                self.event.set_bpf_prog(file)?;
                Ok(0)
            }
        }
    }

    fn kernel_ioctl(
        &self,
        arg: Arc<dyn KernelIoctlData>,
        _data: &FilePrivateData,
    ) -> core::result::Result<usize, SystemError> {
        let epitem = arg
            .arc_any()
            .downcast::<EPollItem>()
            .map_err(|_| SystemError::EFAULT)?;
        self.epitems.lock().push_back(epitem);
        Ok(0)
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        panic!("PerfEvent does not have a filesystem")
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
    fn list(&self) -> Result<Vec<String>> {
        Err(SystemError::ENOSYS)
    }
}

impl Syscall {
    pub fn sys_perf_event_open(
        attr: *const u8,
        pid: i32,
        cpu: i32,
        group_fd: i32,
        flags: u32,
    ) -> Result<usize> {
        let buf = UserBufferReader::new(
            attr as *const perf_event_attr,
            size_of::<perf_event_attr>(),
            true,
        )?;
        let attr = buf.read_one_from_user(0)?;
        perf_event_open(attr, pid, cpu, group_fd, flags)
    }
}

pub fn perf_event_open(
    attr: &perf_event_attr,
    pid: i32,
    cpu: i32,
    group_fd: i32,
    flags: u32,
) -> Result<usize> {
    let args = PerfProbeArgs::try_from(attr, pid, cpu, group_fd, flags)?;
    log::info!("perf_event_process: {:#?}", args);
    let file_mode = if args
        .flags
        .contains(PerfEventOpenFlags::PERF_FLAG_FD_CLOEXEC)
    {
        FileMode::O_RDWR | FileMode::O_CLOEXEC
    } else {
        FileMode::O_RDWR
    };

    let event: Box<dyn PerfEventOps> = match args.type_ {
        // Kprobe
        // See /sys/bus/event_source/devices/kprobe/type
        perf_type_id::PERF_TYPE_MAX => {
            let kprobe_event = kprobe::perf_event_open_kprobe(args);
            Box::new(kprobe_event)
        }
        perf_type_id::PERF_TYPE_SOFTWARE => {
            // For bpf prog output
            assert_eq!(args.config, perf_sw_ids::PERF_COUNT_SW_BPF_OUTPUT);
            assert_eq!(
                args.sample_type,
                Some(perf_event_sample_format::PERF_SAMPLE_RAW)
            );
            let bpf_event = bpf::perf_event_open_bpf(args);
            Box::new(bpf_event)
        }
        _ => {
            unimplemented!("perf_event_process: unknown type: {:?}", args);
        }
    };
    let perf_event = PerfEventInode::new(event);
    let file = File::new(Arc::new(perf_event), file_mode)?;
    let fd_table = ProcessManager::current_pcb().fd_table();
    let fd = fd_table.write().alloc_fd(file, None).map(|x| x as usize)?;
    Ok(fd)
}

pub fn perf_event_output(_ctx: *mut c_void, fd: usize, _flags: u32, data: &[u8]) -> Result<()> {
    let file = get_perf_event_file(fd)?;
    // info!("perf_event_output: fd: {}, flags: {:x?}", fd, flags);
    let bpf_event_file = file.deref().deref();
    let bpf_event_file = bpf_event_file
        .deref()
        .ref_any()
        .downcast_ref::<BpfPerfEvent>()
        .unwrap();
    bpf_event_file.write_event(data)?;
    file.epoll_callback()?;
    Ok(())
}

fn get_perf_event_file(fd: usize) -> Result<Arc<PerfEventInode>> {
    let fd_table = ProcessManager::current_pcb().fd_table();
    let file = fd_table
        .read()
        .get_file_by_fd(fd as _)
        .ok_or(SystemError::EBADF)?;
    let event = file
        .inode()
        .downcast_arc::<PerfEventInode>()
        .ok_or(SystemError::EINVAL)?;
    Ok(event)
}
