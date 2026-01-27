//! Input processing module for Surface Dial
//!
//! This module handles click detection (single, double, triple, long press)
//! and rotation sensitivity/dead zone processing.

use std::time::Instant;

/// Configuration for click detection timing
#[derive(Debug, Clone)]
pub struct ClickConfig {
    /// Maximum time between clicks for double-click detection (ms)
    pub double_click_ms: u64,
    /// Maximum time between clicks for triple-click detection (ms)
    pub triple_click_ms: u64,
    /// Minimum hold time for long press detection (ms)
    pub long_press_ms: u64,
}

impl Default for ClickConfig {
    fn default() -> Self {
        Self {
            double_click_ms: 400,
            triple_click_ms: 600,
            long_press_ms: 1000,
        }
    }
}

impl ClickConfig {
    /// Create a ClickConfig from the application config
    pub fn from_config(config: &crate::config::InteractionConfig) -> Self {
        Self {
            double_click_ms: config.double_click_ms as u64,
            triple_click_ms: config.triple_click_ms as u64,
            long_press_ms: config.long_press_ms as u64,
        }
    }
}

/// Result of click detection processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    /// No action to take
    None,
    /// Single click detected (after double-click window expired)
    SingleClick,
    /// Double click detected
    DoubleClick,
    /// Triple click detected
    TripleClick,
    /// Long press started (button still held)
    LongPressStart,
    /// Long press ended (button released)
    LongPressEnd,
}

/// State machine for detecting various click patterns
#[derive(Debug)]
pub struct ClickDetector {
    config: ClickConfig,
    /// Time of the last button press
    button_press_start: Option<Instant>,
    /// Time of the last button release (for multi-click detection)
    last_release_time: Option<Instant>,
    /// Number of clicks in the current sequence
    click_count: u8,
    /// Whether a long press has been fired for the current hold
    long_press_fired: bool,
    /// Whether the button is currently pressed
    button_down: bool,
}

impl ClickDetector {
    /// Create a new click detector with the given configuration
    pub fn new(config: ClickConfig) -> Self {
        Self {
            config,
            button_press_start: None,
            last_release_time: None,
            click_count: 0,
            long_press_fired: false,
            button_down: false,
        }
    }

    /// Update the click detector configuration
    pub fn update_config(&mut self, config: ClickConfig) {
        self.config = config;
    }

    /// Process a button down event
    pub fn button_down(&mut self) -> ClickResult {
        if self.button_down {
            return ClickResult::None;
        }

        self.button_down = true;
        self.button_press_start = Some(Instant::now());
        self.long_press_fired = false;
        ClickResult::None
    }

    /// Process a button up event
    pub fn button_up(&mut self) -> ClickResult {
        if !self.button_down {
            return ClickResult::None;
        }

        self.button_down = false;
        let now = Instant::now();

        // If long press was fired, this is the end of it
        if self.long_press_fired {
            self.button_press_start = None;
            self.click_count = 0;
            self.last_release_time = None;
            return ClickResult::LongPressEnd;
        }

        // Check if this is part of a multi-click sequence
        let is_continuation = self
            .last_release_time
            .map(|t| now.duration_since(t).as_millis() < self.config.triple_click_ms as u128)
            .unwrap_or(false);

        if is_continuation {
            self.click_count += 1;
        } else {
            self.click_count = 1;
        }

        self.last_release_time = Some(now);
        self.button_press_start = None;

        // Check for triple click
        if self.click_count >= 3 {
            self.click_count = 0;
            self.last_release_time = None;
            return ClickResult::TripleClick;
        }

        // Double click is detected immediately on second click
        if self.click_count == 2 {
            // Check if within double-click window
            if let Some(last) = self.last_release_time {
                if now.duration_since(last).as_millis() < self.config.double_click_ms as u128 {
                    self.click_count = 0;
                    self.last_release_time = None;
                    return ClickResult::DoubleClick;
                }
            }
        }

        ClickResult::None
    }

