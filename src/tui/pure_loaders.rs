use crate::tui::message::InitialGitData;
use rayon::prelude::*;
use sha1::{Digest, Sha1};
use std::path::Path;

use crate::tui::model::PackObject;

pub fn load_pack_objects_pure(pack_path: &Path) -> Result<Vec<PackObject>, String> {
    let pack_data =
        std::fs::read(pack_path).map_err(|e| format!("Error reading pack file: {e}"))?;

    let mut parsed_objects = Vec::new();
    match crate::git::pack::Header::parse(&pack_data) {
        Ok((mut data, _header)) => {
            let mut object_count = 0;
            while !data.is_empty() {
                match crate::git::pack::Object::parse(data) {
                    Ok((new_data, object)) => {
                        let base_info = match &object.header {
                            crate::git::pack::ObjectHeader::OfsDelta { base_offset, .. } => {
                                Some(format!("Base offset: {base_offset}"))
                            }
                            crate::git::pack::ObjectHeader::RefDelta { base_ref, .. } => {
                                Some(format!("Base ref: {}", hex::encode(base_ref)))
                            }
                            _ => None,
                        };
                        object_count += 1;
                        parsed_objects.push((object_count, object, base_info));
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

    // Parallel SHA-1 calculation
    let objects: Vec<PackObject> = parsed_objects
        .into_par_iter()
        .map(|(index, object, base_info)| {
            let obj_type = object.header.obj_type();
            let size = object.header.uncompressed_data_size();
            let mut hasher = Sha1::new();
            let header = format!("{obj_type} {size}\0");
            hasher.update(header.as_bytes());
            hasher.update(&object.uncompressed_data);
            let sha1 = Some(format!("{:x}", hasher.finalize()));
            PackObject {
                index,
                obj_type: obj_type.to_string(),
                size: size as u32,
                sha1,
                base_info,
                object_data: Some(object),
            }
        })
        .collect();

    Ok(objects)
}

use crate::tui::model::GitObject;

/// Build the initial Git objects list without touching AppState/UI.
pub fn load_git_objects_pure(plumber: &crate::GitPlumber) -> Result<InitialGitData, String> {
    let mut root_list: Vec<GitObject> = Vec::new();

    let packs_category = crate::tui::git_tree::build_packs_category(plumber)?;
    let refs_category = crate::tui::git_tree::build_refs_category(plumber)?;
    let loose_category = crate::tui::git_tree::build_loose_category(plumber)?;

    root_list.push(packs_category);
    root_list.push(refs_category);
    root_list.push(loose_category);

    Ok(InitialGitData {
        git_objects_list: root_list,
    })
}
