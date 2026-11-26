use crate::import::{Depot, read_import, valid_id};
use portable_pty::CommandBuilder;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tempfile::TempPath;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamDownload {
    username: Option<String>,
    password: Option<String>,
    app_id: String,
    pub depots: Vec<Depot>,
    branch: Option<String>,
    output_location: PathBuf,
    output_directory_name: Option<String>,
    import_source: Option<PathBuf>,
    no_mobile_auth: bool,
}

impl SteamDownload {
    pub fn imported_key_file(&self) -> Result<Option<TempPath>, String> {
        let Some(source) = &self.import_source else {
            return Ok(None);
        };
        let input = read_import(source)?;
        if input.app_id != self.app_id || input.depots != self.depots {
            return Err("The import changed. Select the Lua or ZIP file again.".into());
        }
        if input.depot_keys.is_empty() {
            return Ok(None);
        }

        let file_error = |_| "Could not prepare the temporary depot key file".to_owned();
        let mut file = tempfile::NamedTempFile::new().map_err(file_error)?;
        for (id, key) in input.depot_keys {
            writeln!(file, "{id};{key}").map_err(file_error)?;
        }
        // Close the writer so the engine can open the file on Windows.
        Ok(Some(file.into_temp_path()))
    }

    pub fn command(
        &self,
        engine: &Path,
        cwd: &Path,
        keys: Option<&Path>,
    ) -> Result<CommandBuilder, String> {
        if !self.output_location.is_absolute() || !self.output_location.is_dir() {
            return Err("Choose an existing destination folder".into());
        }
        let mut dest = self.output_location.clone();
        if self.depots.is_empty() {
            return Err("Missing depot and manifest IDs".into());
        }
        let mut ids = HashSet::new();
        for d in &self.depots {
            valid_id(&d.manifest, u64::MAX)?;
            if !ids.insert(valid_id(&d.id, u32::MAX as u64)?) {
                return Err("Duplicate depot ID".into());
            }
        }
        if self.import_source.is_none() {
            let name = self
                .output_directory_name
                .as_deref()
                .unwrap_or(&self.depots[0].manifest);
            if !name.is_empty() {
                if Path::new(name)
                    .components()
                    .any(|p| !matches!(p, Component::Normal(_) | Component::CurDir))
                {
                    return Err(
                        "Custom output folder must be inside the selected destination".into(),
                    );
                }
                dest.push(name);
            }
        }

        let mut cmd = CommandBuilder::new(engine);
        cmd.cwd(cwd);
        cmd.env(
            "TERM",
            if cfg!(windows) {
                "cygwin"
            } else {
                "xterm-256color"
            },
        );
        let username = self.username.as_deref().filter(|s| !s.trim().is_empty());
        let password = self.password.as_deref().filter(|s| !s.is_empty());
        let branch = self.branch.as_deref().filter(|s| !s.trim().is_empty());
        if password.is_some() && username.is_none() {
            return Err("Enter the Steam username for this password".into());
        }
        // Upstream scans the entire argv for options, including option values.
        if [username, branch, password]
            .into_iter()
            .flatten()
            .any(|s| s.starts_with('-'))
        {
            return Err("Engine option values cannot begin with a dash. A password can also be entered in the terminal.".into());
        }
        if let Some(user) = username {
            cmd.args(["-username", user]);
        }
        if let Some(pass) = password {
            cmd.args(["-password", pass]);
        }
        if self.no_mobile_auth {
            cmd.arg("-no-mobile");
        }
        if let Some(b) = branch {
            cmd.args(["-branch", b]);
        }
        cmd.args(["-app", &valid_id(&self.app_id, u32::MAX as u64)?]);
        cmd.arg("-depot");
        cmd.args(self.depots.iter().map(|d| &d.id));
        cmd.arg("-manifest");
        cmd.args(self.depots.iter().map(|d| &d.manifest));
        cmd.arg("-dir");
        cmd.arg(dest);
        if self.import_source.is_some() {
            cmd.arg("-validate");
        }
        if let Some(keys) = keys {
            cmd.arg("-verified-depotkeys");
            cmd.arg(keys);
        }
        Ok(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_and_imported_jobs_use_their_respective_destination() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let mut job = SteamDownload {
            username: Some("test-user".into()),
            password: None,
            app_id: "90".into(),
            depots: vec![Depot {
                id: "5".into(),
                manifest: "123".into(),
            }],
            branch: None,
            output_location: path.to_owned(),
            output_directory_name: None,
            import_source: None,
            no_mobile_auth: true,
        };
        let engine = path.join("DepotDownloaderMod");
        let command = job.command(&engine, path, None).unwrap();
        assert_eq!(
            command.get_argv().last().unwrap(),
            &path.join("123").into_os_string()
        );
        job.import_source = Some(path.join("90.lua"));
        job.depots.push(Depot {
            id: "6".into(),
            manifest: "9007199254740993".into(),
        });
        let command = job.command(&engine, path, None).unwrap();
        let args: Vec<_> = command
            .get_argv()
            .iter()
            .skip(1)
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(
            args,
            [
                "-username",
                "test-user",
                "-no-mobile",
                "-app",
                "90",
                "-depot",
                "5",
                "6",
                "-manifest",
                "123",
                "9007199254740993",
                "-dir",
                path.to_str().unwrap(),
                "-validate"
            ]
        );
        let source = job.import_source.as_ref().unwrap();
        let key = "ab".repeat(32);
        std::fs::write(
            source,
            format!("addappid(90)\naddappid(5, 1, '{key}')\nsetmanifestid(5, 123)\nsetmanifestid(6, 9007199254740993)"),
        )
        .unwrap();
        let keys = job.imported_key_file().unwrap().unwrap();
        let keys_path = keys.to_path_buf();
        assert_eq!(
            std::fs::read_to_string(&keys_path).unwrap(),
            format!("5;{key}\n")
        );
        let command = job.command(&engine, path, Some(&keys_path)).unwrap();
        let args = command.get_argv();
        assert_eq!(args[args.len() - 2], "-verified-depotkeys");
        assert_eq!(args[args.len() - 1].as_os_str(), keys_path.as_os_str());
        drop(keys);
        assert!(!keys_path.exists());
        std::fs::write(source, "addappid(90)\nsetmanifestid(5, 456)").unwrap();
        assert!(job.imported_key_file().is_err());

        job.branch = Some("-verified-depotkeys".into());
        assert!(job.command(&engine, path, None).is_err());
    }
}
