//! HWMod format loader
//!
//! The hwmod format is the Firebase / HaloWarsModding mod format. A mod is a
//! directory containing a `.hwmod` XML manifest file alongside a sibling
//! `ModData` folder with the actual game content. Art paths in the manifest are
//! relative to the manifest's directory, and the manifest's `ModID` is an
//! uppercase SHA-256 of `<title-author-version>`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::mods::error::ModError;

/// Conventional name of the content folder beside a mod manifest
const MOD_DATA_DIR: &str = "ModData";

/// A loaded hwmod
pub struct HwMod {
    /// The mod manifest containing metadata
    pub manifest: ModManifest,
    /// Path to the `.hwmod` manifest file
    manifest_path: PathBuf,
    /// The mod root (the directory the manifest lives in)
    root: PathBuf,
}

impl HwMod {
    /// Get the mod title
    pub fn title(&self) -> &str {
        self.manifest.required.title.as_deref().unwrap_or("Unknown")
    }

    /// Get the mod author
    pub fn author(&self) -> &str {
        self.manifest
            .required
            .author
            .as_deref()
            .unwrap_or("Unknown")
    }

    /// Get the mod version
    pub fn version(&self) -> &str {
        self.manifest
            .required
            .version
            .as_deref()
            .unwrap_or("Unknown")
    }

    /// Get the mod ID as declared in the manifest
    pub fn mod_id(&self) -> Option<&str> {
        self.manifest.mod_id.as_deref()
    }

    /// Get the mod description
    pub fn description(&self) -> Option<&str> {
        self.manifest
            .optional
            .as_ref()
            .and_then(|o| o.description.as_ref())
            .and_then(|d| d.text.as_deref())
    }

    /// Get the banner art path, relative to the mod root
    pub fn banner_path(&self) -> Option<&str> {
        self.manifest
            .optional
            .as_ref()
            .and_then(|o| o.banner.as_ref())
            .and_then(|b| b.relative_path.as_deref())
    }

    /// Get the icon path, relative to the mod root
    pub fn icon_path(&self) -> Option<&str> {
        self.manifest
            .optional
            .as_ref()
            .and_then(|o| o.icon.as_ref())
            .and_then(|i| i.relative_path.as_deref())
    }

    /// Path to the `.hwmod` manifest file
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// The mod root directory (where the manifest lives)
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `ModData` content folder for this mod
    pub fn mod_data_dir(&self) -> PathBuf {
        self.root.join(MOD_DATA_DIR)
    }

    /// Whether the `ModData` content folder exists on disk
    pub fn has_mod_data(&self) -> bool {
        self.mod_data_dir().is_dir()
    }

    /// Resolve the banner art to an absolute path, if declared and present
    pub fn banner_file(&self) -> Option<PathBuf> {
        self.resolve(self.banner_path())
    }

    /// Resolve the icon to an absolute path, if declared and present
    pub fn icon_file(&self) -> Option<PathBuf> {
        self.resolve(self.icon_path())
    }

    /// The ModID computed from title/author/version, using Firebase's scheme:
    /// the uppercase SHA-256 hex of `<title-author-version>`.
    pub fn computed_mod_id(&self) -> String {
        compute_mod_id(&self.manifest.required)
    }

    /// Whether the declared ModID matches the computed one (Firebase validity).
    pub fn is_valid(&self) -> bool {
        self.manifest
            .mod_id
            .as_deref()
            .is_some_and(|id| id.eq_ignore_ascii_case(&self.computed_mod_id()))
    }

    /// Resolve a manifest-relative path against the mod root, rejecting paths
    /// that escape the root and returning `None` if the file is absent.
    fn resolve(&self, relative: Option<&str>) -> Option<PathBuf> {
        let relative = relative?;
        if !is_safe_relative_path(relative) {
            return None;
        }
        let full = self.root.join(relative.replace('\\', "/"));
        full.exists().then_some(full)
    }
}

/// HWMod manifest structure matching the Firebase format
#[derive(Debug, Deserialize)]
#[serde(rename = "HWMod")]
pub struct ModManifest {
    /// Unique mod identifier
    #[serde(rename = "@ModID")]
    pub mod_id: Option<String>,

    /// Required mod metadata
    #[serde(rename = "RequiredData")]
    pub required: RequiredData,

