//! Just enough AV1 OBU parsing to make every keyframe self-contained.
//!
//! rav1e writes the sequence header into the first keyframe only. A viewer who joins later needs
//! it in front of whichever keyframe it starts from, so the sender copies it into each one.

const OBU_SEQUENCE_HEADER: u8 = 1;
const OBU_TEMPORAL_DELIMITER: u8 = 2;

/// One OBU's type and its byte range in the packet.
struct Obu {
    kind: u8,
    start: usize,
    end: usize,
}

fn leb128(data: &[u8]) -> Option<(usize, usize)> {
    let mut value = 0usize;
    for (i, byte) in data.iter().take(8).enumerate() {
        value |= ((byte & 0x7f) as usize) << (i * 7);
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

fn walk(data: &[u8]) -> Vec<Obu> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let header = data[pos];
        let kind = (header >> 3) & 0x0f;
        let has_extension = header & 0x04 != 0;
        let has_size = header & 0x02 != 0;
        let mut cursor = pos + 1 + has_extension as usize;
        let end = if has_size {
            let Some((size, len)) = data.get(cursor..).and_then(leb128) else {
                break;
            };
            cursor += len;
            cursor + size
        } else {
            data.len()
        };
        if end > data.len() {
            break;
        }
        out.push(Obu {
            kind,
            start: pos,
            end,
        });
        pos = end;
    }
    out
}

/// Remembers the stream's sequence header and puts it into keyframes that lack one.
#[derive(Default)]
pub struct SequenceHeaderInserter {
    header: Option<Vec<u8>>,
}

impl SequenceHeaderInserter {
    pub fn process(&mut self, packet: Vec<u8>, keyframe: bool) -> Vec<u8> {
        let obus = walk(&packet);
        if let Some(seq) = obus.iter().find(|o| o.kind == OBU_SEQUENCE_HEADER) {
            self.header = Some(packet[seq.start..seq.end].to_vec());
            return packet;
        }
        let (true, Some(header)) = (keyframe, &self.header) else {
            return packet;
        };
        // The header goes after the temporal delimiter, when there is one.
        let at = obus
            .first()
            .filter(|o| o.kind == OBU_TEMPORAL_DELIMITER)
            .map(|o| o.end)
            .unwrap_or(0);
        let mut out = Vec::with_capacity(packet.len() + header.len());
        out.extend_from_slice(&packet[..at]);
        out.extend_from_slice(header);
        out.extend_from_slice(&packet[at..]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_after_temporal_delimiter() {
        let td = [0x12, 0x00];
        let seq = [0x0a, 0x02, 0xaa, 0xbb];
        let frame = [0x32, 0x01, 0xcc];
        let first: Vec<u8> = [&td[..], &seq, &frame].concat();
        let mut ins = SequenceHeaderInserter::default();
        assert_eq!(ins.process(first.clone(), true), first);
        let later: Vec<u8> = [&td[..], &frame].concat();
        assert_eq!(ins.process(later.clone(), true), first);
        assert_eq!(ins.process(later.clone(), false), later);
    }
}
