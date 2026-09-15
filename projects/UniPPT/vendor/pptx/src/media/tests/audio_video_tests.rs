use crate::media::{Audio, Video};

#[test]
fn test_video_from_bytes() {
    let vid = Video::from_bytes(vec![0, 0, 0], "video/mp4");
    assert_eq!(vid.ext(), "mp4");
    assert_eq!(vid.content_type(), "video/mp4");
}

// --- Audio tests ---

#[test]
fn test_audio_from_bytes_mp3() {
    let audio = Audio::from_bytes(vec![0xFF, 0xFB, 0x90], "audio/mpeg");
    assert_eq!(audio.ext(), "mp3");
    assert_eq!(audio.content_type(), "audio/mpeg");
    assert_eq!(audio.blob(), &[0xFF, 0xFB, 0x90]);
    assert_eq!(audio.sha1().len(), 40);
}

#[test]
fn test_audio_from_bytes_wav() {
    let audio = Audio::from_bytes(vec![0, 0, 0], "audio/wav");
    assert_eq!(audio.ext(), "wav");
    assert_eq!(audio.content_type(), "audio/wav");
}

#[test]
fn test_audio_from_bytes_m4a() {
    let audio = Audio::from_bytes(vec![0, 0, 0], "audio/mp4");
    assert_eq!(audio.ext(), "m4a");
    assert_eq!(audio.content_type(), "audio/mp4");
}

#[test]
fn test_audio_sha1_consistency() {
    let data = vec![1, 2, 3, 4, 5];
    let a1 = Audio::from_bytes(data.clone(), "audio/mpeg");
    let a2 = Audio::from_bytes(data, "audio/mpeg");
    assert_eq!(a1.sha1(), a2.sha1());
}
