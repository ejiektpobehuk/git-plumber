use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LooseObjectError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid object format: {0}")]
    InvalidFormat(String),

    #[error("Unknown object type: {0}")]
    UnknownType(String),

    #[error("Decompression error: {0}")]
    DecompressionError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LooseObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl std::fmt::Display for LooseObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LooseObjectType::Commit => write!(f, "commit"),
            LooseObjectType::Tree => write!(f, "tree"),
            LooseObjectType::Blob => write!(f, "blob"),
            LooseObjectType::Tag => write!(f, "tag"),
        }
    }
}

impl std::str::FromStr for LooseObjectType {
    type Err = LooseObjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commit" => Ok(LooseObjectType::Commit),
            "tree" => Ok(LooseObjectType::Tree),
            "blob" => Ok(LooseObjectType::Blob),
            "tag" => Ok(LooseObjectType::Tag),
            _ => Err(LooseObjectError::UnknownType(s.to_string())),
        }
    }
}

// Parsed commit object structure
#[derive(Debug, Clone)]
pub struct CommitObject {
    pub tree: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_date: String,
    pub committer: String,
    pub committer_date: String,
    pub message: String,
}

// Parsed tree entry structure
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub sha1: String,
    pub object_type: TreeEntryType,
}

#[derive(Debug, Clone)]
pub enum TreeEntryType {
    Blob,
    Tree,
    Executable,
    Symlink,
    Submodule,
}

// Parsed tree object structure
#[derive(Debug, Clone)]
pub struct TreeObject {
    pub entries: Vec<TreeEntry>,
}

// Parsed tag object structure
#[derive(Debug, Clone)]
pub struct TagObject {
    pub object: String,
    pub object_type: String,
    pub tag: String,
    pub tagger: Option<String>,
    pub tagger_date: Option<String>,
    pub message: String,
}

// Parsed object content
#[derive(Debug, Clone)]
pub enum ParsedContent {
    Commit(CommitObject),
    Tree(TreeObject),
    Blob(Vec<u8>),
    Tag(TagObject),
}

#[derive(Debug, Clone)]
pub struct LooseObject {
    pub object_type: LooseObjectType,
    pub size: usize,
    pub content: Vec<u8>,
    pub object_id: String,
    pub parsed_content: Option<ParsedContent>,
}

impl LooseObject {
    /// Read and parse a loose object from the given path
    pub fn read_from_path(path: &Path) -> Result<Self, LooseObjectError> {
        // Extract object ID from path
        let object_id = Self::extract_object_id(path)?;

        // Read the compressed file
        let compressed_data = fs::read(path)?;

        // Decompress the data
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| LooseObjectError::DecompressionError(e.to_string()))?;

