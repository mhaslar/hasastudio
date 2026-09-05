use std::io;

#[cfg(unix)]
pub(super) fn current() -> io::Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: value is writable timespec storage; the clock reads this live thread's CPU time.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut value) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((value.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(value.tv_nsec as u64))
}

#[cfg(windows)]
pub(super) fn current() -> io::Result<u64> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    // SAFETY: signatures, FILETIME layout and calling convention match processthreadsapi.h.
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadTimes(
            thread: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let (mut creation, mut exit, mut kernel, mut user) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    // SAFETY: the pseudo-handle targets this calling thread; all four outputs are live writable FILETIMEs.
    if unsafe {
        GetThreadTimes(
            GetCurrentThread(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let ticks = |v: FileTime| (u64::from(v.high) << 32) | u64::from(v.low);
    Ok(ticks(kernel)
        .saturating_add(ticks(user))
        .saturating_mul(100))
}
