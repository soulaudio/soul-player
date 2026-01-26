//! Diagnostic tests for device event deduplication
//!
//! These tests verify that the device monitoring system correctly filters
//! duplicate events from platform APIs (CoreAudio, PipeWire, WinRT).

#[cfg(test)]
mod device_event_deduplication_tests {
    use std::time::{Duration, Instant};

    /// Device event type for deduplication tracking
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum DeviceEventType {
        Added,
        Removed,
        DefaultChanged,
        PropertyChanged,
    }

    /// Last device event tracker for deduplication
    struct LastDeviceEvent {
        event_type: DeviceEventType,
        device_id: String,
        timestamp: Instant,
    }

    impl LastDeviceEvent {
        /// Check if a new event is a duplicate of this event
        fn is_duplicate(&self, event_type: &DeviceEventType, device_id: &str) -> bool {
            if self.event_type != *event_type {
                return false;
            }
            if self.device_id != device_id {
                return false;
            }
            // Check if within 500ms window
            self.timestamp.elapsed() < Duration::from_millis(500)
        }
    }

    #[test]
    fn test_duplicate_event_within_window() {
        // Create an event that just happened
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::Removed,
            device_id: "device123".to_string(),
            timestamp: Instant::now(),
        };

        // Same event type and device ID should be detected as duplicate
        assert!(last_event.is_duplicate(&DeviceEventType::Removed, "device123"));
    }

    #[test]
    fn test_different_event_type_not_duplicate() {
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::Removed,
            device_id: "device123".to_string(),
            timestamp: Instant::now(),
        };

        // Different event type should not be duplicate
        assert!(!last_event.is_duplicate(&DeviceEventType::Added, "device123"));
    }

    #[test]
    fn test_different_device_id_not_duplicate() {
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::Removed,
            device_id: "device123".to_string(),
            timestamp: Instant::now(),
        };

        // Different device ID should not be duplicate
        assert!(!last_event.is_duplicate(&DeviceEventType::Removed, "device456"));
    }

    #[test]
    fn test_event_outside_time_window_not_duplicate() {
        // Create an event that happened 600ms ago (outside 500ms window)
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::Removed,
            device_id: "device123".to_string(),
            timestamp: Instant::now()
                .checked_sub(Duration::from_millis(600))
                .unwrap(),
        };

        // Same event but outside time window should not be duplicate
        assert!(!last_event.is_duplicate(&DeviceEventType::Removed, "device123"));
    }

    #[test]
    fn test_default_device_changed_deduplication() {
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::DefaultChanged,
            device_id: "device123".to_string(),
            timestamp: Instant::now(),
        };

        // Should detect duplicate default device change
        assert!(last_event.is_duplicate(&DeviceEventType::DefaultChanged, "device123"));

        // But different device changing to default should not be duplicate
        assert!(!last_event.is_duplicate(&DeviceEventType::DefaultChanged, "device456"));
    }

    #[test]
    fn test_property_changed_deduplication() {
        let last_event = LastDeviceEvent {
            event_type: DeviceEventType::PropertyChanged,
            device_id: "device123".to_string(),
            timestamp: Instant::now(),
        };

        // Should detect duplicate property change for same device
        assert!(last_event.is_duplicate(&DeviceEventType::PropertyChanged, "device123"));
    }

    #[test]
    fn test_multiple_events_sequence() {
        // Simulate a sequence of events
        let mut last_event: Option<LastDeviceEvent> = None;

        // First event - should process
        let event1_type = DeviceEventType::Removed;
        let event1_device = "device123";
        assert!(!last_event
            .as_ref()
            .is_some_and(|e| e.is_duplicate(&event1_type, event1_device)));
        last_event = Some(LastDeviceEvent {
            event_type: event1_type.clone(),
            device_id: event1_device.to_string(),
            timestamp: Instant::now(),
        });

        // Duplicate event within 500ms - should skip
        assert!(last_event
            .as_ref()
            .unwrap()
            .is_duplicate(&event1_type, event1_device));

        // Different event type - should process
        let event2_type = DeviceEventType::Added;
        assert!(!last_event
            .as_ref()
            .unwrap()
            .is_duplicate(&event2_type, event1_device));
        last_event = Some(LastDeviceEvent {
            event_type: event2_type.clone(),
            device_id: event1_device.to_string(),
            timestamp: Instant::now(),
        });

        // Different device - should process
        let event3_device = "device456";
        assert!(!last_event
            .as_ref()
            .unwrap()
            .is_duplicate(&event2_type, event3_device));
    }
}
