use std::path::Path;

#[must_use]
pub fn wav_duration_secs(path: &Path) -> Option<f64> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let mut pos: usize = 12;
    while pos.checked_add(8).is_some_and(|end| end <= data.len()) {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().ok()?) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let channels = f64::from(u16::from_le_bytes(
                data[pos + 10..pos + 12].try_into().ok()?,
            ));
            let sample_rate = f64::from(u32::from_le_bytes(
                data[pos + 12..pos + 16].try_into().ok()?,
            ));
            let bits_per_sample = f64::from(u16::from_le_bytes(
                data[pos + 22..pos + 24].try_into().ok()?,
            ));

            let mut dpos = pos.checked_add(8)?.checked_add(chunk_size)?;
            while dpos.checked_add(8).is_some_and(|end| end <= data.len()) {
                let did = &data[dpos..dpos + 4];
                let dsize_u32 = u32::from_le_bytes(data[dpos + 4..dpos + 8].try_into().ok()?);
                if did == b"data" {
                    let dsize = f64::from(dsize_u32);
                    let bytes_per_sample = bits_per_sample / 8.0;
                    let total_samples = dsize / (channels * bytes_per_sample);
                    return Some(total_samples / sample_rate);
                }
                dpos = dpos.checked_add(8)?.checked_add(dsize_u32 as usize)?;
            }
        }
        pos = pos.checked_add(8)?.checked_add(chunk_size)?;
    }
    None
}

#[must_use]
pub fn mp3_duration_secs(path: &Path) -> Option<f64> {
    let size = std::fs::metadata(path).ok()?.len();
    let bytes_per_sec: u32 = 128_000 / 8;
    let whole_secs = u32::try_from(size / u64::from(bytes_per_sec)).unwrap_or(u32::MAX);
    let remainder = u32::try_from(size % u64::from(bytes_per_sec)).unwrap_or(0);
    Some(f64::from(whole_secs) + f64::from(remainder) / f64::from(bytes_per_sec))
}

#[must_use]
pub fn audio_duration_secs(path: &Path) -> Option<f64> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())?;
    match ext.as_str() {
        "wav" => wav_duration_secs(path),
        "mp3" => mp3_duration_secs(path),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_wav(path: &Path, sample_rate: u32, channels: u16, bits_per_sample: u16, data: &[u8]) {
        let data_size = u32::try_from(data.len()).unwrap_or(0);
        let file_size = 36 + data_size;
        let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample) / 8;
        let block_align = channels * bits_per_sample / 8;

        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(b"RIFF").unwrap();
        f.write_all(&file_size.to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
        f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        f.write_all(&channels.to_le_bytes()).unwrap();
        f.write_all(&sample_rate.to_le_bytes()).unwrap();
        f.write_all(&byte_rate.to_le_bytes()).unwrap();
        f.write_all(&block_align.to_le_bytes()).unwrap();
        f.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_size.to_le_bytes()).unwrap();
        f.write_all(data).unwrap();
    }

    #[test]
    fn wav_duration_valid() {
        let dir = std::env::temp_dir().join("rcp2_test_wav_valid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");

        let data = vec![0u8; 44100];
        write_wav(&path, 44100, 1, 8, &data);

        let duration = wav_duration_secs(&path).unwrap();
        assert!(
            (duration - 1.0).abs() < 0.01,
            "expected ~1.0s, got {duration}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wav_duration_invalid_file() {
        let path = Path::new("/tmp/rcp2_test_nonexistent_wav_file.wav");
        assert!(wav_duration_secs(path).is_none());
    }

    #[test]
    fn mp3_duration_estimate() {
        let dir = std::env::temp_dir().join("rcp2_test_mp3_dur");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.mp3");

        std::fs::write(&path, vec![0u8; 128_000]).unwrap();

        let duration = mp3_duration_secs(&path).unwrap();
        let expected = f64::from(128_000_u32) / (128_000.0 / 8.0);
        assert!(
            (duration - expected).abs() < 0.001,
            "expected {expected}, got {duration}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
