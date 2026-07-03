//! Resolves delta chains inside a pack file to recover the real object IDs.
//!
//! Git object IDs are defined over the *resolved* content
//! (`"{type} {size}\0" + content`), so delta objects have no ID of their
//! own until their chain is applied. This module reconstructs that content
//! using only the pack file itself — no `.idx` required.

use std::borrow::Cow;
use std::collections::HashMap;

use sha1::{Digest, Sha1};

use super::delta::{DeltaInstruction, parse_delta_instructions};
use super::object::{Object, ObjectHeader, ObjectType};

/// Fixed size of the pack header: "PACK" + version + object count.
pub const PACK_HEADER_SIZE: u64 = 12;

/// Content of a resolved object: regular objects borrow their pack data,
/// resolved deltas own the reconstructed bytes.
type ResolvedContent<'a> = (ObjectType, Cow<'a, [u8]>);

/// The real identity of a pack object after delta resolution.
#[derive(Debug, Clone)]
pub struct ResolvedObject {
    /// The effective object type (a delta inherits its base's type)
    pub obj_type: ObjectType,
    /// Size of the fully resolved content in bytes
    pub size: usize,
    /// Hex-encoded git object ID of the resolved content
    pub sha1: String,
}

fn git_object_digest(obj_type: ObjectType, data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(format!("{obj_type} {}\0", data.len()).as_bytes());
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute the git object ID for resolved content.
#[must_use]
pub fn object_id(obj_type: ObjectType, data: &[u8]) -> String {
    hex::encode(git_object_digest(obj_type, data))
}

/// Apply a delta instruction stream (size varints already stripped by
/// `Object::parse`) to base content. Returns `None` if an instruction is
/// malformed or copies out of bounds.
fn apply_delta(base: &[u8], delta: &[u8]) -> Option<Vec<u8>> {
    let (_, instructions) = parse_delta_instructions(delta).ok()?;
    let mut result = Vec::new();
    for instruction in instructions {
        match instruction {
            DeltaInstruction::Copy { offset, size } => {
                let end = offset.checked_add(size)?;
                result.extend_from_slice(base.get(offset..end)?);
            }
            DeltaInstruction::Insert { data } => result.extend_from_slice(&data),
        }
    }
    Some(result)
}

/// Resolve every object in a pack to its real type, size, and git object ID.
///
/// `objects` must be the objects of a single pack in on-disk order, starting
/// with the first object after the pack header — each object's byte offset is
/// reconstructed from the header and compressed sizes, which `ofs_delta`
/// resolution depends on.
///
/// The returned vector is aligned with the input. An entry is `None` when the
/// object cannot be resolved from this pack alone: a `ref_delta` whose base
/// lives outside the pack (thin pack), a base offset pointing at no object
/// boundary, or a malformed delta stream.
#[must_use]
pub fn resolve_objects(objects: &[Object]) -> Vec<Option<ResolvedObject>> {
    let mut offsets = Vec::with_capacity(objects.len());
    let mut offset_to_index = HashMap::with_capacity(objects.len());
    let mut offset = PACK_HEADER_SIZE;
    for (index, object) in objects.iter().enumerate() {
        offsets.push(offset);
        offset_to_index.insert(offset, index);
        offset += (object.header.raw_data().len() + object.compressed_size) as u64;
    }

    // Resolved content per object: regular objects borrow their data,
    // resolved deltas own theirs. Kept for the whole pass so later links
    // in a chain (and ref_deltas found by digest) can read their base.
    let mut content: Vec<Option<ResolvedContent>> = (0..objects.len()).map(|_| None).collect();
    let mut digest_to_index: HashMap<[u8; 20], usize> = HashMap::new();
    let mut results: Vec<Option<ResolvedObject>> = (0..objects.len()).map(|_| None).collect();
    let mut failed = vec![false; objects.len()];

    // Bases usually precede their deltas, so this converges in one or two
    // passes; each extra pass resolves at least one more chain link.
    loop {
        let mut progress = false;

        for index in 0..objects.len() {
            if content[index].is_some() || failed[index] {
                continue;
            }

            // Ok(Some(..)) = resolved, Ok(None) = base not available yet
            // (retry next pass), Err(()) = permanently unresolvable.
            let outcome: Result<Option<ResolvedContent>, ()> = match &objects[index].header {
                ObjectHeader::Regular { obj_type, .. } => match obj_type {
                    ObjectType::Commit | ObjectType::Tree | ObjectType::Blob | ObjectType::Tag => {
                        Ok(Some((
                            *obj_type,
                            Cow::Borrowed(objects[index].uncompressed_data.as_slice()),
                        )))
                    }
                    _ => Err(()),
                },
                ObjectHeader::OfsDelta { base_offset, .. } => u64::try_from(*base_offset)
                    .ok()
                    .and_then(|distance| offsets[index].checked_sub(distance))
                    .and_then(|base| offset_to_index.get(&base).copied())
                    .map_or(Err(()), |base_index| match &content[base_index] {
                        Some((base_type, base_data)) => {
                            apply_delta(base_data, &objects[index].uncompressed_data)
                                .map(|data| Some((*base_type, Cow::Owned(data))))
                                .ok_or(())
                        }
                        None if failed[base_index] => Err(()),
                        None => Ok(None),
                    }),
                ObjectHeader::RefDelta { base_ref, .. } => {
                    // The base may appear later in the pack or be missing
                    // entirely (thin pack) — only the fixpoint decides.
                    match digest_to_index.get(base_ref) {
                        Some(&base_index) => match &content[base_index] {
                            Some((base_type, base_data)) => {
                                apply_delta(base_data, &objects[index].uncompressed_data)
                                    .map(|data| Some((*base_type, Cow::Owned(data))))
                                    .ok_or(())
                            }
                            None => Ok(None),
                        },
                        None => Ok(None),
                    }
                }
            };

            match outcome {
                Err(()) => failed[index] = true,
                Ok(None) => {}
                Ok(Some((obj_type, data))) => {
                    let digest = git_object_digest(obj_type, &data);
                    digest_to_index.insert(digest, index);
                    results[index] = Some(ResolvedObject {
                        obj_type,
                        size: data.len(),
                        sha1: hex::encode(digest),
                    });
                    content[index] = Some((obj_type, data));
                    progress = true;
                }
            }
        }

        if !progress {
            break;
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    const BASE_CONTENT: &[u8] = b"the quick brown fox";
    const RESULT_CONTENT: &[u8] = b"the quick brown fox jumps";

    fn zlib_compress(data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    /// A blob object entry: header byte(s) + zlib-compressed content
    fn blob_entry(content: &[u8]) -> Vec<u8> {
        assert!(content.len() >= 16 && content.len() < (16 << 7));
        let mut entry = vec![
            0x80 | (u8::try_from(ObjectType::Blob as usize).unwrap() << 4)
                | (content.len() & 0x0F) as u8,
            (content.len() >> 4) as u8,
        ];
        entry.extend_from_slice(&zlib_compress(content));
        entry
    }

    /// Delta payload: base size + result size varints, then one copy of the
    /// whole base and one insert of " jumps"
    fn delta_payload() -> Vec<u8> {
        let mut payload = vec![
            BASE_CONTENT.len() as u8,
            RESULT_CONTENT.len() as u8,
            0x90, // copy, one size byte follows, offset 0
            BASE_CONTENT.len() as u8,
            6, // insert 6 bytes
        ];
        payload.extend_from_slice(b" jumps");
        payload
    }

    fn parse_pack_objects(pack: &[u8]) -> Vec<Object> {
        let (mut data, _header) = crate::git::pack::Header::parse(pack).unwrap();
        let mut objects = Vec::new();
        while !data.is_empty() {
            let (rest, object) = Object::parse(data).unwrap();
            objects.push(object);
            data = rest;
        }
        objects
    }

    fn pack_header(object_count: u32) -> Vec<u8> {
        let mut pack = Vec::new();
        pack.extend_from_slice(b"PACK");
        pack.extend_from_slice(&2u32.to_be_bytes());
        pack.extend_from_slice(&object_count.to_be_bytes());
        pack
    }

    // Computed with `git hash-object --stdin` over RESULT_CONTENT
    const RESULT_SHA1: &str = "77fd0f80a077c27201b64eabc78952ffa9bc691d";

    #[test]
    fn resolves_ofs_delta_to_real_object_id() {
        let mut pack = pack_header(2);
        let base_entry = blob_entry(BASE_CONTENT);
        let base_distance = base_entry.len() as u8; // < 128: single varint byte
        pack.extend_from_slice(&base_entry);

        let payload = delta_payload();
        assert!(payload.len() < 16);
        pack.push((ObjectType::OfsDelta as u8) << 4 | payload.len() as u8);
        pack.push(base_distance);
        pack.extend_from_slice(&zlib_compress(&payload));

        let objects = parse_pack_objects(&pack);
        let resolved = resolve_objects(&objects);

        let base = resolved[0].as_ref().unwrap();
        assert_eq!(base.obj_type, ObjectType::Blob);
        assert_eq!(base.sha1, object_id(ObjectType::Blob, BASE_CONTENT));

        let delta = resolved[1].as_ref().unwrap();
        assert_eq!(delta.obj_type, ObjectType::Blob);
        assert_eq!(delta.size, RESULT_CONTENT.len());
        assert_eq!(delta.sha1, RESULT_SHA1);
        assert_eq!(delta.sha1, object_id(ObjectType::Blob, RESULT_CONTENT));
    }

    #[test]
    fn resolves_ref_delta_even_when_base_comes_later() {
        // Delta first, base second: forces a second resolution pass
        let mut pack = pack_header(2);
        let base_digest = git_object_digest(ObjectType::Blob, BASE_CONTENT);

        let payload = delta_payload();
        pack.push((ObjectType::RefDelta as u8) << 4 | payload.len() as u8);
        pack.extend_from_slice(&base_digest);
        pack.extend_from_slice(&zlib_compress(&payload));
        pack.extend_from_slice(&blob_entry(BASE_CONTENT));

        let objects = parse_pack_objects(&pack);
        let resolved = resolve_objects(&objects);

        let delta = resolved[0].as_ref().unwrap();
        assert_eq!(delta.obj_type, ObjectType::Blob);
        assert_eq!(delta.sha1, RESULT_SHA1);
    }

    #[test]
    fn missing_ref_delta_base_yields_none() {
        // Thin-pack style: the base digest doesn't exist in this pack
        let mut pack = pack_header(1);
        let payload = delta_payload();
        pack.push((ObjectType::RefDelta as u8) << 4 | payload.len() as u8);
        pack.extend_from_slice(&[0xAB; 20]);
        pack.extend_from_slice(&zlib_compress(&payload));

        let objects = parse_pack_objects(&pack);
        let resolved = resolve_objects(&objects);
        assert!(resolved[0].is_none());
    }
}
