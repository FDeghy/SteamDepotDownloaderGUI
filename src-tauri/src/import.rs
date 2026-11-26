use prost::Message;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const LUA_LIMIT: u64 = 1024 * 1024;
const MANIFEST_LIMIT: u64 = 64 * 1024 * 1024;
const ZIP_LIMIT: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Depot {
    pub id: String,
    pub manifest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    pub app_id: String,
    pub depots: Vec<Depot>,
    #[serde(skip_serializing)]
    pub depot_keys: HashMap<String, String>,
}

type Result<T> = std::result::Result<T, String>;

pub fn valid_id(value: &str, max: u64) -> Result<String> {
    if value.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(id) = value.parse::<u64>() {
            if (1..max).contains(&id) {
                return Ok(id.to_string());
            }
        }
    }
    Err(format!("Invalid Steam ID: {value}"))
}

fn insert(depots: &mut Vec<Depot>, depot: Depot) -> Result<()> {
    if depots.len() == 4096 {
        return Err("Too many depot declarations".into());
    }
    if depots.iter().any(|other| other.id == depot.id) {
        return Err(format!(
            "Multiple manifests specified for depot {}",
            depot.id
        ));
    }
    depots.push(depot);
    Ok(())
}

fn numeric_stem(path: &Path) -> Result<Option<String>> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()) {
        return valid_id(stem, u32::MAX as u64).map(Some);
    }
    Ok(None)
}

#[derive(Default)]
struct Declarations {
    apps: Vec<(String, bool)>,
    depots: Vec<Depot>,
    depot_keys: Vec<(String, String)>,
}

fn read_lua(path: &Path) -> Result<Declarations> {
    static APP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?ix)^add(appid|token)\s*\(\s*([0-9]+)\s*(?:,\s*[0-9]+\s*,\s*("[a-z0-9]+"|'[a-z0-9]+')\s*)?\)"#
    ).unwrap()
    });
    static MANIFEST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
        r#"(?ix)^setmanifestid\s*\(\s*([0-9]+)\s*,\s*("[0-9]+"|'[0-9]+'|[0-9]+)\s*(?:,\s*(?:"[0-9]+"|'[0-9]+'|[0-9]+)\s*)?\)"#
    ).unwrap()
    });
    let mut data = String::new();
    File::open(path)
        .map_err(err)?
        .take(LUA_LIMIT + 1)
        .read_to_string(&mut data)
        .map_err(err)?;
    if data.len() as u64 > LUA_LIMIT {
        return Err("Lua file exceeds 1 MiB".into());
    }
    let mut result = Declarations::default();
    let mut rest = data.trim_start_matches('\u{feff}');
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ';');
        if rest.is_empty() {
            break;
        }
        if let Some(comment) = rest.strip_prefix("--") {
            if comment
                .strip_prefix('[')
                .is_some_and(|s| s.trim_start_matches('=').starts_with('['))
            {
                return Err("Unsupported Lua block comment".into());
            }
            rest = comment
                .split_once(['\r', '\n'])
                .map_or("", |(_, next)| next);
        } else if let Some(m) = APP.captures(rest) {
            if result.apps.len() == 4096 {
                return Err("Too many app declarations".into());
            }
            let id = valid_id(&m[2], u32::MAX as u64)?;
            let value = m.get(3);
            if let Some(key) = value.as_ref().filter(|_| m[1].eq_ignore_ascii_case("appid")) {
                let key = key.as_str().trim_matches(['"', '\'']).to_owned();
                result.depot_keys.push((id.clone(), key));
            }
            result.apps.push((id, value.is_some()));
            rest = &rest[m[0].len()..];
        } else if let Some(m) = MANIFEST.captures(rest) {
            insert(
                &mut result.depots,
                Depot {
                    id: valid_id(&m[1], u32::MAX as u64)?,
                    manifest: valid_id(m[2].trim_matches(['"', '\'']), u64::MAX)?,
                },
            )?;
            rest = &rest[m[0].len()..];
        } else {
            let line = data[..data.len() - rest.len()]
                .bytes()
                .filter(|b| *b == b'\n')
                .count()
                + 1;
            return Err(format!(
                "Unsupported or malformed Lua declaration on line {line}"
            ));
        }
    }
    Ok(result)
}

