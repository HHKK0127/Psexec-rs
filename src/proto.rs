use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
    PIPE_TYPE_MESSAGE, PIPE_READMODE_MESSAGE, PIPE_UNLIMITED_INSTANCES,
    NMPWAIT_USE_DEFAULT_WAIT,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;

pub const MSGID_SETTINGS: u16 = 1;
pub const MSGID_RESP_SEND_FILES: u16 = 2;
pub const MSGID_SENT_FILES: u16 = 3;
pub const MSGID_OK: u16 = 4;
pub const MSGID_START_APP: u16 = 5;
pub const MSGID_FAILED: u16 = 6;

const CURRENT_SERIALIZE_VERSION: u32 = 1;

fn get_unique_process_id() -> u32 {
    let pid = std::process::id();
    let hostname = hostname();
    let mut id = pid;
    for chunk in hostname.as_bytes().chunks(4) {
        let mut val = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            val |= (b as u32) << (i * 8);
        }
        id ^= val;
    }
    id
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN".into())
}

#[derive(Clone)]
pub struct RemMsg {
    pub msg_id: u16,
    pub unique_process_id: u32,
    pub payload: Vec<u8>,
    pub expected_len: u32,
    read_pos: usize,
}

impl RemMsg {
    pub fn new(msg_id: u16) -> Self {
        Self {
            msg_id,
            unique_process_id: get_unique_process_id(),
            payload: Vec::new(),
            expected_len: 0,
            read_pos: 0,
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 8 {
            return Err("data too short");
        }
        let mut offset = 0;
        let msg_id = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let xor_key = if msg_id == MSGID_SETTINGS {
            if data.len() < offset + 4 {
                return Err("data too short for xor key");
            }
            let key = u32::from_le_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            ]);
            offset += 4;
            Some(key)
        } else {
            None
        };

        let mut buf = data[offset..].to_vec();

        if let Some(key) = xor_key {
            let mut k = key;
            for i in 0..buf.len().saturating_sub(3) {
                let dw = u32::from_le_bytes([
                    buf[i], buf[i + 1], buf[i + 2], buf[i + 3],
                ]);
                let decoded = dw ^ k;
                buf[i..i + 4].copy_from_slice(&decoded.to_le_bytes());
                k = k.wrapping_add(3);
            }
        }

        if buf.len() < 8 {
            return Err("data too short after xor");
        }

        let unique_process_id = u32::from_le_bytes([
            buf[0], buf[1], buf[2], buf[3],
        ]);
        let expected_len = u32::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7],
        ]);
        let payload = buf[8..].to_vec();

        Ok(Self {
            msg_id,
            unique_process_id,
            payload,
            expected_len,
            read_pos: 0,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.msg_id.to_le_bytes());

        if self.msg_id == MSGID_SETTINGS {
            use rand::Rng;
            let xor_key: u32 = rand::thread_rng().gen();
            data.extend_from_slice(&xor_key.to_le_bytes());

            let mut body = Vec::new();
            body.extend_from_slice(&self.unique_process_id.to_le_bytes());
            body.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
            body.extend_from_slice(&self.payload);

            let mut k = xor_key;
            for i in 0..body.len().saturating_sub(3) {
                let dw = u32::from_le_bytes([
                    body[i], body[i + 1], body[i + 2], body[i + 3],
                ]);
                let encoded = dw ^ k;
                body[i..i + 4].copy_from_slice(&encoded.to_le_bytes());
                k = k.wrapping_add(3);
            }
            data.extend_from_slice(&body);
        } else {
            data.extend_from_slice(&self.unique_process_id.to_le_bytes());
            data.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
            data.extend_from_slice(&self.payload);
        }

        data
    }

    pub fn serialize_string(&mut self, s: &str) {
        let len = s.len() as u32;
        self.payload.extend_from_slice(&len.to_le_bytes());
        self.payload.extend_from_slice(s.as_bytes());
    }

    pub fn serialize_u32(&mut self, v: u32) {
        self.payload.extend_from_slice(&v.to_le_bytes());
    }

    pub fn serialize_u64(&mut self, v: u64) {
        self.payload.extend_from_slice(&v.to_le_bytes());
    }

    pub fn serialize_bool(&mut self, v: bool) {
        self.payload.push(if v { 1 } else { 0 });
    }

    pub fn deserialize_string(&mut self) -> String {
        if self.read_pos + 4 > self.payload.len() {
            return String::new();
        }
        let len = u32::from_le_bytes([
            self.payload[self.read_pos],
            self.payload[self.read_pos + 1],
            self.payload[self.read_pos + 2],
            self.payload[self.read_pos + 3],
        ]) as usize;
        self.read_pos += 4;
        if self.read_pos + len > self.payload.len() {
            return String::new();
        }
        let s = String::from_utf8_lossy(&self.payload[self.read_pos..self.read_pos + len]);
        self.read_pos += len;
        s.to_string()
    }

    pub fn deserialize_u32(&mut self) -> u32 {
        if self.read_pos + 4 > self.payload.len() {
            return 0;
        }
        let v = u32::from_le_bytes([
            self.payload[self.read_pos],
            self.payload[self.read_pos + 1],
            self.payload[self.read_pos + 2],
            self.payload[self.read_pos + 3],
        ]);
        self.read_pos += 4;
        v
    }

    pub fn deserialize_u64(&mut self) -> u64 {
        if self.read_pos + 8 > self.payload.len() {
            return 0;
        }
        let v = u64::from_le_bytes([
            self.payload[self.read_pos],
            self.payload[self.read_pos + 1],
            self.payload[self.read_pos + 2],
            self.payload[self.read_pos + 3],
            self.payload[self.read_pos + 4],
            self.payload[self.read_pos + 5],
            self.payload[self.read_pos + 6],
            self.payload[self.read_pos + 7],
        ]);
        self.read_pos += 8;
        v
    }

    pub fn deserialize_bool(&mut self) -> bool {
        if self.read_pos >= self.payload.len() {
            return false;
        }
        let v = self.payload[self.read_pos] != 0;
        self.read_pos += 1;
        v
    }
}