    /// Optional mod metadata
    #[serde(rename = "OptionalData")]
    pub optional: Option<OptionalData>,
}

/// Required mod metadata
#[derive(Debug, Deserialize)]
pub struct RequiredData {
    /// Mod title/name
    #[serde(rename = "@Title")]
    pub title: Option<String>,

    /// Mod author
    #[serde(rename = "@Author")]
    pub author: Option<String>,

    /// Mod version string
    #[serde(rename = "@Version")]
    pub version: Option<String>,
}

/// Optional mod metadata
#[derive(Debug, Deserialize)]
pub struct OptionalData {
    /// Banner art path
    #[serde(rename = "BannerArt")]
    pub banner: Option<ArtPath>,

    /// Icon path
    #[serde(rename = "Icon")]
    pub icon: Option<ArtPath>,

    /// Mod description
    #[serde(rename = "Description")]
    pub description: Option<Description>,
}

/// Art path container
#[derive(Debug, Deserialize)]
pub struct ArtPath {
    /// Relative path to the art file
    #[serde(rename = "$text")]
    pub relative_path: Option<String>,
}

/// Description container
#[derive(Debug, Deserialize)]
pub struct Description {
    /// Description text
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

/// Load an hwmod from a path: either a `.hwmod` manifest file, or a directory
/// containing one (searched recursively, files before subdirectories).
pub fn load(path: &Path) -> Result<HwMod, ModError> {
    if !path.exists() {
        return Err(ModError::NotFound(path.to_path_buf()));
    }

    let manifest_path = if path.is_dir() {
        find_manifest(path)?.ok_or_else(|| ModError::ManifestNotFound(path.to_path_buf()))?
    } else {
        path.to_path_buf()
    };

    let bytes = fs::read(&manifest_path).map_err(|e| ModError::Io(e, manifest_path.clone()))?;
    let manifest = parse_manifest(&bytes)?;

    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    Ok(HwMod {
        manifest,
        manifest_path,
        root,
    })
}

/// Parse a manifest from XML bytes
pub fn parse_manifest(xml: &[u8]) -> Result<ModManifest, ModError> {
    quick_xml::de::from_reader(xml).map_err(|e| ModError::ManifestParse(e.to_string()))
}

/// Find the first `.hwmod` file under `dir`, preferring files directly in the
/// directory over ones nested in subdirectories. Entries are visited in sorted
/// order so the result is deterministic.
fn find_manifest(dir: &Path) -> Result<Option<PathBuf>, ModError> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| ModError::Io(e, dir.to_path_buf()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();

    for entry in &entries {
        if entry.is_file() && has_hwmod_extension(entry) {
            return Ok(Some(entry.clone()));
        }
    }
    for entry in &entries {
        if entry.is_dir()
            && let Some(found) = find_manifest(entry)?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// Whether a path has a `.hwmod` extension (case-insensitive).
fn has_hwmod_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("hwmod"))
}

/// Compute Firebase's ModID: the uppercase SHA-256 hex of
/// `<title-author-version>`, using empty strings for any missing field.
fn compute_mod_id(required: &RequiredData) -> String {
    let data = format!(
        "<{}-{}-{}>",
        required.title.as_deref().unwrap_or(""),
        required.author.as_deref().unwrap_or(""),
        required.version.as_deref().unwrap_or(""),
    );
    Sha256::digest(data.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

/// Reject absolute paths and any `..` component so a mod-supplied relative path
/// can't escape the mod root (path traversal).
fn is_safe_relative_path(relative_path: &str) -> bool {
    let normalized = relative_path.replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || Path::new(&normalized).is_absolute()
    {
        return false;
    }
    !normalized.split('/').any(|component| component == "..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A temp directory removed on drop, so a panicking test never leaks state.
    /// Names are unique per process + call to avoid collisions between tests
    /// (including concurrent test binaries).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "hwmod_test_{}_{}_{}",
                label,
                std::process::id(),
                n
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const MANIFEST_WITH_EMPTY_ART: &str = r#"<HWMod ManifestVersion="1" ModID="47473A9018E7DC39A95AEF8CC95CC891F6D9F5A61FFE2D6DAA36157E8B25105F">
  <RequiredData Title="Example Reskin Mod" Author="Modder" Version="1.0" />
  <OptionalData>
    <BannerArt></BannerArt>
    <Icon></Icon>
    <Description>This mod is a reskin that modifies maps and lighting. No new units or upgrades are added. The campaign has also been modified but music is untouched to preserve mission scripts.</Description>
  </OptionalData>
</HWMod>"#;

    const MINIMAL_MANIFEST: &str = r#"<HWMod>
  <RequiredData Title="Minimal Mod" Author="Test" Version="0.1" />
</HWMod>"#;

    const FULL_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<HWMod ManifestVersion="1" ModID="test.mod.full">
    <RequiredData Title="Full Test Mod" Author="Test Author" Version="2.0.0" />
    <OptionalData>
        <BannerArt>art/banner.png</BannerArt>
        <Icon>art/icon.png</Icon>
        <Description>A fully featured test mod.</Description>
    </OptionalData>
</HWMod>"#;

    #[test]
    fn parse_manifest_with_empty_art_paths() {
        let manifest = parse_manifest(MANIFEST_WITH_EMPTY_ART.as_bytes()).unwrap();

        assert_eq!(
            manifest.mod_id.as_deref(),
            Some("47473A9018E7DC39A95AEF8CC95CC891F6D9F5A61FFE2D6DAA36157E8B25105F")
        );
        assert_eq!(
            manifest.required.title.as_deref(),
            Some("Example Reskin Mod")
        );
        assert_eq!(manifest.required.author.as_deref(), Some("Modder"));
        assert_eq!(manifest.required.version.as_deref(), Some("1.0"));

        let optional = manifest.optional.as_ref().unwrap();
        // Empty elements should parse as None for the text content
        assert!(optional.banner.as_ref().unwrap().relative_path.is_none());
        assert!(optional.icon.as_ref().unwrap().relative_path.is_none());

        let description = optional
            .description
            .as_ref()
            .unwrap()
            .text
            .as_deref()
            .unwrap();
        assert!(description.contains("reskin"));
        assert!(description.contains("mission scripts"));
    }

    #[test]
    fn parse_minimal_manifest() {
        let manifest = parse_manifest(MINIMAL_MANIFEST.as_bytes()).unwrap();

        assert!(manifest.mod_id.is_none());
        assert_eq!(manifest.required.title.as_deref(), Some("Minimal Mod"));
        assert_eq!(manifest.required.author.as_deref(), Some("Test"));
        assert_eq!(manifest.required.version.as_deref(), Some("0.1"));
        assert!(manifest.optional.is_none());
    }

    #[test]
    fn parse_full_manifest() {
        let manifest = parse_manifest(FULL_MANIFEST.as_bytes()).unwrap();

        assert_eq!(manifest.mod_id.as_deref(), Some("test.mod.full"));
        assert_eq!(manifest.required.title.as_deref(), Some("Full Test Mod"));
        assert_eq!(manifest.required.author.as_deref(), Some("Test Author"));
        assert_eq!(manifest.required.version.as_deref(), Some("2.0.0"));

        let optional = manifest.optional.as_ref().unwrap();
        assert_eq!(
            optional.banner.as_ref().unwrap().relative_path.as_deref(),
            Some("art/banner.png")
        );
        assert_eq!(
            optional.icon.as_ref().unwrap().relative_path.as_deref(),
            Some("art/icon.png")
        );
        assert_eq!(
            optional.description.as_ref().unwrap().text.as_deref(),
            Some("A fully featured test mod.")
        );
    }

    #[test]
    fn parse_invalid_xml() {
        let result = parse_manifest(b"not xml at all");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_required_data() {
        let result = parse_manifest(b"<HWMod></HWMod>");
        assert!(result.is_err());
    }

    #[test]
    fn computes_firebase_mod_id() {
        // Expected value computed independently with sha256sum over the exact
        // string Firebase hashes: "<Example Reskin Mod-Modder-1.0>".
        let required = RequiredData {
            title: Some("Example Reskin Mod".to_string()),
            author: Some("Modder".to_string()),
            version: Some("1.0".to_string()),
        };
        assert_eq!(
            compute_mod_id(&required),
            "3C919E70C74967AE19DC62B23F1375DF12E724F3045EE47DF6C078F46D1DEAC8"
        );
    }

    #[test]
    fn load_hwmod_file_directly() {
        let dir = TempDir::new("file");
        let manifest = dir.path().join("Full Test Mod v2.0.0.hwmod");
        fs::write(&manifest, FULL_MANIFEST).unwrap();
        fs::create_dir_all(dir.path().join(MOD_DATA_DIR)).unwrap();

        let hwmod = load(&manifest).unwrap();
        assert_eq!(hwmod.title(), "Full Test Mod");
        assert_eq!(hwmod.author(), "Test Author");
        assert_eq!(hwmod.version(), "2.0.0");
        assert_eq!(hwmod.manifest_path(), manifest);
        assert_eq!(hwmod.root(), dir.path());
        assert_eq!(hwmod.mod_data_dir(), dir.path().join(MOD_DATA_DIR));
        assert!(hwmod.has_mod_data());
    }

    #[test]
    fn load_finds_manifest_in_directory() {
        let dir = TempDir::new("dir");
        fs::write(dir.path().join("MyMod v1.hwmod"), FULL_MANIFEST).unwrap();

        let hwmod = load(dir.path()).unwrap();
        assert_eq!(hwmod.title(), "Full Test Mod");
        // No ModData folder was created.
        assert!(!hwmod.has_mod_data());
    }

    #[test]
    fn load_finds_manifest_in_subdirectory() {
        let dir = TempDir::new("nested");
        let sub = dir.path().join("inner");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("Nested.hwmod"), MINIMAL_MANIFEST).unwrap();

        let hwmod = load(dir.path()).unwrap();
        assert_eq!(hwmod.title(), "Minimal Mod");
        assert_eq!(hwmod.root(), sub);
    }

    #[test]
    fn load_missing_manifest() {
        let dir = TempDir::new("empty");
        let result = load(dir.path());
        assert!(matches!(result, Err(ModError::ManifestNotFound(_))));
    }

    #[test]
    fn load_nonexistent_path() {
        let result = load(Path::new("/nonexistent/path/to/mod"));
        assert!(matches!(result, Err(ModError::NotFound(_))));
    }

    #[test]
    fn resolves_art_and_validates_matching_id() {
        let dir = TempDir::new("art");
        // ModID is the genuine SHA-256 for this title/author/version.
        let manifest = r#"<HWMod ModID="3C919E70C74967AE19DC62B23F1375DF12E724F3045EE47DF6C078F46D1DEAC8">
  <RequiredData Title="Example Reskin Mod" Author="Modder" Version="1.0" />
  <OptionalData>
    <Icon>art/icon.png</Icon>
  </OptionalData>
</HWMod>"#;
        fs::write(dir.path().join("mod.hwmod"), manifest).unwrap();
        fs::create_dir_all(dir.path().join("art")).unwrap();
        fs::write(dir.path().join("art").join("icon.png"), b"fake").unwrap();

        let hwmod = load(dir.path()).unwrap();
        assert!(hwmod.is_valid());
        assert_eq!(
            hwmod.icon_file(),
            Some(dir.path().join("art").join("icon.png"))
        );
        // Banner not declared.
        assert!(hwmod.banner_file().is_none());
    }

    #[test]
    fn rejects_mismatched_id_and_unsafe_art() {
        let dir = TempDir::new("bad");
        let manifest = r#"<HWMod ModID="DEADBEEF">
  <RequiredData Title="X" Author="Y" Version="1" />
  <OptionalData>
    <Icon>../escape.png</Icon>
  </OptionalData>
</HWMod>"#;
        fs::write(dir.path().join("mod.hwmod"), manifest).unwrap();

        let hwmod = load(dir.path()).unwrap();
        assert!(!hwmod.is_valid());
        // A traversal path must never resolve, even to an existing file.
        assert!(hwmod.icon_file().is_none());
    }

    #[test]
    fn safe_relative_paths() {
        assert!(is_safe_relative_path("manifest.xml"));
        assert!(is_safe_relative_path("art/banner.png"));
        assert!(is_safe_relative_path("art\\icon.png"));
        assert!(!is_safe_relative_path(""));
        assert!(!is_safe_relative_path("../secret"));
        assert!(!is_safe_relative_path("art/../../secret"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }
}
