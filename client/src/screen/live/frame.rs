//! One encoded video frame and its wire format.

/// One encoded video frame and how to place it.
#[derive(Clone, Debug)]
pub struct VideoFrame {
    pub keyframe: bool,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

impl VideoFrame {
    /// Wire format: flags (bit 0 = keyframe), width, height (big-endian u16), AV1 OBUs.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + self.data.len());
        out.push(self.keyframe as u8);
        out.extend_from_slice(&self.width.to_be_bytes());
        out.extend_from_slice(&self.height.to_be_bytes());
        out.extend_from_slice(&self.data);
        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 5 {
            return None;
        }
        Some(Self {
            keyframe: bytes[0] & 1 == 1,
            width: u16::from_be_bytes([bytes[1], bytes[2]]),
            height: u16::from_be_bytes([bytes[3], bytes[4]]),
            data: bytes[5..].to_vec(),
        })
    }
}
