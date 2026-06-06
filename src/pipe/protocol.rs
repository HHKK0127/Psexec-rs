use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Settings = 1,
    RespSendFiles = 2,
    SentFiles = 3,
    StartApp = 4,
    Ok = 5,
    Failed = 6,
    Output = 7,
    ExecutionComplete = 8,
}

impl From<u32> for MessageType {
    fn from(v: u32) -> Self {
        match v {
            1 => MessageType::Settings,
            2 => MessageType::RespSendFiles,
            3 => MessageType::SentFiles,
            4 => MessageType::StartApp,
            5 => MessageType::Ok,
            6 => MessageType::Failed,
            7 => MessageType::Output,
            8 => MessageType::ExecutionComplete,
            _ => MessageType::Failed,
        }
    }
}

impl Into<u32> for MessageType {
    fn into(self) -> u32 {
        match self {
            MessageType::Settings => 1,
            MessageType::RespSendFiles => 2,
            MessageType::SentFiles => 3,
            MessageType::StartApp => 4,
            MessageType::Ok => 5,
            MessageType::Failed => 6,
            MessageType::Output => 7,
            MessageType::ExecutionComplete => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub msg_type: u32,
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(msg_type: MessageType, data: Vec<u8>) -> Self {
        Message {
            msg_type: msg_type.into(),
            data,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Write message type (4 bytes, little-endian)
        buffer.extend_from_slice(&(self.msg_type as u32).to_le_bytes());

        // Write data length (4 bytes, little-endian)
        buffer.extend_from_slice(&(self.data.len() as u32).to_le_bytes());

        // Write data
        buffer.extend_from_slice(&self.data);

        buffer
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("Message too short".to_string());
        }

        let msg_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if data.len() < 8 + len {
            return Err(format!("Incomplete message data: expected {}, got {}", len, data.len() - 8));
        }

        let payload = data[8..8 + len].to_vec();

        Ok(Message {
            msg_type,
            data: payload,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSettings {
    pub command: String,
    pub working_directory: Option<String>,
    pub priority: Option<u32>,
    pub env_vars: Option<std::collections::HashMap<String, String>>,
}

impl ExecutionSettings {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(data)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecutionResult {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::new(MessageType::Output, b"test output".to_vec());
        let serialized = msg.serialize();

        assert!(serialized.len() >= 8);
        assert_eq!(u32::from_le_bytes([serialized[0], serialized[1], serialized[2], serialized[3]]), 7);
    }

    #[test]
    fn test_message_deserialization() {
        let original = Message::new(MessageType::Output, b"test output".to_vec());
        let serialized = original.serialize();
        let deserialized = Message::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.msg_type, 7);
        assert_eq!(deserialized.data, b"test output".to_vec());
    }

    #[test]
    fn test_execution_settings() {
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("TEST".to_string(), "value".to_string());

        let settings = ExecutionSettings {
            command: "whoami".to_string(),
            working_directory: Some("C:\\".to_string()),
            priority: Some(32),
            env_vars: Some(env_vars),
        };

        let bytes = settings.to_bytes().unwrap();
        let deserialized = ExecutionSettings::from_bytes(&bytes).unwrap();

        assert_eq!(deserialized.command, "whoami");
        assert_eq!(deserialized.priority, Some(32));
    }
}
