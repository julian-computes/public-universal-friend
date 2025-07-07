use anyhow::Result;
use p2panda_core::Hash;
use std::fmt;
use uuid::Uuid;

use crate::p2p::ChatGroup;

/// Represents a chat room with its identifier and metadata
#[derive(Debug, Clone)]
pub struct Room {
    pub hash: Hash,
    pub uuid: Uuid,
    pub name: String,
    pub identifier: String, // Format: hash-uuid-name
}

impl Room {
    /// Create a new room from a name, generating a BLAKE3 hash and UUID
    pub fn new(name: String) -> Self {
        let hash = Hash::new(name.as_bytes());
        let uuid = Uuid::new_v4();
        let identifier = format!("{}-{}-{}", hash, uuid, name);

        Self {
            hash,
            uuid,
            name,
            identifier,
        }
    }

    /// Parse a room identifier string (hash-uuid-name format)
    pub fn from_identifier(identifier: String) -> Result<Self> {
        // Find the first dash (after hash)
        let first_dash = identifier.find('-').ok_or_else(|| {
            anyhow::anyhow!("Invalid room identifier format. Expected: hash-uuid-name")
        })?;

        let _hash_str = &identifier[0..first_dash];
        let remainder = &identifier[first_dash + 1..];

        // UUID is always 36 characters (with dashes), so we can extract it precisely
        if remainder.len() < 36 {
            return Err(anyhow::anyhow!(
                "Invalid room identifier format. UUID section too short"
            ));
        }

        let uuid_str = &remainder[0..36];
        let name_part = &remainder[36..];

        // Remove leading dash from name if present
        let name = if name_part.starts_with('-') {
            name_part[1..].to_string()
        } else {
            return Err(anyhow::anyhow!(
                "Invalid room identifier format. Missing dash before name"
            ));
        };

        // Parse the UUID
        let uuid = Uuid::parse_str(uuid_str)
            .map_err(|e| anyhow::anyhow!("Invalid UUID in room identifier: {}", e))?;

        // Reconstruct the hash from the name for validation
        let hash = Hash::new(name.as_bytes());

        Ok(Self {
            hash,
            uuid,
            name,
            identifier,
        })
    }

    /// Create a ChatGroup for p2p networking from this room.
    pub fn to_chat_group(&self) -> ChatGroup {
        ChatGroup::from_hash(self.hash.clone())
    }
}

impl fmt::Display for Room {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

/// Copy text to system clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;

        child
            .stdin
            .take()
            .expect("pbcopy failed")
            .write_all(text.as_bytes())?;

        child.wait()?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try xclip first, then xsel
        let result = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .arg(text)
            .output();

        if result.is_err() {
            Command::new("xsel")
                .args(["--clipboard", "--input"])
                .arg(text)
                .output()?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("clip").arg(text).output()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new("my-test-room".to_string());
        assert_eq!(room.name, "my-test-room");
        assert!(room.identifier.contains("my-test-room"));

        // Should have 2 dashes: hash-uuid-name
        let dash_count = room.identifier.chars().filter(|&c| c == '-').count();
        assert!(dash_count >= 2);
    }

    #[test]
    fn test_room_from_identifier() {
        let original = Room::new("test-room".to_string());
        let parsed = Room::from_identifier(original.identifier.clone()).unwrap();
        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.uuid, original.uuid);
    }

    #[test]
    fn test_invalid_identifier() {
        let result = Room::from_identifier("invalid-format".to_string());
        assert!(result.is_err());

        let result2 = Room::from_identifier("hash-only".to_string());
        assert!(result2.is_err());
    }
}
