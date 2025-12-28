use std::os::unix::io::RawFd;

pub type Result<T> = std::result::Result<T, std::io::Error>;

fn ok_or_get_error(result: libc::c_int) -> Result<libc::c_int> {
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

#[repr(i32)]
pub enum CtlOperation {
    Add = libc::EPOLL_CTL_ADD,
    Mod = libc::EPOLL_CTL_MOD,
    Del = libc::EPOLL_CTL_DEL,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct CtlConfig: u32 {
        /// **Data is ready to read** - The file descriptor has data available.
        const READABLE = libc::EPOLLIN as u32;
        /// **Ready to write** - You can write data without blocking.
        const WRITABLE = libc::EPOLLOUT as u32;
        /// **Urgent data available** - Out-of-band data can be read. (not very used)
        const URGENT_DATA = libc::EPOLLPRI as u32;
        /// **An error occurred** - Something went wrong with this file descriptor.
        ///
        /// You DON'T need to register for this - it's always monitored automatically.
        const ERROR = libc::EPOLLERR as u32;
        /// **Connection hung up** - The other end closed the connection.
        ///
        /// You DON'T need to register for this - it's always monitored automatically.
        ///
        /// IMPORTANT: This doesn't mean all data is gone! You might still have bytes
        /// in your buffer. Keep reading until you get 0 bytes (EOF) to ensure you
        /// consumed everything.
        const HUNG_UP = libc::EPOLLHUP as u32;
        /// **Peer shut down their write end** - The other side closed or half-closed.
        const PEER_CLOSED = libc::EPOLLRDHUP as u32;

        /// **Edge-triggered mode** - Only notify on state *changes*, not continuous state.
        const EDGE_TRIGGERED = libc::EPOLLET as u32;
        /// **One-shot mode** - Automatically disable after one event fires.
        ///
        /// After you get an event, this file descriptor stops being monitored.
        /// You must manually re-enable it with `EPOLL_CTL_MOD`.
        const ONE_SHOT = libc::EPOLLONESHOT as u32;

        /// **Exclusive wakeup** - Wake only ONE epoll instance, not all of them.
        const EXCLUSIVE_WAKE = libc::EPOLLEXCLUSIVE as u32;

        /// **Prevent system suspend** - Keep the system awake while handling this event.
        ///
        /// Requires CAP_BLOCK_SUSPEND capability and only works if EDGE_TRIGGERED
        /// and ONE_SHOT are NOT set.
        ///
        /// The system won't suspend from when the event is returned by `wait()`
        /// until your next `wait()` call, or until you remove/modify the fd.
        ///
        /// Use case: Critical real-time events that can't be delayed by system sleep.
        const KEEP_AWAKE = libc::EPOLLWAKEUP as u32;
    }
}

#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Event<T: Into<u64>> {
    pub config: CtlConfig,
    pub data: T,
}

impl<T: Into<u64>> Event<T> {
    pub fn new(config: CtlConfig, data: T) -> Event<T> {
        Self { config, data }
    }
}

pub fn create(cloexec: bool) -> Result<RawFd> {
    let flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
    unsafe { ok_or_get_error(libc::epoll_create1(flags)) }
}

pub fn ctl<T: Into<u64>>(
    epoll_fd: RawFd,
    operation: CtlOperation,
    fd: RawFd,
    mut config: Event<T>,
) -> Result<()> {
    let config = &mut config as *mut _ as *mut libc::epoll_event;
    unsafe { ok_or_get_error(libc::epoll_ctl(epoll_fd, operation as i32, fd, config))? };
    Ok(())
}

pub fn wait<T: Into<u64>>(
    epoll_fd: RawFd,
    timeout: Option<i32>,
    buf: &mut [Event<T>],
) -> Result<usize> {
    let timeout = timeout.unwrap_or(-1);
    let buf = unsafe {
        std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut libc::epoll_event, buf.len())
    };
    let event_count = unsafe {
        ok_or_get_error(libc::epoll_wait(
            epoll_fd,
            buf.as_mut_ptr(),
            buf.len() as i32,
            timeout,
        ))? as usize
    };
    Ok(event_count)
}

pub fn close(epoll_fd: RawFd) -> Result<()> {
    ok_or_get_error(unsafe { libc::close(epoll_fd) })?;
    Ok(())
}
