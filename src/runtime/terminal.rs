//! Cursor queries for the native terminal; the Aula supplies its own host.

use std::io;

#[cfg(unix)]
pub(crate) fn cursor_position() -> io::Result<(i64, i64)> {
    use std::fs::OpenOptions;
    use std::io::{IsTerminal, Read, Write};
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    if !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consulta da posição do cursor exige saída em um terminal",
        ));
    }
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let fd = tty.as_raw_fd();
    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let original = unsafe { original.assume_init() };
    struct Restore(i32, libc::termios);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe { libc::tcsetattr(self.0, libc::TCSANOW, &self.1) };
        }
    }
    let mut mode = original;
    mode.c_lflag &= !(libc::ICANON | libc::ECHO);
    mode.c_cc[libc::VMIN] = 1;
    mode.c_cc[libc::VTIME] = 0;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &mode) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let _restore = Restore(fd, original);
    tty.write_all(b"\x1b[6n")?;
    tty.flush()?;
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut response = Vec::new();
    while response.len() < 64 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let mut poll = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if ready < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        if ready == 0 {
            break;
        }
        let mut byte = [0];
        tty.read_exact(&mut byte)?;
        response.push(byte[0]);
        if byte[0] == b'R' {
            return parse_response(&response).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "resposta inválida à consulta do cursor",
                )
            });
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "o terminal não respondeu à consulta do cursor",
    ))
}

#[cfg(any(unix, test))]
fn parse_response(bytes: &[u8]) -> Option<(i64, i64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let (row, col) = text
        .strip_prefix("\x1b[")?
        .strip_suffix('R')?
        .split_once(';')?;
    let row = row.parse::<i64>().ok()?.checked_sub(1)?;
    let col = col.parse::<i64>().ok()?.checked_sub(1)?;
    (row >= 0 && col >= 0).then_some((row, col))
}

#[cfg(windows)]
pub(crate) fn cursor_position() -> io::Result<(i64, i64)> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Coord {
        x: i16,
        y: i16,
    }
    #[repr(C)]
    struct Rect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }
    #[repr(C)]
    struct Info {
        size: Coord,
        cursor: Coord,
        attributes: u16,
        window: Rect,
        max_size: Coord,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(kind: u32) -> *mut c_void;
        fn GetConsoleScreenBufferInfo(handle: *mut c_void, info: *mut Info) -> i32;
    }
    let mut info = std::mem::MaybeUninit::<Info>::uninit();
    if unsafe { GetConsoleScreenBufferInfo(GetStdHandle(-11_i32 as u32), info.as_mut_ptr()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let info = unsafe { info.assume_init() };
    Ok((
        i64::from(info.cursor.y) - i64::from(info.window.top),
        i64::from(info.cursor.x) - i64::from(info.window.left),
    ))
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn cursor_position() -> io::Result<(i64, i64)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "consulta do cursor indisponível nesta plataforma",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cursor_response_uses_zero_based_coordinates() {
        assert_eq!(super::parse_response(b"\x1b[1;1R"), Some((0, 0)));
        assert_eq!(super::parse_response(b"\x1b[12;34R"), Some((11, 33)));
        for invalid in [
            b"\x1b[0;1R".as_slice(),
            b"\x1b[1;0R",
            b"\x1b[-1;2R",
            b"text",
            b"\x1b[1;2H",
        ] {
            assert_eq!(super::parse_response(invalid), None);
        }
    }
}
