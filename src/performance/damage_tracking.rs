//! Damage Tracking System - Phase 4.6
//! Tracks changed screen regions to minimize redrawing

use eframe::egui;

/// Screen region affected by UI changes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamagedRegion {
    pub rect: egui::Rect,
}

impl DamagedRegion {
    /// Create new damaged region from rect
    pub fn new(rect: egui::Rect) -> Self {
        Self { rect }
    }

    /// Merge with another region
    pub fn merge(&self, other: &DamagedRegion) -> DamagedRegion {
        DamagedRegion {
            rect: self.rect.union(other.rect),
        }
    }

    /// Check if regions intersect
    pub fn intersects(&self, other: &DamagedRegion) -> bool {
        self.rect.intersects(other.rect)
    }

    /// Get area of damaged region in pixels
    pub fn area(&self) -> f32 {
        self.rect.area()
    }
}

/// Damage tracker for tracking screen regions that need redrawing
pub struct DamageTracker {
    damaged_regions: Vec<DamagedRegion>,
    full_screen_damaged: bool,
    frame_number: u64,
}

impl DamageTracker {
    /// Create new damage tracker
    pub fn new() -> Self {
        Self {
            damaged_regions: vec![],
            full_screen_damaged: true, // First frame always damages full screen
            frame_number: 0,
        }
    }

    /// Mark region as damaged
    pub fn mark_damaged(&mut self, rect: egui::Rect) {
        if self.full_screen_damaged {
            return; // Already tracking full screen
        }

        let region = DamagedRegion::new(rect);

        // Try to merge with existing regions
        let mut merged = false;
        for damaged in &mut self.damaged_regions {
            if damaged.intersects(&region) {
                *damaged = damaged.merge(&region);
                merged = true;
                break;
            }
        }

        if !merged {
            self.damaged_regions.push(region);
        }
    }

    /// Mark entire screen as damaged
    pub fn mark_full_screen(&mut self) {
        self.full_screen_damaged = true;
        self.damaged_regions.clear();
    }

    /// Get all damaged regions for this frame
    pub fn get_damaged_regions(&self) -> &[DamagedRegion] {
        &self.damaged_regions
    }

    /// Check if full screen is damaged
    pub fn is_full_screen_damaged(&self) -> bool {
        self.full_screen_damaged
    }

    /// Reset damage tracking for next frame
    pub fn reset_frame(&mut self) {
        self.damaged_regions.clear();
        self.full_screen_damaged = false;
        self.frame_number += 1;
    }

    /// Get current frame number
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Get total damaged area
    pub fn total_damaged_area(&self) -> f32 {
        self.damaged_regions.iter().map(|r| r.area()).sum()
    }

    /// Get damage coverage percentage (0.0 to 1.0)
    pub fn damage_coverage(&self, screen_area: f32) -> f32 {
        if self.full_screen_damaged {
            1.0
        } else {
            (self.total_damaged_area() / screen_area).min(1.0)
        }
    }
}

impl Default for DamageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damaged_region_creation() {
        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        let region = DamagedRegion::new(rect);
        assert_eq!(region.rect, rect);
    }

    #[test]
    fn test_region_merge() {
        let rect1 = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        let rect2 = egui::Rect::from_min_max(
            egui::pos2(50.0, 50.0),
            egui::pos2(150.0, 150.0),
        );

        let region1 = DamagedRegion::new(rect1);
        let region2 = DamagedRegion::new(rect2);
        let merged = region1.merge(&region2);

        assert!(merged.area() >= region1.area());
        assert!(merged.area() >= region2.area());
    }

    #[test]
    fn test_region_intersection() {
        let rect1 = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        let rect2 = egui::Rect::from_min_max(
            egui::pos2(50.0, 50.0),
            egui::pos2(150.0, 150.0),
        );
        let rect3 = egui::Rect::from_min_max(
            egui::pos2(200.0, 200.0),
            egui::pos2(300.0, 300.0),
        );

        let region1 = DamagedRegion::new(rect1);
        let region2 = DamagedRegion::new(rect2);
        let region3 = DamagedRegion::new(rect3);

        assert!(region1.intersects(&region2));
        assert!(!region1.intersects(&region3));
    }

    #[test]
    fn test_damage_tracker_creation() {
        let tracker = DamageTracker::new();
        assert!(tracker.is_full_screen_damaged());
        assert_eq!(tracker.frame_number(), 0);
    }

    #[test]
    fn test_mark_damaged() {
        let mut tracker = DamageTracker::new();
        tracker.reset_frame();

        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        tracker.mark_damaged(rect);

        assert_eq!(tracker.get_damaged_regions().len(), 1);
        assert!(!tracker.is_full_screen_damaged());
    }

    #[test]
    fn test_reset_frame() {
        let mut tracker = DamageTracker::new();
        assert_eq!(tracker.frame_number(), 0);

        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        tracker.mark_damaged(rect);

        tracker.reset_frame();
        assert_eq!(tracker.frame_number(), 1);
        assert!(tracker.get_damaged_regions().is_empty());
        assert!(!tracker.is_full_screen_damaged());
    }

    #[test]
    fn test_damage_coverage() {
        let mut tracker = DamageTracker::new();
        assert_eq!(tracker.damage_coverage(10000.0), 1.0); // Full screen on first frame

        tracker.reset_frame();

        // Small rect (50x50 = 2500) on large screen (100000)
        let rect = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(50.0, 50.0),
        );
        tracker.mark_damaged(rect);

        let coverage = tracker.damage_coverage(100000.0);
        assert!(coverage > 0.0 && coverage < 1.0);
    }

    #[test]
    fn test_region_merging() {
        let mut tracker = DamageTracker::new();
        tracker.reset_frame();

        let rect1 = egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 100.0),
        );
        let rect2 = egui::Rect::from_min_max(
            egui::pos2(50.0, 50.0),
            egui::pos2(150.0, 150.0),
        );

        tracker.mark_damaged(rect1);
        tracker.mark_damaged(rect2);

        // Should merge overlapping regions
        assert_eq!(tracker.get_damaged_regions().len(), 1);
    }
}
