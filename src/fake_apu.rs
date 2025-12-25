#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeApuUploadState {
    WaitingCc,
    Uploading,
    Done,
}

impl Default for FakeApuUploadState {
    fn default() -> Self {
        Self::WaitingCc
    }
}

#[derive(Default)]
pub struct FakeApuUpload {
    pub state: FakeApuUploadState,
    pub buf: Vec<u8>,
    pub port_shadow: [u8; 4],
    pub total_bytes: u32,
}

impl FakeApuUpload {
    pub fn reset(&mut self) {
        self.state = FakeApuUploadState::WaitingCc;
        self.buf.clear();
        self.port_shadow = [0xAA, 0xBB, 0x00, 0x00];
        self.total_bytes = 0;
    }
}