impl Declarations {
    fn resolve(self, hint: Option<String>, manifests: Vec<Depot>) -> Result<Import> {
        let depots = if self.depots.is_empty() {
            manifests
        } else {
            if !manifests.is_empty()
                && (manifests.len() != self.depots.len()
                    || !manifests.iter().all(|d| self.depots.contains(d)))
            {
                return Err("Archive manifests and Lua declarations disagree".into());
            }
            self.depots
        };
        if depots.is_empty() {
            return Err("Missing depot and manifest IDs".into());
        }
        let candidates: HashSet<_> = self
            .apps
            .iter()
            .filter(|(id, key)| !key && !depots.iter().any(|d| &d.id == id))
            .map(|(id, _)| id.clone())
            .collect();
        let app_id = match hint {
            Some(id) => id,
            None if candidates.len() == 1 => candidates.into_iter().next().unwrap(),
            _ => {
                return Err(
                    "Ambiguous or missing App ID. Use an <AppID>.lua or <AppID>.zip filename."
                        .into(),
                );
            }
        };
        if self
            .apps
            .iter()
            .any(|(id, _)| id != &app_id && !depots.iter().any(|d| &d.id == id))
        {
            return Err("Lua contains another app or a depot without a manifest".into());
        }
        let mut depot_keys = HashMap::new();
        for (id, key) in self.depot_keys {
            if !depots.iter().any(|depot| depot.id == id) {
                continue;
            }
            if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(format!("Depot {id} key must contain 64 hexadecimal digits"));
            }
            if depot_keys.insert(id.clone(), key).is_some() {
                return Err(format!("Multiple keys specified for depot {id}"));
            }
        }
        Ok(Import {
            app_id,
            depots,
            depot_keys,
        })
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// SteamKit2's protobuf manifest envelope. Only metadata is used for downloads.
// Filenames, file contents and signatures in imported files are never trusted.
#[derive(Message)]
struct Metadata {
    #[prost(uint32, tag = "1")]
    depot: u32,
    #[prost(uint64, tag = "2")]
    manifest: u64,
}
#[derive(Message)]
struct OtherSection {}

fn manifest_ids(data: &[u8]) -> Result<Depot> {
    let mut input = data;
    let mut ids = None;
    for expected in [0x71F617D0u32, 0x1F4812BE, 0x1B81B817] {
        let header = input.get(..8).ok_or("Truncated Steam manifest header")?;
        if header[..4] != expected.to_le_bytes() {
            return Err("Invalid Steam manifest section".into());
        }
        let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let section = input[8..]
            .get(..length)
            .ok_or("Truncated Steam manifest section")?;
        if expected == 0x1F4812BE {
            let m = Metadata::decode(section).map_err(err)?;
            ids = Some(Depot {
                id: valid_id(&m.depot.to_string(), u32::MAX as u64)?,
                manifest: valid_id(&m.manifest.to_string(), u64::MAX)?,
            });
        } else {
            OtherSection::decode(section).map_err(err)?;
        }
        input = &input[8 + length..];
    }
    if input != 0x32C415ABu32.to_le_bytes() {
        return Err("Invalid end of Steam manifest".into());
    }
    Ok(ids.unwrap())
}

fn read_zip(path: &Path) -> Result<Import> {
    let file = File::open(path).map_err(err)?;
    if file.metadata().map_err(err)?.len() > ZIP_LIMIT {
        return Err("ZIP exceeds 256 MiB".into());
    }
    let mut zip = zip::ZipArchive::new(file).map_err(err)?;
    if zip.len() > 4096 {
        return Err("ZIP contains too many entries".into());
    }
    let temp = tempfile::tempdir().map_err(err)?;
    let mut seen = HashSet::new();
    let mut lua = None;
    let mut hint = numeric_stem(path)?;
    let mut manifests = Vec::new();
    let mut remaining = ZIP_LIMIT;
    for index in 0..zip.len() {
        let mut item = zip.by_index(index).map_err(err)?;
        let entry_name = item.name().map_err(err)?;
        let name = entry_name.trim_end_matches('/');
        if item.enclosed_name().is_none()
            || name.contains(['\\', ':', '\0'])
            || name.split('/').any(|s| matches!(s, "" | "." | ".."))
            || !matches!(
                item.unix_mode().unwrap_or(0) & 0o170000,
                0 | 0o100000 | 0o040000
            )
            || !seen.insert(name.to_lowercase())
        {
            return Err("Unsafe, special, or duplicate ZIP entry".into());
        }
        if item.is_dir() {
            continue;
        }
        let member = PathBuf::from(name);
        let ext = member
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let limit = match ext.as_str() {
            "lua" => LUA_LIMIT,
            "manifest" => MANIFEST_LIMIT,
            _ => continue,
        }
        .min(remaining);
        let size = item.size();
        if size > limit {
            return Err("ZIP extraction size limit exceeded".into());
        }
        // Generated names keep archive paths out of all filesystem operations.
        let out = temp.path().join(index.to_string());
        let count = io::copy(
            &mut (&mut item).take(size + 1),
            &mut File::create_new(&out).map_err(err)?,
        )
        .map_err(err)?;
        if count != size {
            return Err("ZIP entry size mismatch".into());
        }
        remaining -= count;
        if ext == "lua" {
            if lua.is_some() {
                return Err("Multiple Lua files in archive".into());
            }
            if let Some(id) = numeric_stem(&member)? {
                if hint.as_ref().is_some_and(|h| h != &id) {
                    return Err("Conflicting App IDs in filenames".into());
                }
                hint = Some(id);
            }
            lua = Some(read_lua(&out)?);
        } else {
            let depot = manifest_ids(&fs::read(out).map_err(err)?)?;
            if member.file_stem().and_then(|s| s.to_str())
                != Some(&format!("{}_{}", depot.id, depot.manifest))
            {
                return Err(
                    "Manifest filename must match its embedded depot and manifest IDs".into(),
                );
            }
            insert(&mut manifests, depot)?;
        }
    }
    lua.unwrap_or_default().resolve(hint, manifests)
}

pub fn read_import(path: &Path) -> Result<Import> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "lua" => read_lua(path)?.resolve(numeric_stem(path)?, vec![]),
        "zip" => read_zip(path),
        _ => Err("Select a .lua or .zip file".into()),
    }
}

