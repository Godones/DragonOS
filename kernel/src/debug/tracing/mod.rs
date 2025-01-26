mod events;
pub mod tracepoint;

use crate::debug::sysfs::debugfs_kset;
use crate::driver::base::kobject::KObject;
use crate::filesystem::kernfs::callback::{KernCallbackData, KernFSCallback};
use crate::filesystem::kernfs::KernFSInode;
use crate::filesystem::vfs::syscall::ModeType;
use crate::filesystem::vfs::PollStatus;
use alloc::string::ToString;
use alloc::sync::Arc;
use system_error::SystemError;

static mut TRACING_ROOT_INODE: Option<Arc<KernFSInode>> = None;

fn tracing_root_inode() -> Arc<KernFSInode> {
    unsafe { TRACING_ROOT_INODE.clone().unwrap() }
}

/// Initialize the debugfs tracing directory
pub fn init_debugfs_tracing() -> Result<(), SystemError> {
    let debugfs = debugfs_kset();
    let root_dir = debugfs.inode().unwrap();
    let tracing_root = root_dir.add_dir(
        "tracing".to_string(),
        ModeType::from_bits_truncate(0o555),
        None,
        Some(&TracingDirCallBack),
    )?;
    let events_root = tracing_root.add_dir(
        "events".to_string(),
        ModeType::from_bits_truncate(0o755),
        None,
        Some(&TracingDirCallBack),
    )?;

    events::init_events(events_root)?;

    unsafe {
        TRACING_ROOT_INODE = Some(tracing_root);
    }
    Ok(())
}

#[derive(Debug)]
pub struct TracingDirCallBack;

impl KernFSCallback for TracingDirCallBack {
    fn open(&self, _data: KernCallbackData) -> Result<(), SystemError> {
        Ok(())
    }

    fn read(
        &self,
        _data: KernCallbackData,
        _buf: &mut [u8],
        _offset: usize,
    ) -> Result<usize, SystemError> {
        Err(SystemError::EISDIR)
    }

    fn write(
        &self,
        _data: KernCallbackData,
        _buf: &[u8],
        _offset: usize,
    ) -> Result<usize, SystemError> {
        Err(SystemError::EISDIR)
    }

    fn poll(&self, _data: KernCallbackData) -> Result<PollStatus, SystemError> {
        Err(SystemError::EISDIR)
    }
}
