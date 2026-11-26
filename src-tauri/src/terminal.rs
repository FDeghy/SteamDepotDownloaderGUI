use crate::steam::SteamDownload;
use portable_pty::{MasterPty, PtySize, native_pty_system};
use std::collections::HashSet;
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, BorrowedHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, ipc::Channel};

#[derive(Default)]
pub struct Terminal {
    active: Option<Active>,
    size: Option<PtySize>,
}

struct Active {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    #[cfg(windows)]
    process: OwnedHandle,
    #[cfg(not(windows))]
    killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    cancelled: bool,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn engine_path() -> Result<PathBuf, String> {
    Ok(std::env::current_exe()
        .map_err(err)?
        .with_file_name(if cfg!(windows) {
            "DepotDownloaderMod.exe"
        } else {
            "DepotDownloaderMod"
        }))
}

#[tauri::command]
pub async fn engine_version() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut cmd = std::process::Command::new(engine_path()?);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd.arg("--version").output().map_err(err)?;
        let version = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || !version.contains(include_str!("../engine-revision").trim())
        {
            return Err("Bundled engine version check failed".into());
        }
        Ok(version.lines().next().unwrap_or_default().to_string())
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn async_write_to_pty(data: String, state: State<'_, Mutex<Terminal>>) -> Result<(), String> {
    if let Some(active) = &mut state.lock().map_err(err)?.active {
        active.writer.write_all(data.as_bytes()).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn async_resize_pty(
    rows: u16,
    cols: u16,
    state: State<'_, Mutex<Terminal>>,
) -> Result<(), String> {
    let size = PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        ..Default::default()
    };
    let mut term = state.lock().map_err(err)?;
    term.size = Some(size);
    if let Some(active) = &term.active {
        active.master.resize(size).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, Mutex<Terminal>>) -> Result<(), String> {
    if let Some(active) = &mut state.lock().map_err(err)?.active {
        #[cfg(windows)]
        {
            // portable-pty 0.9.0 reverses the success test in WinChildKiller.
            // This owned handle always identifies the process spawned for this job.
            if unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(
                    active.process.as_raw_handle(),
                    1,
                )
            } == 0
            {
                return Err(err(std::io::Error::last_os_error()));
            }
        }
        #[cfg(not(windows))]
        active.killer.kill().map_err(err)?;
        active.cancelled = true;
    }
    Ok(())
}

#[tauri::command]
pub async fn start_download(
    steam_download: SteamDownload,
    app: AppHandle,
    output: Channel<Vec<u8>>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || run(steam_download, app, output))
        .await
        .map_err(err)?
}

fn run(job: SteamDownload, app: AppHandle, output: Channel<Vec<u8>>) -> Result<String, String> {
    let cwd = app
        .path()
        .local_data_dir()
        .map_err(err)?
        .join("SteamDepotDownloaderGUI");
    let keys = job.imported_key_file()?;
    let cmd = job.command(&engine_path()?, &cwd, keys.as_deref())?;
    std::fs::create_dir_all(cwd).map_err(err)?;
    let state = app.state::<Mutex<Terminal>>();
    let mut terminal = state.lock().map_err(err)?;
    if terminal.active.is_some() {
        return Err("A download is already running".into());
    }
    let pair = native_pty_system()
        .openpty(terminal.size.unwrap_or_default())
        .map_err(err)?;
    let reader = pair.master.try_clone_reader().map_err(err)?;
    let writer = pair.master.take_writer().map_err(err)?;
    let mut child = pair.slave.spawn_command(cmd).map_err(err)?;
    drop(pair.slave);
    #[cfg(windows)]
    let process = {
        // The child owns the original handle for the duration of this borrow.
        let handle = child.as_raw_handle().and_then(|raw| {
            unsafe { BorrowedHandle::borrow_raw(raw) }
                .try_clone_to_owned()
                .ok()
        });
        match handle {
            Some(handle) => handle,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Could not retain the engine process handle for cancellation".into());
            }
        }
    };
    terminal.active = Some(Active {
        writer,
        master: pair.master,
        #[cfg(windows)]
        process,
        #[cfg(not(windows))]
        killer: child.clone_killer(),
        cancelled: false,
    });
    drop(terminal);
    let _ = output.send(Vec::new()); // Acknowledge startup before enabling cancellation.

    // This revision can return success after skipping an unauthorized depot.
    // Check its per-depot completion reports before showing success in the GUI.
    let expected = job
        .depots
        .iter()
        .map(|d| format!("Depot {} - Downloaded", d.id.trim_start_matches('0')))
        .collect();
    let drain = std::thread::spawn(move || stream_output(reader, output, expected));
    let status = child.wait();
    let active = state.lock().map_err(err)?.active.take().unwrap();
    let cancelled = active.cancelled;
    drop(active); // Closing ConPTY lets the output thread drain and reach EOF.
    let remaining = drain.join().map_err(|_| "Engine output reader stopped")?;
    if cancelled {
        return Ok("Download cancelled".into());
    }
    let status = status.map_err(err)?;
    if !status.success() {
        return Err(format!(
            "Engine exited with code {}. See download output.",
            status.exit_code()
        ));
    }
    if !remaining?.is_empty() {
        return Err(
            "The engine did not complete every requested depot. See download output.".into(),
        );
    }
    Ok("Download complete".into())
}

fn stream_output(
    mut reader: Box<dyn Read + Send>,
    output: Channel<Vec<u8>>,
    mut expected: HashSet<String>,
) -> Result<HashSet<String>, String> {
    let mut tail = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => break,
            #[cfg(unix)]
            Err(e) if e.raw_os_error() == Some(5) => break, // PTY slave closed (EIO).
            Err(e) => return Err(err(e)),
        };
        let _ = output.send(buffer[..n].to_vec());
        tail.extend_from_slice(&buffer[..n]);
        let text = String::from_utf8_lossy(&tail);
        expected.retain(|marker| !text.contains(marker));
        let keep = tail.len().saturating_sub(64);
        tail.drain(..keep);
    }
    Ok(expected)
}
