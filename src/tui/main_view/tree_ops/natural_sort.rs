use crate::tui::model::{GitObject, GitObjectType};

// ===== Natural sort helpers =====

#[derive(Debug, Clone, PartialEq, Eq)]
enum NatPart {
    Str(String),
    Num(u128),
}

impl Ord for NatPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use NatPart::{Num, Str};
        match (self, other) {
            (Str(a), Str(b)) => a.cmp(b),
            (Num(a), Num(b)) => a.cmp(b),
            (Str(_), Num(_)) => std::cmp::Ordering::Less,
            (Num(_), Str(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for NatPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn natural_key(s: &str) -> Vec<NatPart> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut is_digit: Option<bool> = None;

    for ch in s.chars() {
        let d = ch.is_ascii_digit();
        match is_digit {
            None => {
                is_digit = Some(d);
                buf.push(ch.to_ascii_lowercase());
            }
            Some(prev) if prev == d => {
                buf.push(ch.to_ascii_lowercase());
            }
            Some(_) => {
                if is_digit == Some(true) {
                    parts.push(NatPart::Num(buf.parse::<u128>().unwrap_or(0)));
                } else {
                    parts.push(NatPart::Str(buf.clone()));
                }
                buf.clear();
                is_digit = Some(d);
                buf.push(ch.to_ascii_lowercase());
            }
        }
    }
    if !buf.is_empty() {
        if is_digit == Some(true) {
            parts.push(NatPart::Num(buf.parse::<u128>().unwrap_or(0)));
        } else {
            parts.push(NatPart::Str(buf));
        }
    }
    parts
}

/// Natural sorting utility for Git objects with special rules for "objects" and "refs" folders
pub struct NaturalSorter;

impl NaturalSorter {
    /// Sort tree nodes for display with natural ordering and special folder precedence
    pub fn sort_tree_for_display(nodes: &mut [GitObject]) {
        // Sort the current level
        nodes.sort_by(|a, b| {
            // Special case: "objects" always comes first
            let a_name = match &a.obj_type {
                GitObjectType::Category(name) => name.as_str(),
                GitObjectType::FileSystemFolder { path, .. } => {
                    path.file_name().unwrap_or_default().to_str().unwrap_or("")
                }
                _ => &a.name,
            };

            let b_name = match &b.obj_type {
                GitObjectType::Category(name) => name.as_str(),
                GitObjectType::FileSystemFolder { path, .. } => {
                    path.file_name().unwrap_or_default().to_str().unwrap_or("")
                }
                _ => &b.name,
            };

            // Objects folder always comes first, refs folder comes second
            match (a_name, b_name) {
                ("objects", "objects") => std::cmp::Ordering::Equal,
                ("objects", _) => std::cmp::Ordering::Less,
                (_, "objects") => std::cmp::Ordering::Greater,
                ("refs", "refs") => std::cmp::Ordering::Equal,
                ("refs", _) if b_name != "objects" => std::cmp::Ordering::Less,
                (_, "refs") if a_name != "objects" => std::cmp::Ordering::Greater,
                _ => natural_key(a_name).cmp(&natural_key(b_name)),
            }
        });

        // Recursively sort children
        for node in nodes.iter_mut() {
            match &node.obj_type {
                GitObjectType::Category(name) => {
                    // Don't sort children of "objects" category (loose objects should keep their natural order)
                    if name != "objects" && name != "Loose Objects" {
                        Self::sort_tree_for_display(&mut node.children);
                    }
                }
                GitObjectType::FileSystemFolder { .. } => {
                    Self::sort_tree_for_display(&mut node.children);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_key() {
        let key1 = natural_key("file1.txt");
        let key2 = natural_key("file10.txt");
        let key3 = natural_key("file2.txt");

        assert!(key1 < key2);
        assert!(key3 < key2);
        assert!(key1 < key3);
    }

    #[test]
    fn test_special_folder_ordering() {
        // Test that "objects" comes before other folders
        let objects_key = natural_key("objects");
        let refs_key = natural_key("refs");
        let other_key = natural_key("hooks");

        // This would be tested in the actual sort function, but we can verify
        // the natural key generation works correctly
        assert_ne!(objects_key, refs_key);
        assert_ne!(objects_key, other_key);
    }
}
