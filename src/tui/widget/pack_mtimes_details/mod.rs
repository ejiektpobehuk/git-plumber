pub mod formatters;

use crate::git::pack::PackMtimes;
use crate::tui::widget::ScrollableTextWidget;
use ratatui::text::ToText;

use formatters::PackMtimesFormatter;

#[derive(Debug, Clone)]
pub enum PackMtimesWidget {
    Uninitialized,
    Initialized {
        mtimes: PackMtimes,
        scrollable_widget: ScrollableTextWidget,
    },
}

impl PackMtimesWidget {
    #[must_use]
    pub fn new(mtimes: PackMtimes) -> Self {
        let mut scrollable_widget = ScrollableTextWidget::new();
        // Pre-generate and cache the content
        let content = PackMtimesFormatter::new(&mtimes).generate_content();
        scrollable_widget.set_text(content);

        Self::Initialized {
            mtimes,
            scrollable_widget,
        }
    }

    #[must_use]
    pub fn text(&self) -> ratatui::text::Text<'static> {
        match self {
            Self::Initialized {
                scrollable_widget, ..
            } => scrollable_widget.text(),
            Self::Uninitialized => "Initializing Pack Mtimes Preview...".to_text(),
        }
    }

    pub const fn scroll_up(&mut self) {
        if let Self::Initialized {
            scrollable_widget, ..
        } = self
        {
            scrollable_widget.scroll_up();
        }
    }

    pub fn scroll_down(&mut self) {
        if let Self::Initialized {
            scrollable_widget, ..
        } = self
        {
            scrollable_widget.scroll_down();
        }
    }

    pub const fn scroll_to_top(&mut self) {
        if let Self::Initialized {
            scrollable_widget, ..
        } = self
        {
            scrollable_widget.scroll_to_top();
        }
    }

    pub const fn scroll_to_bottom(&mut self) {
        if let Self::Initialized {
            scrollable_widget, ..
        } = self
        {
            scrollable_widget.scroll_to_bottom();
        }
    }

    pub fn render(
        &mut self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
    ) {
        match self {
            Self::Initialized {
                scrollable_widget, ..
            } => {
                scrollable_widget.render(f, area, "Pack Mtimes Details", is_focused);
            }
            Self::Uninitialized => {
                // For uninitialized state, create a temporary scrollable widget with the loading message
                let mut temp_widget = ScrollableTextWidget::new();
                temp_widget.set_text("Initializing Pack Mtimes Preview...".to_text());
                temp_widget.render(f, area, "Pack Mtimes Details", is_focused);
            }
        }
    }
}