#[tauri::command]
pub async fn import_download(path: PathBuf) -> Result<Import> {
    tauri::async_runtime::spawn_blocking(move || read_import(&path))
        .await
        .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn lua(text: &str) -> Result<Import> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.lua");
        fs::write(&path, text).unwrap();
        read_lua(&path)?.resolve(None, vec![])
    }

    #[test]
    fn parses_declarations_without_executing_lua() {
        let input = lua("addappid(90) -- game\naddtoken(5, 1, 'ignored')\nsetManifestId(5, '9007199254740993', 0)").unwrap();
        assert_eq!(input.app_id, "90");
        assert_eq!(
            lua("addappid(90); setmanifestid(\n5, '319')")
                .unwrap()
                .depots[0]
                .manifest,
            "319"
        );
        assert_eq!(input.depots[0].manifest, "9007199254740993");
        for source in [
            "addappid(1)\nprint(42)",
            "addappid(1)",
            "addappid(1)\nsetmanifestid(2,'3)",
            "addappid(1)\naddappid(2)\nsetmanifestid(3,4)",
            "addappid(1)\nsetmanifestid(2,3)\nsetmanifestid(2,4)",
            "--[[\naddappid(1)\n--]]\nsetmanifestid(2,3)",
            "addappid(1)\nsetmanifestid(2,18446744073709551615)",
        ] {
            assert!(lua(source).is_err(), "{source}");
        }
    }

    #[test]
    fn imports_keys_without_exposing_them_in_the_preview() {
        let key = "aBcD".repeat(16);
        let source = format!("addappid(90)\naddappid(5, 1, '{key}')\nsetmanifestid(5, 319)");
        let input = lua(&source).unwrap();
        assert_eq!(input.depot_keys["5"], key);
        assert!(!serde_json::to_string(&input).unwrap().contains(&key));
        assert!(lua(&source.replace(&key, "nothex")).is_err());
        assert!(lua(&format!("{source}\naddappid(5, 1, '{key}')")).is_err());

        let bytes = manifest();
        let lua_entry = ("90.lua", source.as_bytes());
        for entries in [
            &[lua_entry][..],
            &[lua_entry, ("5_319.manifest", bytes.as_slice())][..],
        ] {
            assert_eq!(archive("90.zip", entries).unwrap().depot_keys["5"], key);
        }
    }

    fn manifest() -> Vec<u8> {
        let meta = Metadata {
            depot: 5,
            manifest: 319,
        }
        .encode_to_vec();
        let mut result = Vec::new();
        for (magic, data) in [
            (0x71F617D0u32, vec![]),
            (0x1F4812BE, meta),
            (0x1B81B817, vec![]),
        ] {
            result.extend(magic.to_le_bytes());
            result.extend((data.len() as u32).to_le_bytes());
            result.extend(data);
        }
        result.extend(0x32C415ABu32.to_le_bytes());
        result
    }

    fn archive(name: &str, files: &[(&str, &[u8])]) -> Result<Import> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        for (member, data) in files {
            zip.start_file(*member, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        read_zip(&path)
    }

    #[test]
    fn checks_manifest_contents_and_archive_identity() {
        let bytes = manifest();
        let entry = ("nested/5_319.manifest", bytes.as_slice());
        assert_eq!(archive("90.zip", &[entry]).unwrap().app_id, "90");
        assert_eq!(
            archive("game.zip", &[entry, ("90.lua", b"addappid(90)")])
                .unwrap()
                .depots[0]
                .id,
            "5"
        );
        assert!(archive("game.zip", &[entry]).is_err());
        assert!(archive("90.zip", &[entry, ("../escape", b"")]).is_err());
        assert!(archive("90.zip", &[entry, ("91.lua", b"addappid(91)")]).is_err());
        assert!(archive("90.zip", &[("5_320.manifest", &bytes)]).is_err());
        assert!(archive("90.zip", &[("5_319.manifest", &bytes[..12])]).is_err());
    }
}
