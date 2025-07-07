use p2panda_core::Hash;
use p2panda_net::TopicId;
use p2panda_sync::TopicQuery;
use serde::{Deserialize, Serialize};

/// Represents a chat group for p2p networking, identified by a BLAKE3 hash.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ChatGroup(Hash);

impl ChatGroup {
    /// Create a ChatGroup from an existing Hash.
    pub fn from_hash(hash: Hash) -> Self {
        Self(hash)
    }

    /// Get the underlying BLAKE3 hash.
    pub fn hash(&self) -> &Hash {
        &self.0
    }
}

impl TopicQuery for ChatGroup {}

impl TopicId for ChatGroup {
    /// Returns the 32-byte identifier for this topic.
    ///
    /// This is used by p2panda to identify and route messages for this chat group.
    fn id(&self) -> [u8; 32] {
        self.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_group_from_hash() {
        let hash = Hash::new("test-room".as_bytes());
        let group = ChatGroup::from_hash(hash.clone());
        assert_eq!(group.hash(), &hash);
    }

    #[test]
    fn test_topic_id() {
        let hash = Hash::new("test-room".as_bytes());
        let group = ChatGroup::from_hash(hash);
        let topic_id = group.id();
        assert_eq!(topic_id.len(), 32);

        // Should be deterministic - same hash gives same ID
        let hash2 = Hash::new("test-room".as_bytes());
        let group2 = ChatGroup::from_hash(hash2);
        assert_eq!(group.id(), group2.id());
    }

    #[test]
    fn test_different_hashes_different_ids() {
        let hash1 = Hash::new("room-one".as_bytes());
        let hash2 = Hash::new("room-two".as_bytes());
        let group1 = ChatGroup::from_hash(hash1);
        let group2 = ChatGroup::from_hash(hash2);
        assert_ne!(group1.id(), group2.id());
    }
}
