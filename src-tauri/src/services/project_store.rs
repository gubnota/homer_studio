use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 1;
const MANIFEST: &str = "project.json";
pub const MAX_MANUSCRIPT_BYTES: usize = 20 * 1024 * 1024;
const MAX_SEGMENT_CHARS: usize = 1_600;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL",
            message: message.into(),
        }
    }

    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn io(action: &str, error: std::io::Error) -> Self {
        Self::new("IO_ERROR", format!("{action}: {error}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub order: usize,
    pub text: String,
    pub selected_take: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub order: usize,
    pub source_path: String,
    pub processed_path: Option<String>,
    pub segments: Vec<Segment>,
    pub audio_path: Option<String>,
    pub audio_stale: bool,
    #[serde(default)]
    pub audio_duration_ms: Option<u64>,
    #[serde(default)]
    pub audio_origin: Option<String>,
    #[serde(default)]
    pub review_status: Option<String>,
    #[serde(default)]
    pub cues: Vec<LineCue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineCue {
    pub order: usize,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRecord {
    pub id: String,
    pub audio_path: String,
    pub timestamps_path: String,
    pub duration_ms: u64,
    pub created_at_ms: u64,
    #[serde(default)]
    pub source_revision: u64,
    #[serde(default)]
    pub source_updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub exports: Vec<ExportRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSnapshot {
    #[serde(flatten)]
    pub chapter: Chapter,
    pub source_text: String,
    pub processed_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub root_path: String,
    pub chapters: Vec<ChapterSnapshot>,
    pub exports: Vec<ExportRecord>,
}

#[derive(Debug)]
struct ParsedChapter {
    title: String,
    text: String,
}

pub fn create(
    parent_path: &str,
    title: &str,
    manuscript: &str,
) -> Result<ProjectSnapshot, CommandError> {
    let title = clean_title(title)?;
    validate_manuscript(manuscript)?;
    let parent = fs::canonicalize(parent_path)
        .map_err(|error| CommandError::io("Cannot open the selected parent folder", error))?;
    if !parent.is_dir() {
        return Err(CommandError::new(
            "INVALID_PATH",
            "The selected project location is not a folder.",
        ));
    }
    let root = parent.join(project_folder_name(&title));
    if root.exists() {
        return Err(CommandError::new(
            "PROJECT_EXISTS",
            format!("A folder named '{}' already exists.", root.display()),
        ));
    }

    fs::create_dir(&root)
        .map_err(|error| CommandError::io("Cannot create project folder", error))?;
    let result = (|| {
        for directory in ["chapters", "output", ".work"] {
            fs::create_dir(root.join(directory))
                .map_err(|error| CommandError::io("Cannot create project subfolder", error))?;
        }
        let now = now_ms();
        let chapters = parse_chapters(manuscript)
            .into_iter()
            .enumerate()
            .map(|(order, parsed)| write_new_chapter(&root, order, parsed))
            .collect::<Result<Vec<_>, _>>()?;
        let manifest = ProjectManifest {
            schema_version: SCHEMA_VERSION,
            id: Uuid::new_v4().to_string(),
            title,
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
            chapters,
            exports: Vec::new(),
        };
        write_manifest(&root, &manifest)?;
        snapshot(&root, manifest)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result
}

pub fn open(root_path: &str) -> Result<ProjectSnapshot, CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let manifest = read_manifest(&root)?;
    snapshot(&root, manifest)
}

pub fn update_chapter(
    root_path: &str,
    expected_revision: u64,
    chapter_id: &str,
    title: &str,
    source_text: &str,
) -> Result<ProjectSnapshot, CommandError> {
    validate_manuscript(source_text)?;
    let title = clean_title(title)?;
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let chapter = manifest
        .chapters
        .iter_mut()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let source_path = owned_path(&root, &chapter.source_path)?;
    let source_changed = fs::read_to_string(&source_path)
        .map(|current| current != source_text)
        .unwrap_or(true);
    atomic_write(&source_path, source_text.as_bytes())?;
    chapter.title = title;
    chapter.segments = segment_text(source_text);
    chapter.audio_stale = chapter.audio_path.is_some();
    if source_changed {
        if let Some(processed_path) = chapter.processed_path.take() {
            let _ = fs::remove_file(owned_path(&root, &processed_path)?);
        }
        chapter.review_status = chapter.audio_path.as_ref().map(|_| "pending".into());
    }
    manifest.revision += 1;
    manifest.updated_at_ms = now_ms();
    write_manifest(&root, &manifest)?;
    snapshot(&root, manifest)
}

pub fn accept_processed(
    root_path: &str,
    expected_revision: u64,
    chapter_id: &str,
    text: &str,
) -> Result<ProjectSnapshot, CommandError> {
    validate_manuscript(text)?;
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let chapter = manifest
        .chapters
        .iter_mut()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let relative_path = format!("chapters/{chapter_id}/processed.txt");
    atomic_write(&owned_path(&root, &relative_path)?, text.as_bytes())?;
    chapter.processed_path = Some(relative_path);
    chapter.segments = segment_text(text);
    chapter.audio_stale = chapter.audio_path.is_some();
    chapter.review_status = chapter.audio_path.as_ref().map(|_| "pending".into());
    manifest.revision += 1;
    manifest.updated_at_ms = now_ms();
    write_manifest(&root, &manifest)?;
    snapshot(&root, manifest)
}

pub fn commit_chapter_audio(
    root_path: &str,
    expected_revision: u64,
    chapter_id: &str,
    staged_audio: &Path,
    duration_ms: u64,
    origin: &str,
    cues: Vec<LineCue>,
) -> Result<ProjectSnapshot, CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let chapter = manifest
        .chapters
        .iter_mut()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let relative_path = format!("chapters/{chapter_id}/audio.m4a");
    let destination = owned_path(&root, &relative_path)?;
    fs::rename(staged_audio, &destination)
        .or_else(|_| fs::copy(staged_audio, &destination).map(|_| ()))
        .map_err(|error| CommandError::io("Cannot store chapter audio", error))?;
    if staged_audio.exists() {
        let _ = fs::remove_file(staged_audio);
    }
    chapter.audio_path = Some(relative_path);
    chapter.audio_stale = false;
    chapter.audio_duration_ms = Some(duration_ms);
    chapter.audio_origin = Some(origin.to_string());
    chapter.review_status = Some("pending".into());
    chapter.cues = cues;
    manifest.revision += 1;
    manifest.updated_at_ms = now_ms();
    write_manifest(&root, &manifest)?;
    snapshot(&root, manifest)
}

pub fn set_review_status(
    root_path: &str,
    expected_revision: u64,
    chapter_id: &str,
    status: &str,
) -> Result<ProjectSnapshot, CommandError> {
    if !matches!(status, "pending" | "approved" | "changes_requested") {
        return Err(CommandError::new(
            "INVALID_REVIEW_STATUS",
            "Choose pending, approved, or changes requested.",
        ));
    }
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let chapter = manifest
        .chapters
        .iter_mut()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    if chapter.audio_path.is_none() || chapter.audio_stale {
        return Err(CommandError::new(
            "AUDIO_NOT_CURRENT",
            "Generate or import current audio before reviewing it.",
        ));
    }
    chapter.review_status = Some(status.into());
    manifest.revision += 1;
    manifest.updated_at_ms = now_ms();
    write_manifest(&root, &manifest)?;
    snapshot(&root, manifest)
}

pub fn chapter_audio_path(root_path: &str, chapter_id: &str) -> Result<PathBuf, CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let manifest = read_manifest(&root)?;
    let chapter = manifest
        .chapters
        .iter()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or_else(|| CommandError::new("CHAPTER_NOT_FOUND", "The chapter no longer exists."))?;
    let relative = chapter
        .audio_path
        .as_deref()
        .ok_or_else(|| CommandError::new("AUDIO_NOT_FOUND", "This chapter does not have audio."))?;
    let path = owned_path(&root, relative)?;
    fs::canonicalize(path).map_err(|error| CommandError::io("Cannot open chapter audio", error))
}

pub fn commit_export(
    root_path: &str,
    expected_revision: u64,
    staged_audio: &Path,
    staged_timestamps: &Path,
    duration_ms: u64,
) -> Result<ProjectSnapshot, CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let id = Uuid::new_v4().to_string();
    let stem = format!("{}-{}", project_folder_name(&manifest.title), &id[..8]);
    let audio_path = format!("output/{stem}.m4a");
    let timestamps_path = format!("output/{stem}-timestamps.txt");
    let audio_destination = owned_path(&root, &audio_path)?;
    let timestamps_destination = owned_path(&root, &timestamps_path)?;
    fs::copy(staged_audio, &audio_destination)
        .map_err(|error| CommandError::io("Cannot store audiobook", error))?;
    if let Err(error) = fs::copy(staged_timestamps, &timestamps_destination) {
        let _ = fs::remove_file(&audio_destination);
        return Err(CommandError::io("Cannot store timestamps", error));
    }
    manifest.exports.push(ExportRecord {
        id,
        audio_path,
        timestamps_path,
        duration_ms,
        created_at_ms: now_ms(),
        source_revision: expected_revision,
        source_updated_at_ms: manifest.updated_at_ms,
    });
    manifest.revision += 1;
    if let Err(error) = write_manifest(&root, &manifest) {
        let _ = fs::remove_file(&audio_destination);
        let _ = fs::remove_file(&timestamps_destination);
        return Err(error);
    }
    if let Some(work) = staged_audio.parent() {
        let _ = fs::remove_dir_all(work);
    }
    snapshot(&root, manifest)
}

pub fn export_paths(root_path: &str, export_id: &str) -> Result<(PathBuf, PathBuf), CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let manifest = read_manifest(&root)?;
    let record = manifest
        .exports
        .iter()
        .find(|record| record.id == export_id)
        .ok_or_else(|| CommandError::new("EXPORT_NOT_FOUND", "The export no longer exists."))?;
    Ok((
        fs::canonicalize(owned_path(&root, &record.audio_path)?)
            .map_err(|error| CommandError::io("Cannot open exported audiobook", error))?,
        fs::canonicalize(owned_path(&root, &record.timestamps_path)?)
            .map_err(|error| CommandError::io("Cannot open exported timestamps", error))?,
    ))
}

pub fn reorder(
    root_path: &str,
    expected_revision: u64,
    chapter_ids: &[String],
) -> Result<ProjectSnapshot, CommandError> {
    let root = fs::canonicalize(root_path)
        .map_err(|error| CommandError::io("Cannot open project folder", error))?;
    let mut manifest = read_manifest(&root)?;
    require_revision(&manifest, expected_revision)?;
    let current: HashSet<_> = manifest
        .chapters
        .iter()
        .map(|chapter| chapter.id.as_str())
        .collect();
    let requested: HashSet<_> = chapter_ids.iter().map(String::as_str).collect();
    if current != requested || chapter_ids.len() != manifest.chapters.len() {
        return Err(CommandError::new(
            "INVALID_ORDER",
            "Chapter order must contain every chapter exactly once.",
        ));
    }
    let mut by_id: HashMap<_, _> = manifest
        .chapters
        .drain(..)
        .map(|chapter| (chapter.id.clone(), chapter))
        .collect();
    manifest.chapters = chapter_ids
        .iter()
        .enumerate()
        .map(|(order, id)| {
            let mut chapter = by_id.remove(id).expect("validated chapter id");
            chapter.order = order;
            chapter
        })
        .collect();
    manifest.revision += 1;
    manifest.updated_at_ms = now_ms();
    write_manifest(&root, &manifest)?;
    snapshot(&root, manifest)
}

fn read_manifest(root: &Path) -> Result<ProjectManifest, CommandError> {
    let bytes = fs::read(root.join(MANIFEST))
        .map_err(|error| CommandError::io("Cannot read project.json", error))?;
    let manifest: ProjectManifest = serde_json::from_slice(&bytes).map_err(|error| {
        CommandError::new(
            "INVALID_PROJECT",
            format!("project.json is invalid: {error}"),
        )
    })?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(CommandError::new(
            "UNSUPPORTED_PROJECT_VERSION",
            format!(
                "Project schema {} is not supported by this version.",
                manifest.schema_version
            ),
        ));
    }
    Ok(manifest)
}

fn snapshot(root: &Path, manifest: ProjectManifest) -> Result<ProjectSnapshot, CommandError> {
    let chapters = manifest
        .chapters
        .iter()
        .map(|chapter| {
            let source_text = fs::read_to_string(owned_path(root, &chapter.source_path)?)
                .map_err(|error| CommandError::io("Cannot read chapter source", error))?;
            let processed_text = chapter
                .processed_path
                .as_deref()
                .map(|path| {
                    fs::read_to_string(owned_path(root, path)?)
                        .map_err(|error| CommandError::io("Cannot read processed chapter", error))
                })
                .transpose()?;
            Ok(ChapterSnapshot {
                chapter: chapter.clone(),
                source_text,
                processed_text,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    Ok(ProjectSnapshot {
        schema_version: manifest.schema_version,
        id: manifest.id,
        title: manifest.title,
        revision: manifest.revision,
        created_at_ms: manifest.created_at_ms,
        updated_at_ms: manifest.updated_at_ms,
        root_path: root.to_string_lossy().into_owned(),
        chapters,
        exports: manifest.exports,
    })
}

fn write_new_chapter(
    root: &Path,
    order: usize,
    parsed: ParsedChapter,
) -> Result<Chapter, CommandError> {
    let id = Uuid::new_v4().to_string();
    let directory = root.join("chapters").join(&id);
    fs::create_dir(&directory)
        .map_err(|error| CommandError::io("Cannot create chapter folder", error))?;
    let source_path = format!("chapters/{id}/source.txt");
    fs::write(directory.join("source.txt"), parsed.text.as_bytes())
        .map_err(|error| CommandError::io("Cannot write chapter source", error))?;
    Ok(Chapter {
        id,
        title: parsed.title,
        order,
        source_path,
        processed_path: None,
        segments: segment_text(&parsed.text),
        audio_path: None,
        audio_stale: false,
        audio_duration_ms: None,
        audio_origin: None,
        review_status: None,
        cues: Vec::new(),
    })
}

fn write_manifest(root: &Path, manifest: &ProjectManifest) -> Result<(), CommandError> {
    let data = serde_json::to_vec_pretty(manifest)
        .map_err(|error| CommandError::internal(format!("Cannot serialize project: {error}")))?;
    let target = root.join(MANIFEST);
    if target.exists() {
        fs::copy(&target, root.join("project.json.bak"))
            .map_err(|error| CommandError::io("Cannot back up project manifest", error))?;
    }
    atomic_write(&target, &data)
}

fn atomic_write(target: &Path, data: &[u8]) -> Result<(), CommandError> {
    let temp = target.with_extension("tmp");
    fs::write(&temp, data)
        .map_err(|error| CommandError::io("Cannot write temporary file", error))?;
    fs::rename(&temp, target).map_err(|error| CommandError::io("Cannot publish saved file", error))
}

fn owned_path(root: &Path, relative: &str) -> Result<PathBuf, CommandError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CommandError::new(
            "UNSAFE_PROJECT_PATH",
            "Project contains an unsafe asset path.",
        ));
    }
    Ok(root.join(path))
}

fn require_revision(
    manifest: &ProjectManifest,
    expected_revision: u64,
) -> Result<(), CommandError> {
    if manifest.revision != expected_revision {
        return Err(CommandError::new(
            "REVISION_CONFLICT",
            "The project changed on disk. Reopen it before saving again.",
        ));
    }
    Ok(())
}

fn clean_title(title: &str) -> Result<String, CommandError> {
    let value = title.trim();
    if value.is_empty() || value.chars().count() > 180 {
        return Err(CommandError::new(
            "INVALID_TITLE",
            "Enter a title between 1 and 180 characters.",
        ));
    }
    Ok(value.replace(['\n', '\r'], " "))
}

pub fn validate_manuscript(text: &str) -> Result<(), CommandError> {
    if text.trim().is_empty() {
        return Err(CommandError::new(
            "EMPTY_MANUSCRIPT",
            "The manuscript is empty.",
        ));
    }
    if text.len() > MAX_MANUSCRIPT_BYTES {
        return Err(CommandError::new(
            "MANUSCRIPT_TOO_LARGE",
            "The manuscript is larger than 20 MB.",
        ));
    }
    Ok(())
}

fn project_folder_name(title: &str) -> String {
    let slug = title
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "homer-project".to_string()
    } else {
        slug
    }
}

fn parse_chapters(manuscript: &str) -> Vec<ParsedChapter> {
    let mut chapters = Vec::new();
    let mut title: Option<String> = None;
    let mut lines = Vec::new();
    for line in manuscript.lines() {
        if let Some(heading) = heading_title(line) {
            if !lines.iter().all(|line: &&str| line.trim().is_empty()) {
                chapters.push(ParsedChapter {
                    title: title.take().unwrap_or_else(|| "Introduction".to_string()),
                    text: lines.join("\n").trim().to_string(),
                });
            }
            title = Some(heading);
            lines.clear();
        } else {
            lines.push(line);
        }
    }
    if !lines.iter().all(|line| line.trim().is_empty()) || title.is_some() {
        chapters.push(ParsedChapter {
            title: title.unwrap_or_else(|| "Chapter 1".to_string()),
            text: lines.join("\n").trim().to_string(),
        });
    }
    if chapters.is_empty() {
        chapters.push(ParsedChapter {
            title: "Chapter 1".to_string(),
            text: manuscript.trim().to_string(),
        });
    }
    chapters
}

fn heading_title(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let hashes = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=6).contains(&hashes) {
        let rest = trimmed[hashes..].trim();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    let lowercase = trimmed.to_lowercase();
    if lowercase.starts_with("chapter ") || lowercase.starts_with("part ") {
        return Some(trimmed.to_string());
    }
    None
}

fn segment_text(text: &str) -> Vec<Segment> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for paragraph in text
        .split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if !current.is_empty()
            && current.chars().count() + paragraph.chars().count() + 2 > MAX_SEGMENT_CHARS
        {
            chunks.push(std::mem::take(&mut current));
        }
        if paragraph.chars().count() > MAX_SEGMENT_CHARS {
            for sentence in paragraph.split_inclusive(['.', '!', '?']) {
                for piece in split_at_char_limit(sentence.trim(), MAX_SEGMENT_CHARS) {
                    if !current.is_empty()
                        && current.chars().count() + piece.chars().count() + 1 > MAX_SEGMENT_CHARS
                    {
                        chunks.push(std::mem::take(&mut current));
                    }
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(&piece);
                }
            }
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(paragraph);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
        .into_iter()
        .enumerate()
        .map(|(order, text)| Segment {
            id: Uuid::new_v4().to_string(),
            order,
            text,
            selected_take: None,
        })
        .collect()
}

fn split_at_char_limit(text: &str, limit: usize) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if word.chars().count() > limit {
            if !current.is_empty() {
                pieces.push(std::mem::take(&mut current));
            }
            let characters = word.chars().collect::<Vec<_>>();
            pieces.extend(
                characters
                    .chunks(limit)
                    .map(|chunk| chunk.iter().collect::<String>()),
            );
        } else if !current.is_empty() && current.chars().count() + word.chars().count() + 1 > limit
        {
            pieces.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    pieces
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_and_plain_chapter_headings() {
        let parsed = parse_chapters("Preface\n\n# First\nHello\n\nChapter Two\nWorld");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].title, "Introduction");
        assert_eq!(parsed[1].title, "First");
        assert_eq!(parsed[2].title, "Chapter Two");
    }

    #[test]
    fn creates_bounded_segments() {
        let input = format!(
            "{} {}. {}",
            "a".repeat(2_000),
            "c".repeat(700),
            "b".repeat(1_000)
        );
        let segments = segment_text(&input);
        assert_eq!(segments.len(), 3);
        assert!(segments
            .iter()
            .all(|segment| segment.text.chars().count() <= MAX_SEGMENT_CHARS));
    }

    #[test]
    fn rejects_parent_components_in_owned_paths() {
        assert!(owned_path(Path::new("/tmp/project"), "../secret").is_err());
        assert!(owned_path(Path::new("/tmp/project"), "chapters/id/source.txt").is_ok());
    }

    #[test]
    fn persists_edits_with_revision_checks_and_backup() {
        let parent = std::env::temp_dir().join(format!("homer-store-test-{}", Uuid::new_v4()));
        fs::create_dir(&parent).expect("create temporary parent");
        let created = create(parent.to_str().unwrap(), "Test Book", "# One\nOriginal")
            .expect("create project");
        assert_eq!(created.revision, 1);
        assert_eq!(created.chapters.len(), 1);

        let chapter_id = created.chapters[0].chapter.id.clone();
        let updated = update_chapter(&created.root_path, 1, &chapter_id, "Opening", "Updated")
            .expect("update chapter");
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.chapters[0].source_text, "Updated");
        assert!(Path::new(&updated.root_path)
            .join("project.json.bak")
            .is_file());

        let conflict = update_chapter(&created.root_path, 1, &chapter_id, "Old", "Stale")
            .expect_err("reject stale revision");
        assert_eq!(conflict.code, "REVISION_CONFLICT");

        let processed = accept_processed(&created.root_path, 2, &chapter_id, "Spoken version")
            .expect("accept processed text");
        assert_eq!(processed.revision, 3);
        assert_eq!(processed.chapters[0].source_text, "Updated");
        assert_eq!(
            processed.chapters[0].processed_text.as_deref(),
            Some("Spoken version")
        );
        assert!(Path::new(&processed.root_path)
            .join("chapters")
            .join(&chapter_id)
            .join("processed.txt")
            .is_file());
        fs::remove_dir_all(parent).expect("remove temporary project");
    }
}
