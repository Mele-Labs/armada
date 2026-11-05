//! `get_shown_file` for a video: the shape the Gateway reads to mint an address, as the phone's mirror spells it.

use serde_json::json;

use crate::ShownVideo;

#[test]
fn a_shown_video_round_trips_as_path_type_and_size() {
    let wire = json!({"path": "/Users/u/demo.mp4", "media_type": "video/mp4", "bytes": 268435456_u64});
    let read: ShownVideo = crate::decode("a shown video", wire.to_string().as_bytes()).unwrap();
    assert_eq!(read, ShownVideo { path: "/Users/u/demo.mp4".into(), media_type: "video/mp4".into(), bytes: 268435456 });
    assert_eq!(serde_json::to_value(&read).unwrap(), wire);
}
