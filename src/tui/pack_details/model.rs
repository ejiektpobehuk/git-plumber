use crate::tui::model::PackObject;
use std::path::PathBuf;

pub struct PackViewState {
    pub pack_file_path: PathBuf,
    pub pack_object_list: Vec<PackObject>,
    pub pack_object_index: usize,
    pub pack_object_list_scroll_position: usize,
    pub preview_scroll_position: usize,
}
