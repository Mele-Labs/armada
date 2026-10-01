//! A Sketch's drawing on the wire: read as kept, written as staged, and
//! refused at decode where it is no drawing. Since 20.0.

use crate::{decode, encode, AddStudioNode, EditStudioSketch, StudioNodeAdded, StudioNodeContent};

const DRAWN: &str = r#"{"boxes":[{"id":"b1","x":0,"y":0,"body":"The rail"},{"id":"b2","x":300,"y":40,"body":""}],"joins":[{"id":"b1-p1","from":"b1","to":"p1"}],"strokes":[{"id":"s1","points":[{"x":1,"y":2},{"x":-3,"y":4}]}],"pictures":[{"id":"p1","x":10,"y":200,"width":320,"height":200,"staged":{"staged_path":"/tmp/a.png","width":1280,"height":800}}]}"#;

const KEPT: &str = r#"{"kind":"sketch","drawing":{"boxes":[{"id":"b1","x":0,"y":0,"body":"The rail"}],"joins":[{"id":"b1-p1","from":"b1","to":"p1"}],"strokes":[{"id":"s1","points":[{"x":1,"y":2},{"x":-3,"y":4}]}],"pictures":[{"id":"p1","x":10,"y":200,"width":320,"height":200,"frame":{"filename":"01S-01F.png","byte_size":2048,"width":1280,"height":800}}]}}"#;

/// **A drawing reads back whole**, through the domain and onto the wire again:
/// every box, join, stroke and picture, the picture naming its kept frame.
#[test]
fn a_drawing_round_trips_through_the_wire_and_the_domain() {
    let read = decode::<StudioNodeContent>("content", KEPT.as_bytes()).expect("a Sketch");
    let domain = read.to_domain();
    let StudioNodeContent::Sketch { drawing } = StudioNodeContent::from(&domain) else {
        panic!("still a Sketch");
    };
    assert_eq!(drawing.pictures()[0].frame.filename, "01S-01F.png");
    assert_eq!(
        encode(&StudioNodeContent::from(&domain)).expect("plain data"),
        KEPT,
        "and writes back as it was read"
    );
}

/// **Written, each picture is its staged file**, and the drawing beside the
/// position decodes as a Sketch apart from every other kind.
#[test]
fn a_sketch_is_added_as_its_drawing_with_its_pictures_staged() {
    let body = format!(r#"{{"kind":"sketch","drawing":{DRAWN},"position":{{"x":3,"y":4}}}}"#);
    let added = decode::<AddStudioNode>("a node", body.as_bytes()).expect("a Sketch");
    let StudioNodeAdded::Sketch { drawing } = &added.content else {
        panic!("decoded apart, as a Picture is");
    };
    let domain = drawing.to_domain();
    assert_eq!(domain.boxes().len(), 2);
    assert_eq!(
        domain.pictures()[0]
            .frame
            .as_ref()
            .map(|one| one.staged_path.as_str()),
        Some("/tmp/a.png")
    );
    assert_eq!(
        encode(&added).expect("plain data"),
        body,
        "and writes back as sent"
    );

    let edit = format!(r#"{{"node_id":"01S","drawing":{DRAWN}}}"#);
    decode::<EditStudioSketch>("an edit", edit.as_bytes()).expect("a redraw");
}

/// **No write names a kept frame, and none is a malformed drawing.** A picture
/// carrying `frame` is refused, as a Picture's is; so is a join to nothing, an
/// id on two parts, a one-point stroke and a picture at no size — each by the
/// domain's own rule, before Fleet sees the request.
#[test]
fn a_written_picture_naming_a_frame_or_a_malformed_drawing_does_not_decode() {
    let at = r#""position":{"x":0,"y":0}"#;
    let wrapped = |drawing: &str| format!(r#"{{"kind":"sketch","drawing":{drawing},{at}}}"#);
    let empty = r#""joins":[],"strokes":[],"pictures":[]"#;
    for drawing in [
        r#"{"boxes":[],"joins":[],"strokes":[],"pictures":[{"id":"p1","x":0,"y":0,"width":1,"height":1,"frame":{"filename":"../../x.png","byte_size":1,"width":1,"height":1}}]}"#.to_string(),
        r#"{"boxes":[{"id":"b1","x":0,"y":0,"body":""}],"joins":[{"id":"j","from":"b1","to":"b9"}],"strokes":[],"pictures":[]}"#.to_string(),
        format!(r#"{{"boxes":[{{"id":"b1","x":0,"y":0,"body":""}},{{"id":"b1","x":9,"y":9,"body":""}}],{empty}}}"#),
        r#"{"boxes":[],"joins":[],"strokes":[{"id":"s1","points":[{"x":0,"y":0}]}],"pictures":[]}"#.to_string(),
        r#"{"boxes":[],"joins":[],"strokes":[],"pictures":[{"id":"p1","x":0,"y":0,"width":0,"height":1}]}"#.to_string(),
        r#"{"boxes":[{"id":"b1","x":0.5,"y":0,"body":""}],"joins":[],"strokes":[],"pictures":[]}"#.to_string(),
        r#"{"boxes":[]}"#.to_string(),
    ] {
        let body = wrapped(&drawing);
        decode::<AddStudioNode>("a node", body.as_bytes()).expect_err(&body);
    }
    let body = r#"{"kind":"sketch","body":"the diagram in words","position":{"x":0,"y":0}}"#;
    decode::<AddStudioNode>("a node", body.as_bytes()).expect_err("text is no drawing");

    let read = decode::<StudioNodeContent>("content", KEPT.as_bytes()).expect("a Sketch");
    StudioNodeAdded::try_from(read).expect_err("a Sketch as read names kept frames");
}