        // Parse the object header and content
        Self::parse_object_data(&decompressed, object_id)
    }

    /// Extract object ID from the file path
    /// Path format: .git/objects/ab/cdef123456...
    fn extract_object_id(path: &Path) -> Result<String, LooseObjectError> {
        let filename = path
            .file_name()
            .ok_or_else(|| LooseObjectError::InvalidFormat("No filename".to_string()))?
            .to_string_lossy();

        let parent_dir = path
            .parent()
            .ok_or_else(|| LooseObjectError::InvalidFormat("No parent directory".to_string()))?
            .file_name()
            .ok_or_else(|| LooseObjectError::InvalidFormat("No parent directory name".to_string()))?
            .to_string_lossy();

        if parent_dir.len() != 2 {
            return Err(LooseObjectError::InvalidFormat(
                "Parent directory should be 2 characters".to_string(),
            ));
        }

        if filename.len() != 38 {
            return Err(LooseObjectError::InvalidFormat(
                "Filename should be 38 characters".to_string(),
            ));
        }

        Ok(format!("{parent_dir}{filename}"))
    }

    /// Parse the decompressed object data
    /// Format: "<type> <size>\0<content>"
    fn parse_object_data(data: &[u8], object_id: String) -> Result<Self, LooseObjectError> {
        // Find the null terminator that separates header from content
        let null_pos = data.iter().position(|&b| b == 0).ok_or_else(|| {
            LooseObjectError::InvalidFormat("No null terminator found".to_string())
        })?;

        // Split header and content
        let header = &data[..null_pos];
        let content = &data[null_pos + 1..];

        // Parse header: "<type> <size>"
        let header_str = String::from_utf8_lossy(header);
        let parts: Vec<&str> = header_str.split(' ').collect();

        if parts.len() != 2 {
            return Err(LooseObjectError::InvalidFormat(
                "Header should contain type and size".to_string(),
            ));
        }

        let object_type = parts[0].parse::<LooseObjectType>()?;
        let size = parts[1]
            .parse::<usize>()
            .map_err(|_| LooseObjectError::InvalidFormat("Invalid size".to_string()))?;

        // Verify size matches content length
        if size != content.len() {
            return Err(LooseObjectError::InvalidFormat(format!(
                "Size mismatch: header says {}, content is {}",
                size,
                content.len()
            )));
        }

        // Parse type-specific content
        let parsed_content = match object_type {
            LooseObjectType::Commit => {
                Self::parse_commit_content(content).map(ParsedContent::Commit)
            }
            LooseObjectType::Tree => Self::parse_tree_content(content).map(ParsedContent::Tree),
            LooseObjectType::Blob => Ok(ParsedContent::Blob(content.to_vec())),
            LooseObjectType::Tag => Self::parse_tag_content(content).map(ParsedContent::Tag),
        };

        Ok(LooseObject {
            object_type,
            size,
            content: content.to_vec(),
            object_id,
            parsed_content: parsed_content.ok(),
        })
    }

    /// Parse commit object content
    fn parse_commit_content(content: &[u8]) -> Result<CommitObject, LooseObjectError> {
        let content_str = String::from_utf8_lossy(content);
        let lines = content_str.lines();

        let mut tree = String::new();
        let mut parents = Vec::new();
        let mut author = String::new();
        let mut author_date = String::new();
        let mut committer = String::new();
        let mut committer_date = String::new();
        let mut message = String::new();

        // Parse header lines
        let mut in_message = false;
        for line in lines {
            if in_message {
                if !message.is_empty() {
                    message.push('\n');
                }
                message.push_str(line);
            } else if line.is_empty() {
                in_message = true;
            } else if let Some(stripped) = line.strip_prefix("tree ") {
                tree = stripped.to_string();
            } else if let Some(stripped) = line.strip_prefix("parent ") {
                parents.push(stripped.to_string());
            } else if let Some(author_line) = line.strip_prefix("author ") {
                if let Some(date_start) = author_line.rfind(' ') {
                    if let Some(name_end) = author_line[..date_start].rfind(' ') {
                        author = author_line[..name_end].to_string();
                        author_date = author_line[name_end + 1..].to_string();
                    }
                }
            } else if let Some(committer_line) = line.strip_prefix("committer ") {
                if let Some(date_start) = committer_line.rfind(' ') {
                    if let Some(name_end) = committer_line[..date_start].rfind(' ') {
                        committer = committer_line[..name_end].to_string();
                        committer_date = committer_line[name_end + 1..].to_string();
                    }
                }
            }
        }

        Ok(CommitObject {
            tree,
            parents,
            author,
            author_date,
            committer,
            committer_date,
            message,
        })
    }

    /// Parse tree object content
    fn parse_tree_content(content: &[u8]) -> Result<TreeObject, LooseObjectError> {
        let mut entries = Vec::new();
        let mut i = 0;

        while i < content.len() {
            // Read mode (until space)
            let mode_start = i;
            while i < content.len() && content[i] != b' ' {
                i += 1;
            }
            if i >= content.len() {
                break;
            }
            let mode = String::from_utf8_lossy(&content[mode_start..i]).to_string();
            i += 1; // Skip space

            // Read filename (until null)
            let name_start = i;
            while i < content.len() && content[i] != 0 {
                i += 1;
            }
            if i >= content.len() {
                break;
            }
            let name = String::from_utf8_lossy(&content[name_start..i]).to_string();
            i += 1; // Skip null

            // Read SHA-1 (20 bytes)
            if i + 20 > content.len() {
                break;
            }
            let sha1 = hex::encode(&content[i..i + 20]);
            i += 20;

            // Determine object type from mode
            let object_type = match mode.as_str() {
                "100644" => TreeEntryType::Blob,
                "100755" => TreeEntryType::Executable,
                "120000" => TreeEntryType::Symlink,
                "160000" => TreeEntryType::Submodule,
                "040000" => TreeEntryType::Tree,
                _ => TreeEntryType::Blob, // Default fallback
            };

            entries.push(TreeEntry {
                mode,
                name,
                sha1,
                object_type,
            });
        }

        Ok(TreeObject { entries })
    }

    /// Parse tag object content
    fn parse_tag_content(content: &[u8]) -> Result<TagObject, LooseObjectError> {
        let content_str = String::from_utf8_lossy(content);
        let lines = content_str.lines();

        let mut object = String::new();
        let mut object_type = String::new();
        let mut tag = String::new();
        let mut tagger = None;
        let mut tagger_date = None;
        let mut message = String::new();

        // Parse header lines
        let mut in_message = false;
        for line in lines {
            if in_message {
                if !message.is_empty() {
                    message.push('\n');
                }
                message.push_str(line);
            } else if line.is_empty() {
                in_message = true;
            } else if let Some(stripped) = line.strip_prefix("object ") {
                object = stripped.to_string();
            } else if let Some(stripped) = line.strip_prefix("type ") {
                object_type = stripped.to_string();
            } else if let Some(stripped) = line.strip_prefix("tag ") {
                tag = stripped.to_string();
            } else if let Some(tagger_line) = line.strip_prefix("tagger ") {
                if let Some(date_start) = tagger_line.rfind(' ') {
                    if let Some(name_end) = tagger_line[..date_start].rfind(' ') {
                        tagger = Some(tagger_line[..name_end].to_string());
                        tagger_date = Some(tagger_line[name_end + 1..].to_string());
                    }
                }
            }
        }

        Ok(TagObject {
            object,
            object_type,
            tag,
            tagger,
            tagger_date,
            message,
        })
    }

    /// Get the content as a UTF-8 string (for text objects like commits)
    pub fn content_as_string(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }

    /// Check if this object is binary (likely a blob)
    pub fn is_binary(&self) -> bool {
        // Simple heuristic: if content contains null bytes, it's likely binary
        self.content.contains(&0) || matches!(self.object_type, LooseObjectType::Blob)
    }

    /// Get parsed content if available
    pub fn get_parsed_content(&self) -> Option<&ParsedContent> {
        self.parsed_content.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    #[test]
    fn test_extract_object_id() {
        let path = Path::new(".git/objects/ab/cdef1234567890123456789012345678901234");
        let object_id = LooseObject::extract_object_id(path).unwrap();
        assert_eq!(object_id, "abcdef1234567890123456789012345678901234");
    }

    #[test]
    fn test_parse_object_data() {
        let content = b"Hello, World!";
        let header = b"blob 13\0";
        let mut data = Vec::new();
        data.extend_from_slice(header);
        data.extend_from_slice(content);

        let object = LooseObject::parse_object_data(&data, "test123".to_string()).unwrap();
        assert_eq!(object.object_type, LooseObjectType::Blob);
        assert_eq!(object.size, 13);
        assert_eq!(object.content, content);
        assert_eq!(object.object_id, "test123");
    }

    #[test]
    fn test_create_and_read_loose_object() {
        let temp_dir = tempfile::tempdir().unwrap();
        let objects_dir = temp_dir.path().join("objects").join("ab");
        std::fs::create_dir_all(&objects_dir).unwrap();

        // Create test object content
        let content = b"Hello, World!";
        let header = b"blob 13\0";
        let mut data = Vec::new();
        data.extend_from_slice(header);
        data.extend_from_slice(content);

        // Compress the data
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Write to file
        let file_path = objects_dir.join("cdef1234567890123456789012345678901234");
        std::fs::write(&file_path, compressed).unwrap();

        // Read and parse
        let object = LooseObject::read_from_path(&file_path).unwrap();
        assert_eq!(object.object_type, LooseObjectType::Blob);
        assert_eq!(object.size, 13);
        assert_eq!(object.content, content);
        assert_eq!(object.object_id, "abcdef1234567890123456789012345678901234");
    }

    #[test]
    fn test_parse_commit_content() {
        let content = b"tree 1234567890123456789012345678901234567890\nparent abcdef1234567890123456789012345678901234\nauthor John Doe <john@example.com> 1234567890 +0000\ncommitter John Doe <john@example.com> 1234567890 +0000\n\nInitial commit\n";

        let commit = LooseObject::parse_commit_content(content).unwrap();
        assert_eq!(commit.tree, "1234567890123456789012345678901234567890");
        assert_eq!(commit.parents.len(), 1);
        assert_eq!(
            commit.parents[0],
            "abcdef1234567890123456789012345678901234"
        );
        assert!(commit.author.contains("John Doe"));
        assert_eq!(commit.message, "Initial commit");
    }
}
