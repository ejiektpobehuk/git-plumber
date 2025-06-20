use super::PackViewState;
use crate::tui::message::{Message, PackNavigation};
use crate::tui::model::{AppState, AppView};
use crate::tui::widget::PackObjectWidget;

impl AppState {
    pub fn handle_pack_view_mode_message(
        &mut self,
        msg: Message,
        plumber: &crate::GitPlumber,
    ) -> bool {
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
                    if let AppView::PackObjectDetail { state } = &mut self.view {
                        let scroll_position = state.pack_widget.scroll_position();
                        if scroll_position < state.pack_object_detail_max_scroll {
                            scroll_position += 1;
                            state.pack_widget.set_scroll_position(scroll_position);
                        }
                    }
                }
                _ => {}
            },
            Message::OpenMainView => {}
            _ => {
                unreachable!("handle_pack_view_mode_message called with non-pack-view message")
            }
        }
        true
    }
}
