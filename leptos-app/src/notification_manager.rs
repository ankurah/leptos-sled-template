use std::collections::HashMap;

/// Stub NotificationManager for tracking unread message counts per room.
///
/// This will eventually integrate with the Ankurah message system to track
/// unread counts, play notification sounds, etc. For now it's just a placeholder
/// so RoomList can render unread badges.
#[derive(Clone)]
pub struct NotificationManager {
    // Stub: in the real implementation this would be a signal or live query
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Get unread message counts by room ID (base64).
    pub fn unread_counts(&self) -> HashMap<String, usize> {
        // Stub: return empty map for now
        HashMap::new()
    }
}
