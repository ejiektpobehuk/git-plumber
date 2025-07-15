use super::LooseObjectViewState;
use crate::tui::message::{LooseObjectNavigation, Message};
use crate::tui::model::{AppState, AppView};

impl AppState {
    pub fn handle_loose_object_view_mode_message(&mut self, msg: Message) -> bool {
        match msg {
            Message::LooseObjectNavigation(msg) => match msg {
                LooseObjectNavigation::ScrollUp => {
                    if let AppView::LooseObjectDetail {
                        state: LooseObjectViewState { loose_widget },
                    } = &mut self.view
                    {
                        loose_widget.scroll_up();
                    }
                }
                LooseObjectNavigation::ScrollDown => {
                    if let AppView::LooseObjectDetail {
                        state: LooseObjectViewState { loose_widget },
                    } = &mut self.view
                    {
                        loose_widget.scroll_down();
                    }
                }
                LooseObjectNavigation::ScrollToTop => {
                    if let AppView::LooseObjectDetail {
                        state: LooseObjectViewState { loose_widget },
                    } = &mut self.view
                    {
                        loose_widget.scroll_to_top();
                    }
                }
                LooseObjectNavigation::ScrollToBottom => {
                    if let AppView::LooseObjectDetail {
                        state: LooseObjectViewState { loose_widget },
                    } = &mut self.view
                    {
                        loose_widget.scroll_to_bottom();
                    }
                }
            },
            _ => {
                unreachable!(
                    "handle_loose_object_view_mode_message called with non-loose-object-view message"
                )
            }
        }
        true
    }
}
