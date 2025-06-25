use super::PackViewState;
use crate::tui::message::{Message, PackNavigation};
use crate::tui::model::{AppState, AppView};

impl AppState {
    pub fn handle_pack_view_mode_message(&mut self, msg: Message) -> bool {
        match msg {
            Message::PackNavigation(msg) => match msg {
                PackNavigation::ScrollUp => {
                    if let AppView::PackObjectDetail {
                        state: PackViewState { pack_widget },
                    } = &mut self.view
                    {
                        pack_widget.scroll_up();
                    }
                }
                PackNavigation::ScrollDown => {
                    if let AppView::PackObjectDetail {
                        state: PackViewState { pack_widget },
                    } = &mut self.view
                    {
                        pack_widget.scroll_down();
                    }
                }
                PackNavigation::ScrollToTop => {
                    if let AppView::PackObjectDetail {
                        state: PackViewState { pack_widget },
                    } = &mut self.view
                    {
                        pack_widget.scroll_to_top();
                    }
                }
                PackNavigation::ScrollToBottom => {
                    if let AppView::PackObjectDetail {
                        state: PackViewState { pack_widget },
                    } = &mut self.view
                    {
                        pack_widget.scroll_to_bottom();
                    }
                }
            },
            _ => {
                unreachable!("handle_pack_view_mode_message called with non-pack-view message")
            }
        }
        true
    }
}
