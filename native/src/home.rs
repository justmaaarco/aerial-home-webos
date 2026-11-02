//! Local Home relaunch signal. Only resets this app's view; never launches an
//! app, selects an input or changes TV settings. Avoids competing with SDL's
//! existing registerApp subscription on webOS.
use std::{
    fs, io,
    os::unix::{
        fs::{FileTypeExt, PermissionsExt},
        net::UnixDatagram,
    },
    path::PathBuf,
};
fn path() -> io::Result<PathBuf> {
    Ok(crate::platform::storage_directory()
        .map_err(io::Error::other)?
        .join("home.sock"))
}
pub fn notify() -> io::Result<()> {
    let socket = UnixDatagram::unbound()?;
    socket.send_to(b"home", path()?)?;
    Ok(())
}
pub struct HomeSignal {
    socket: UnixDatagram,
    path: PathBuf,
}
impl HomeSignal {
    pub fn bind() -> io::Result<Self> {
        Self::bind_at(path()?)
    }
    fn bind_at(path: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(path.parent().unwrap())?;
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if !meta.file_type().is_socket() {
                return Err(io::Error::other("Home signal path is not a socket"));
            }
            if UnixDatagram::unbound()?.send_to(b"home", &path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Native launcher is already running",
                ));
            }
            fs::remove_file(&path)?;
        }
        let socket = UnixDatagram::bind(&path)?;
        socket.set_nonblocking(true)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        Ok(Self { socket, path })
    }
    pub fn take(&self) -> bool {
        let mut bytes = [0; 16];
        let mut home = false;
        while let Ok(n) = self.socket.recv(&mut bytes) {
            home |= &bytes[..n] == b"home";
        }
        home
    }
}
impl Drop for HomeSignal {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path).is_ok_and(|m| m.file_type().is_socket()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_signal_accepts_only_home_and_does_not_replace_other_files() {
        let dir = std::env::temp_dir().join(format!(
            "aerial-home-signal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("home.sock");
        let signal = HomeSignal::bind_at(path.clone()).unwrap();
        let client = UnixDatagram::unbound().unwrap();
        client.send_to(b"launch PS5", &path).unwrap();
        assert!(!signal.take());
        client.send_to(b"home", &path).unwrap();
        assert!(signal.take());
        assert!(!signal.take());
        drop(signal);
        fs::write(&path, b"existing file").unwrap();
        assert!(HomeSignal::bind_at(path.clone()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"existing file");
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
