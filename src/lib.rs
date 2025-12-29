use std::os::unix::io::RawFd;

pub type Result<T> = std::result::Result<T, std::io::Error>;

fn ok_or_get_error(result: libc::c_int) -> Result<libc::c_int> {
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Interest: u32 {
        /// Monitor for data available to read
        const READ = libc::EPOLLIN as u32;
        /// Monitor for ready to write without blocking
        const WRITE = libc::EPOLLOUT as u32;
        /// Monitor for urgent out-of-band data (TCP OOB data, rarely used)
        const URGENT = libc::EPOLLPRI as u32;
        /// Monitor for error conditions (always monitored automatically, but can be explicit)
        const ERROR = libc::EPOLLERR as u32;
        /// Monitor for hang up (always monitored automatically, but can be explicit)
        const HANG_UP = libc::EPOLLHUP as u32;
        /// Monitor for peer closing their write end (graceful shutdown detection)
        const READ_CLOSED = libc::EPOLLRDHUP as u32;

        /// Use edge-triggered mode (only notify on state changes)
        const EDGE_TRIGGERED = libc::EPOLLET as u32;
        /// One-shot mode (automatically disable after one event)
        const ONE_SHOT = libc::EPOLLONESHOT as u32;
        /// Exclusive wakeup (wake only one epoll instance, not all)
        const EXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
        /// Prevent system suspend while handling events (requires CAP_BLOCK_SUSPEND)
        const WAKE_UP = libc::EPOLLWAKEUP as u32;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Events: u32 {
        /// Data is available to read
        const READABLE = libc::EPOLLIN as u32;
        /// Can write without blocking
        const WRITABLE = libc::EPOLLOUT as u32;
        /// Urgent out-of-band data available
        const URGENT = libc::EPOLLPRI as u32;

        /// Error condition occurred
        const ERROR = libc::EPOLLERR as u32;
        /// Hang up (connection closed)
        const HANG_UP = libc::EPOLLHUP as u32;
        /// Peer closed their write end
        const READ_CLOSED = libc::EPOLLRDHUP as u32;
    }
}

#[repr(i32)]
pub enum CtlOperation {
    Add = libc::EPOLL_CTL_ADD,
    Mod = libc::EPOLL_CTL_MOD,
    Del = libc::EPOLL_CTL_DEL,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Event {
    pub config: u32,
    pub data: RawFd,
}

impl Event {
    pub fn new(config: Interest, data: RawFd) -> Event {
        Self {
            config: config.bits(),
            data,
        }
    }
}

impl Event {
    pub fn events(self) -> Events {
        Events::from_bits_truncate(self.config)
    }
}

pub fn create(cloexec: bool) -> Result<RawFd> {
    let flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
    unsafe { ok_or_get_error(libc::epoll_create1(flags)) }
}

pub fn ctl(epoll_fd: RawFd, operation: CtlOperation, fd: RawFd, config: Event) -> Result<()> {
    let mut config = libc::epoll_event {
        events: config.config,
        u64: config.data as u64,
    };
    unsafe { ok_or_get_error(libc::epoll_ctl(epoll_fd, operation as i32, fd, &mut config))? };
    Ok(())
}

pub fn wait(epoll_fd: RawFd, timeout: Option<i32>, buf: &mut [Event]) -> Result<usize> {
    let timeout = timeout.unwrap_or(-1);
    let mut sys = vec![libc::epoll_event { events: 0, u64: 0 }; buf.len()];
    // let buffer = unsafe {
    //     std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut libc::epoll_event, buf.len())
    // };

    let n = unsafe {
        ok_or_get_error(libc::epoll_wait(
            epoll_fd,
            sys.as_mut_ptr(),
            sys.len() as i32,
            timeout,
        ))? as usize
    };

    for (dst, src) in buf.iter_mut().zip(sys.iter()).take(n) {
        dst.config = src.events;
        dst.data = src.u64 as RawFd;
    }

    Ok(n)
}

pub fn close(epoll_fd: RawFd) -> Result<()> {
    ok_or_get_error(unsafe { libc::close(epoll_fd) })?;
    Ok(())
}
