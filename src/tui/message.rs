use crate::tui::model::PackObject;
use ratatui::text::Text;

// Define the possible messages (Actions)
#[derive(Debug)]
pub enum Message {
    Quit,
    SelectNext,
    SelectPrevious,
    SelectFirst,
    SelectLast,
    LoadGitObjects(Result<(), String>),
    LoadGitObjectInfo(Result<String, String>),
    LoadEducationalContent(Result<Text<'static>, String>),
    Refresh,
    // Pack navigation messages
    TogglePackFocus,
    EnterPackObjectDetail,
    ExitPackObjectDetail,
    HandlePackObjectDetailAction,
    BackFromObjectDetail,
    LoadPackObjects(Result<Vec<PackObject>, String>),
    MainNavigation(MainNavigation),
    OpenMainView,
    OpenPackView,
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
    FocusEducational,
    FocusDetails,
    FocusPackObjectDetails,
    FocusEducationalOrList,
    FocusPackList,
    FocusToggle,
}