pub struct NamedPipe {
    handle: HANDLE,
}

impl NamedPipe {
    pub fn create_server(name: &str) -> Result<Self, windows::core::Error> {
        let wide: Vec<u16> = std::os::windows::ffi::OsStrExt::encode_wide(
            std::ffi::OsStr::new(name),
        )
        .chain(Some(0))
        .collect();

        unsafe {
            const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
            let handle = CreateNamedPipeW(
                windows::core::PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX),
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE,
                PIPE_UNLIMITED_INSTANCES,
                16384u32,
                16384u32,
                NMPWAIT_USE_DEFAULT_WAIT,
                None,
            );
            if handle.is_invalid() {
                return Err(windows::core::Error::from_win32());
            }
            Ok(Self { handle })
        }
    }

    pub fn wait_for_client(&self) -> Result<(), windows::core::Error> {
        unsafe {
            ConnectNamedPipe(self.handle, None)?;
            Ok(())
        }
    }

    pub fn connect_client(name: &str) -> Result<Self, windows::core::Error> {
        let wide: Vec<u16> = std::os::windows::ffi::OsStrExt::encode_wide(
            std::ffi::OsStr::new(name),
        )
        .chain(Some(0))
        .collect();

        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
        };
        use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};

        unsafe {
            let handle = CreateFileW(
                windows::core::PCWSTR(wide.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )?;
            Ok(Self { handle })
        }
    }

    pub fn send_msg(&self, msg: &RemMsg) -> Result<(), windows::core::Error> {
        let data = msg.to_bytes();
        unsafe {
            use windows::Win32::Storage::FileSystem::WriteFile;
            let mut written = 0u32;
            WriteFile(self.handle, Some(&data), Some(&mut written), None)?;
            Ok(())
        }
    }

    pub fn recv_msg(&self) -> Result<RemMsg, windows::core::Error> {
        unsafe {
            use windows::Win32::Storage::FileSystem::ReadFile;
            let mut header = [0u8; 8];
            let mut total_header_read = 0u32;

            while total_header_read < header.len() as u32 {
                let mut bytes_this_call = 0u32;
                let result = ReadFile(
                    self.handle,
                    Some(&mut header[total_header_read as usize..]),
                    Some(&mut bytes_this_call),
                    None,
                );
                if result.is_err() {
                    break;
                }
                if bytes_this_call == 0 {
                    return Err(windows::core::Error::from_win32());
                }
                total_header_read += bytes_this_call;
            }

            let msg_id = u16::from_le_bytes([header[0], header[1]]);
            let header_size = if msg_id == MSGID_SETTINGS { 6u32 } else { 2u32 };

            let mut body = Vec::new();

            loop {
                let mut tmp = [0u8; 16384];
                let mut bytes_this_call = 0u32;
                let result = ReadFile(self.handle, Some(&mut tmp), Some(&mut bytes_this_call), None);
                if result.is_err() {
                    break;
                }
                if bytes_this_call == 0 {
                    break;
                }
                body.extend_from_slice(&tmp[..bytes_this_call as usize]);
            }

            let mut full = header[..header_size as usize].to_vec();
            full.extend_from_slice(&body);
            RemMsg::from_bytes(&full).map_err(|_| {
                windows::core::Error::from_win32()
            })
        }
    }

    pub fn close(&self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.handle);
        }
    }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.handle);
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
