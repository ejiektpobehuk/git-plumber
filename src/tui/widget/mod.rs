pub mod formatters_utils;
pub mod loose_obj_details;
pub mod multi_pack_index_details;
pub mod pack_idx_details;
pub mod pack_mtimes_details;
pub mod pack_obj_details;
pub mod pack_rev_details;
pub mod scrollable_text;

pub use multi_pack_index_details::MultiPackIndexWidget;
pub use pack_idx_details::PackIndexWidget;
pub use pack_mtimes_details::PackMtimesWidget;
pub use pack_obj_details::PackObjectWidget;
pub use pack_rev_details::PackReverseIndexWidget;
pub use scrollable_text::ScrollableTextWidget;
