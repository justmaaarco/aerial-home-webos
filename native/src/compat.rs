//! The broad-compatibility webOS SDK predates getauxval. Rust uses it during
//! startup, so read actual ARM kernel auxiliary records without allocating.
use std::ffi::{c_char, c_int, c_void};
extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn __errno_location() -> *mut c_int;
}

#[no_mangle]
pub unsafe extern "C" fn getauxval(key: usize) -> usize {
    let errno = __errno_location();
    let original = *errno;
    let fd = open(b"/proc/self/auxv\0".as_ptr().cast(), 0x80000);
    let mut found = false;
    let mut value = 0;
    if fd >= 0 {
        'records: loop {
            let mut entry = [0usize; 2];
            let mut filled = 0;
            while filled < std::mem::size_of_val(&entry) {
                let n = read(
                    fd,
                    entry.as_mut_ptr().cast::<u8>().add(filled).cast(),
                    std::mem::size_of_val(&entry) - filled,
                );
                if n < 0 && *errno == 4 {
                    continue;
                }
                if n <= 0 {
                    break 'records;
                }
                filled += n as usize;
            }
            if entry[0] == 0 {
                break;
            }
            if entry[0] == key {
                found = true;
                value = entry[1];
                break;
            }
        }
        close(fd);
    }
    *errno = if found { original } else { 2 };
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_reads_real_kernel_page_size_and_preserves_errno() {
        unsafe {
            extern "C" {
                fn sysconf(name: c_int) -> isize;
            }
            let errno = __errno_location();
            *errno = 13;
            assert_eq!(getauxval(6), sysconf(30) as usize);
            assert_eq!(*errno, 13);
            assert_eq!(getauxval(usize::MAX), 0);
            assert_eq!(*errno, 2);
        }
    }
}
