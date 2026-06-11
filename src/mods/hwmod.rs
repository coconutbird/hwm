//! HWMod format loader
//!
//! The hwmod format is the Firebase/HaloWarsModding mod format.
//! It consists of a manifest.xml file with mod metadata and content files.

use std::path::Path;

use serde::Deserialize;

use crate::mods::error::ModError;
use crate::mods::source::ModSource;

/// The manifest file name
const MANIFEST_FILE: &str = "manifest.xml";

/// A loaded hwmod
pub struct HwMod {
    /// The mod manifest containing metadata
    pub manifest: ModManifest,
    /// The source of the mod files
    pub source: ModSource,
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

    /// Get the mod ID
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

    /// Get the banner art relative path
    pub fn banner_path(&self) -> Option<&str> {
        self.manifest
            .optional
            .as_ref()
            .and_then(|o| o.banner.as_ref())
            .and_then(|b| b.relative_path.as_deref())
    }

    /// Get the icon relative path
    pub fn icon_path(&self) -> Option<&str> {
        self.manifest
            .optional
            .as_ref()
            .and_then(|o| o.icon.as_ref())
            .and_then(|i| i.relative_path.as_deref())
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

/// Load an hwmod from a path
pub fn load(path: &Path) -> Result<HwMod, ModError> {
    let source = ModSource::from_path(path)?;

    // Read the manifest file
    let manifest_bytes = source
        .read_file(MANIFEST_FILE)
        .map_err(|_| ModError::ManifestNotFound)?;

    // Parse the XML manifest
    let manifest: ModManifest = quick_xml::de::from_reader(manifest_bytes.as_slice())
        .map_err(|e| ModError::ManifestParse(e.to_string()))?;

    Ok(HwMod { manifest, source })
}

/// Parse a manifest from XML bytes (useful for testing)
pub fn parse_manifest(xml: &[u8]) -> Result<ModManifest, ModError> {
    quick_xml::de::from_reader(xml).map_err(|e| ModError::ManifestParse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

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
        let xml = r#"<HWMod></HWMod>"#;
        let result = parse_manifest(xml.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn load_from_directory() {
        let temp_dir = std::env::temp_dir().join("hwmod_test_dir");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("manifest.xml"), FULL_MANIFEST).unwrap();

        let hwmod = load(&temp_dir).unwrap();
        assert_eq!(hwmod.title(), "Full Test Mod");
        assert_eq!(hwmod.author(), "Test Author");
        assert_eq!(hwmod.version(), "2.0.0");
        assert_eq!(hwmod.mod_id(), Some("test.mod.full"));
        assert_eq!(hwmod.description(), Some("A fully featured test mod."));
        assert_eq!(hwmod.banner_path(), Some("art/banner.png"));
        assert_eq!(hwmod.icon_path(), Some("art/icon.png"));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn load_from_zip() {
        let temp_file = std::env::temp_dir().join("hwmod_test.zip");
        let _ = fs::remove_file(&temp_file);

        // Create a zip file with the manifest
        let file = fs::File::create(&temp_file).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file::<_, ()>("manifest.xml", Default::default())
            .unwrap();
        zip.write_all(MANIFEST_WITH_EMPTY_ART.as_bytes()).unwrap();
        zip.finish().unwrap();

        let hwmod = load(&temp_file).unwrap();
        assert_eq!(hwmod.title(), "Example Reskin Mod");
        assert_eq!(hwmod.author(), "Modder");
        assert_eq!(hwmod.version(), "1.0");
        assert!(hwmod.description().unwrap().contains("reskin"));

        fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn load_missing_manifest() {
        let temp_dir = std::env::temp_dir().join("hwmod_test_empty");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let result = load(&temp_dir);
        assert!(matches!(result, Err(ModError::ManifestNotFound)));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn load_nonexistent_path() {
        let result = load(Path::new("/nonexistent/path/to/mod"));
        assert!(matches!(result, Err(ModError::NotFound(_))));
    }

    #[test]
    fn hwmod_accessors_with_missing_optional() {
        let temp_dir = std::env::temp_dir().join("hwmod_test_minimal");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        fs::write(temp_dir.join("manifest.xml"), MINIMAL_MANIFEST).unwrap();

        let hwmod = load(&temp_dir).unwrap();
        assert_eq!(hwmod.title(), "Minimal Mod");
        assert_eq!(hwmod.author(), "Test");
        assert_eq!(hwmod.version(), "0.1");
        assert!(hwmod.mod_id().is_none());
        assert!(hwmod.description().is_none());
        assert!(hwmod.banner_path().is_none());
        assert!(hwmod.icon_path().is_none());

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
