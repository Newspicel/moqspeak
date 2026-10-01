//! Round trips through the wire format and the codec, and the speed of the encoder.

mod round_trip {
    use super::super::*;

    #[test]
    fn video_frame_round_trips() {
        let f = VideoFrame {
            keyframe: true,
            width: 1280,
            height: 720,
            data: vec![9, 8, 7],
        };
        let g = VideoFrame::decode(&f.encode()).unwrap();
        assert!(g.keyframe);
        assert_eq!((g.width, g.height, g.data), (1280, 720, vec![9, 8, 7]));
    }

    #[test]
    fn av1_round_trips_through_rav1e_and_rav1d() {
        let (w, h) = (1280usize, 720usize);
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                rgba[i] = (x * 4) as u8;
                rgba[i + 1] = (y * 5) as u8;
                rgba[i + 2] = 128;
                rgba[i + 3] = 255;
            }
        }
        let mut enc = encode::Encoder::new(w, h, 10).unwrap();
        let mut dec = Decoder::new().unwrap();
        let mut decoded = None;
        for _ in 0..6 {
            let frames = enc.encode(&rgba, w, h).unwrap();
            for frame in frames {
                let r = dec.decode(&frame.data).unwrap();
                if let Some(pic) = r {
                    decoded = Some(pic);
                }
            }
        }
        let (dw, dh, out) = decoded.expect("a decoded picture");
        // A viewer that joins at a later keyframe decodes too.
        let mut late = Decoder::new().unwrap();
        for _ in 0..3 {
            enc.encode(&rgba, w, h).unwrap();
        }
        let mut late_picture = None;
        for _ in 0..(super::super::encode::KEY_INTERVAL as usize + 6) {
            for frame in enc.encode(&rgba, w, h).unwrap() {
                // Like a real late joiner, it is handed delta frames before its first keyframe.
                if let Ok(Some(p)) = late.decode(&frame.data) {
                    late_picture = Some(p);
                }
            }
        }
        assert!(late_picture.is_some(), "late joiner decoded nothing");
        assert_eq!((dw, dh), (w as u32, h as u32));
        // The middle pixel survives within codec tolerance.
        let i = ((h / 2) * w + w / 2) * 4;
        for c in 0..3 {
            assert!(
                (out[i + c] as i32 - rgba[i + c] as i32).abs() < 24,
                "channel {c}"
            );
        }
    }
}

mod bench {
    /// `cargo test --release -p moqspeak encode_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn encode_speed() {
        let (w, h) = (1600usize, 1000usize);
        let mut rgba = vec![0u8; w * h * 4];
        let mut enc = super::super::encode::Encoder::new(w, h, 10).unwrap();
        let start = std::time::Instant::now();
        let mut bytes = 0;
        for f in 0..40 {
            for (i, px) in rgba.chunks_mut(4).enumerate() {
                let x = (i % w + f * 7) as u8;
                px.copy_from_slice(&[x, (i / w) as u8, x ^ 0x55, 255]);
            }
            bytes += enc
                .encode(&rgba, w, h)
                .unwrap()
                .iter()
                .map(|p| p.data.len())
                .sum::<usize>();
        }
        let secs = start.elapsed().as_secs_f64();
        eprintln!("{:.1} fps, {} kB total", 40.0 / secs, bytes / 1000);
    }
}

mod live_path {
    use super::super::*;

    /// The test-pattern sharer, through the wire format, into a decoder that joins late.
    #[test]
    fn test_pattern_decodes_after_late_join() {
        let (tx, rx) = std::sync::mpsc::channel();
        let sharer = Sharer::test_pattern(move |f| {
            let _ = tx.send(f.encode());
        })
        .unwrap();
        // Skip the first second so the decoder starts mid-stream.
        std::thread::sleep(std::time::Duration::from_millis(1000));
        while rx.try_recv().is_ok() {}
        let mut decoder = Decoder::new().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut got = None;
        let (mut frames, mut keys) = (0, 0);
        while std::time::Instant::now() < deadline && got.is_none() {
            let Ok(bytes) = rx.recv_timeout(std::time::Duration::from_millis(500)) else {
                continue;
            };
            let frame = VideoFrame::decode(&bytes).unwrap();
            frames += 1;
            keys += frame.keyframe as u32;
            match decoder.decode(&frame.data) {
                Ok(Some(p)) => got = Some((p.0, p.1)),
                Ok(None) => {}
                Err(e) => eprintln!("decode error: {e:#}"),
            }
        }
        drop(sharer);
        assert_eq!(got, Some((1280, 720)), "frames={frames} keys={keys}");
    }
}
