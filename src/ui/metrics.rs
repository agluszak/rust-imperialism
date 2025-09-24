#[derive(Debug, Clone)]
pub struct ScrollbarMetrics {
    pub content_height: f32,
    pub visible_height: f32,
    pub max_scroll: f32,
    pub thumb_size_ratio: f32,
    pub can_scroll: bool,
}

impl ScrollbarMetrics {
    pub fn calculate(visible_size: bevy::math::Vec2, font_size: f32) -> Self {
        Self::calculate_with_content_size(visible_size, font_size, None)
    }

    pub fn calculate_with_content_size(
        visible_size: bevy::math::Vec2,
        font_size: f32,
        actual_content_size: Option<bevy::math::Vec2>,
    ) -> Self {
        let visible_height = visible_size.y;

        // Prefer actual computed content size from the Text node when available
        let content_height = if let Some(actual_size) = actual_content_size {
            // Use actual text height directly
            actual_size.y
        } else {
            // Fallback: assume no scrolling until the text is measured
            // Use visible height so thumb fills the track and doesn't move yet
            visible_height
        };

        // Add a tiny epsilon to avoid thrashing around the exact threshold due to float precision
        let epsilon = 0.5;
        let can_scroll = content_height > (visible_height + epsilon);
        let max_scroll = if can_scroll {
            (content_height - visible_height).max(0.0)
        } else {
            0.0
        };

        let thumb_size_ratio = if can_scroll {
            (visible_height / content_height).clamp(0.05, 1.0) // Min 5% height
        } else {
            1.0
        };

        Self {
            content_height,
            visible_height,
            max_scroll,
            thumb_size_ratio,
            can_scroll,
        }
    }

    pub fn clamp_scroll_position(&self, position: f32) -> f32 {
        if self.can_scroll {
            position.clamp(0.0, self.max_scroll)
        } else {
            0.0
        }
    }

    pub fn scroll_ratio(&self, scroll_position: f32) -> f32 {
        if self.can_scroll && self.max_scroll > 0.0 {
            (scroll_position / self.max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub fn thumb_position_percent(&self, scroll_position: f32) -> f32 {
        let scroll_ratio = self.scroll_ratio(scroll_position);
        let max_thumb_travel = 100.0 - (self.thumb_size_ratio * 100.0);
        scroll_ratio * max_thumb_travel
    }
}