    /// Called periodically to check for timed events (long press, single click timeout)
    pub fn tick(&mut self) -> ClickResult {
        let now = Instant::now();

        // Check for long press while button is held
        if self.button_down {
            if let Some(start) = self.button_press_start {
                if !self.long_press_fired
                    && now.duration_since(start).as_millis() >= self.config.long_press_ms as u128
                {
                    self.long_press_fired = true;
                    return ClickResult::LongPressStart;
                }
            }
        }

        // Check for single click timeout (no second click came)
        if self.click_count == 1 && !self.button_down {
            if let Some(last) = self.last_release_time {
                if now.duration_since(last).as_millis() >= self.config.double_click_ms as u128 {
                    self.click_count = 0;
                    self.last_release_time = None;
                    return ClickResult::SingleClick;
                }
            }
        }

        ClickResult::None
    }

    /// Reset the detector state
    pub fn reset(&mut self) {
        self.button_press_start = None;
        self.last_release_time = None;
        self.click_count = 0;
        self.long_press_fired = false;
        self.button_down = false;
    }

    /// Check if a long press is currently active
    pub fn is_long_pressing(&self) -> bool {
        self.long_press_fired && self.button_down
    }
}

/// Configuration for rotation sensitivity
#[derive(Debug, Clone)]
pub struct SensitivityConfig {
    /// Dead zone threshold (ignore small rotations)
    pub dead_zone: i32,
    /// Sensitivity multiplier
    pub multiplier: f64,
    /// Invert rotation direction
    pub invert: bool,
}

impl Default for SensitivityConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0,
            multiplier: 1.0,
            invert: false,
        }
    }
}

impl SensitivityConfig {
    /// Create a SensitivityConfig from the application config
    pub fn from_config(config: &crate::config::SensitivityConfig) -> Self {
        Self {
            dead_zone: config.dead_zone,
            multiplier: config.multiplier,
            invert: config.invert,
        }
    }
}

/// Processor for rotation input with dead zone and sensitivity
#[derive(Debug)]
pub struct RotationProcessor {
    config: SensitivityConfig,
    /// Accumulated rotation within dead zone
    accumulated: i32,
}

impl RotationProcessor {
    /// Create a new rotation processor with the given configuration
    pub fn new(config: SensitivityConfig) -> Self {
        Self {
            config,
            accumulated: 0,
        }
    }

    /// Update the processor configuration
    pub fn update_config(&mut self, config: SensitivityConfig) {
        self.config = config;
    }

    /// Process a raw rotation value
    ///
    /// Returns Some(adjusted_rotation) if the rotation exceeds the dead zone,
    /// None if within the dead zone.
    pub fn process(&mut self, raw_rotation: i8) -> Option<i32> {
        let rotation = if self.config.invert {
            -(raw_rotation as i32)
        } else {
            raw_rotation as i32
        };

        // Apply dead zone
        self.accumulated += rotation;

        if self.accumulated.abs() > self.config.dead_zone {
            let result = self.accumulated;
            self.accumulated = 0;

            // Apply multiplier
            let adjusted = (result as f64 * self.config.multiplier).round() as i32;
            Some(adjusted)
        } else {
            None
        }
    }

    /// Reset the accumulated rotation
    pub fn reset(&mut self) {
        self.accumulated = 0;
    }
}

