use crate::tui::model::PackObject;
#[derive(Debug, Clone)]
pub enum Command {
    LoadInitial,
    LoadPackObjects { path: std::path::PathBuf },
}

#[derive(Debug, Clone)]
pub struct InitialGitData {
    pub git_objects_list: Vec<crate::tui::model::GitObject>,
}

use ratatui::text::Text;

// Define the possible messages (Actions)
#[derive(Debug)]
pub enum Message {
    Quit,
    LoadGitObjects(Result<(), String>),
    GitObjectsLoaded(InitialGitData),
    LoadGitObjectInfo(Result<String, String>),
    LoadEducationalContent(Result<Text<'static>, String>),
    Refresh,
    LoadPackObjects {
        path: std::path::PathBuf,
        result: Result<Vec<PackObject>, String>,
    },
    MainNavigation(MainNavigation),
    PackNavigation(PackNavigation),
    LooseObjectNavigation(LooseObjectNavigation),
    OpenMainView,
    OpenPackView,
    OpenLooseObjectView,
}

#[derive(Debug)]
pub enum MainNavigation {
    // Educational content
    ScrollEducationalUp,
    ScrollEducationalDown,
    ScrollEducationalToTop,
    ScrollEducationalToBottom,
    // Git Internal Object List
    SelectPreviouwGitObject,
    SelectNextGitObject,
    SelectFirstGitObject,
    SelectLastGitObject,
    ToggleExpand,
    JumpToParentCategory,
    // Preview/content
    ScrollPreviewUp,
    ScrollPreviewDown,
    ScrollPreviewToTop,
    ScrollPreviewToBottom,
    // Pack object list
    SelectNextPackObject,
    SelectPreviousPackObject,
    SelectFirstPackObject,
    SelectLastPackObject,
    // Focus
    FocusGitObjects,
    FocusPackObjectDetails,
    FocusEducationalOrList,
    FocusToggle,
}

#[derive(Debug)]
pub enum PackNavigation {
    ScrollUp,
    ScrollDown,
    ScrollToTop,
    ScrollToBottom,
}

#[derive(Debug)]
pub enum LooseObjectNavigation {
    ScrollUp,
    ScrollDown,
    ScrollToTop,
    ScrollToBottom,
}
