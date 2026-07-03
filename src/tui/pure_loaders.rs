use crate::tui::message::InitialGitData;
use std::path::Path;

use crate::tui::model::PackObject;

pub fn load_pack_objects_pure(pack_path: &Path) -> Result<Vec<PackObject>, String> {
    let pack_data =
        std::fs::read(pack_path).map_err(|e| format!("Error reading pack file: {e}"))?;

    let mut parsed_objects = Vec::new();
    match crate::git::pack::Header::parse(&pack_data) {
        Ok((mut data, _header)) => {
            while !data.is_empty() {
                match crate::git::pack::Object::parse(data) {
                    Ok((new_data, object)) => {
                        parsed_objects.push(object);
                        data = new_data;
                    }
                    Err(_) => break,
                }
            }
        }
        Err(e) => {
            return Err(format!("Error parsing pack header: {e:?}"));
        }
    }

    // Resolve delta chains so every object gets its real git object ID;
    // unresolvable objects (e.g. thin-pack deltas) get sha1 = None
    let resolved = crate::git::pack::resolve_objects(&parsed_objects);

    let objects: Vec<PackObject> = parsed_objects
        .into_iter()
        .zip(resolved)
        .enumerate()
        .map(|(index, (object, resolved))| {
            let base_info = match &object.header {
                crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } => {
                    Some(format!("Base offset: {base_offset}"))
                }
                crate::git::pack::ObjectHeader::RefDelta { base_ref, .. } => {
                    Some(format!("Base ref: {}", hex::encode(base_ref)))
                }
                _ => None,
            };
            PackObject {
                index: index + 1,
                obj_type: object.header.obj_type().to_string(),
                size: u32::try_from(object.header.uncompressed_data_size()).unwrap_or(u32::MAX),
                sha1: resolved.map(|r| r.sha1),
                base_info,
                object_data: Some(object),
            }
        })
        .collect();

    Ok(objects)
}

/// Build the initial Git objects list without touching AppState/UI.
pub fn load_git_objects_pure(plumber: &crate::GitPlumber) -> Result<InitialGitData, String> {
    // Use the new file tree structure - it returns the contents directly
    let git_objects_list = crate::tui::git_tree::build_git_file_tree(plumber)?;

    Ok(InitialGitData { git_objects_list })
}