/// Calculate volume step based on rotation speed (acceleration)
pub fn calculate_step(
    last_rotation: Option<Instant>,
    min_step: i32,
    max_step: i32,
    fast_ms: u64,
    slow_ms: u64,
) -> i32 {
    let Some(last) = last_rotation else {
        return min_step;
    };

    let elapsed_ms = last.elapsed().as_millis() as u64;

    if elapsed_ms <= fast_ms {
        max_step
    } else if elapsed_ms >= slow_ms {
        min_step
    } else {
        // Linear interpolation between min and max
        let range = slow_ms - fast_ms;
        let pos = elapsed_ms - fast_ms;
        let ratio = 1.0 - (pos as f64 / range as f64);
        let step_range = (max_step - min_step) as f64;
        min_step + (ratio * step_range) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_click_detector_single_click() {
        let config = ClickConfig {
            double_click_ms: 100,
            triple_click_ms: 150,
            long_press_ms: 500,
        };
        let mut detector = ClickDetector::new(config);

        // Press and release
        assert_eq!(detector.button_down(), ClickResult::None);
        assert_eq!(detector.button_up(), ClickResult::None);

        // Wait for double-click window to expire
        sleep(Duration::from_millis(150));
        assert_eq!(detector.tick(), ClickResult::SingleClick);
    }

    #[test]
    fn test_click_detector_double_click() {
        let config = ClickConfig {
            double_click_ms: 200,
            triple_click_ms: 300,
            long_press_ms: 500,
        };
        let mut detector = ClickDetector::new(config);

        // First click
        detector.button_down();
        detector.button_up();

        // Second click within window
        sleep(Duration::from_millis(50));
        detector.button_down();
        let result = detector.button_up();
        assert_eq!(result, ClickResult::DoubleClick);
    }

    #[test]
    fn test_click_detector_long_press() {
        let config = ClickConfig {
            double_click_ms: 200,
            triple_click_ms: 300,
            long_press_ms: 100,
        };
        let mut detector = ClickDetector::new(config);

        detector.button_down();

        // Wait for long press
        sleep(Duration::from_millis(150));
        assert_eq!(detector.tick(), ClickResult::LongPressStart);

        // Release
        assert_eq!(detector.button_up(), ClickResult::LongPressEnd);
    }

    #[test]
    fn test_rotation_processor_no_dead_zone() {
        let config = SensitivityConfig {
            dead_zone: 0,
            multiplier: 1.0,
            invert: false,
        };
        let mut processor = RotationProcessor::new(config);

        assert_eq!(processor.process(1), Some(1));
        assert_eq!(processor.process(-1), Some(-1));
        assert_eq!(processor.process(3), Some(3));
    }

    #[test]
    fn test_rotation_processor_with_dead_zone() {
        let config = SensitivityConfig {
            dead_zone: 2,
            multiplier: 1.0,
            invert: false,
        };
        let mut processor = RotationProcessor::new(config);

        // Within dead zone
        assert_eq!(processor.process(1), None);
        assert_eq!(processor.process(1), None);

        // Exceeds dead zone
        assert_eq!(processor.process(1), Some(3));
    }

    #[test]
    fn test_rotation_processor_invert() {
        let config = SensitivityConfig {
            dead_zone: 0,
            multiplier: 1.0,
            invert: true,
        };
        let mut processor = RotationProcessor::new(config);

        assert_eq!(processor.process(1), Some(-1));
        assert_eq!(processor.process(-3), Some(3));
    }

    #[test]
    fn test_rotation_processor_multiplier() {
        let config = SensitivityConfig {
            dead_zone: 0,
            multiplier: 2.0,
            invert: false,
        };
        let mut processor = RotationProcessor::new(config);

        assert_eq!(processor.process(3), Some(6));
    }

    #[test]
    fn test_calculate_step_slow() {
        // No previous rotation
        assert_eq!(calculate_step(None, 2, 8, 80, 400), 2);

        // Old rotation (slow)
        let old = Instant::now() - Duration::from_millis(500);
        assert_eq!(calculate_step(Some(old), 2, 8, 80, 400), 2);
    }

    #[test]
    fn test_calculate_step_fast() {
        // Very recent rotation (fast)
        let recent = Instant::now() - Duration::from_millis(50);
        assert_eq!(calculate_step(Some(recent), 2, 8, 80, 400), 8);
    }
}
