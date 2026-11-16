//! The two bits of arithmetic a page placed over a gpui window needs, kept apart from AppKit so they
//! are read without a window: where gpui's top-left bounds land in an NSView, and which data store a
//! manifest's sign-ins live in.

/// A box in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Frame {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Frame { x, y, width, height }
    }
}

/// Where a box gpui measured from the window's top-left goes in its content view. An NSView counts
/// from the bottom-left unless it is flipped, so y is turned over against the view's height.
pub fn frame_in(bounds: Frame, parent_height: f64, flipped: bool) -> Frame {
    if flipped {
        return bounds;
    }
    Frame { y: parent_height - bounds.y - bounds.height, ..bounds }
}

/// The identifier of the persistent store a manifest's pages keep their sign-ins in. **One store per
/// manifest, as Classic keys its partition** (`partitionFor`: `persist:armada-capture-<manifest>`, and
/// `unnamed` where Fleet gave none), so two repositories share nothing. A `WKWebsiteDataStore` is named
/// by a UUID, so the same name always gives the same one: a hash of it, set as a version 8 UUID.
pub fn store_id(manifest: Option<&str>) -> [u8; 16] {
    let name = format!("persist:armada-capture-{}", manifest.unwrap_or("unnamed"));
    let hash = |seed: u64| {
        name.bytes().fold(seed, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
    };
    let (high, low) = (hash(0xcbf2_9ce4_8422_2325), hash(0x8422_2325_cbf2_9ce4));
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&high.to_be_bytes());
    id[8..].copy_from_slice(&low.to_be_bytes());
    id[6] = (id[6] & 0x0f) | 0x80;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_from_the_top_is_turned_over_in_a_view_counted_from_the_bottom() {
        let at = frame_in(Frame::new(10., 40., 300., 200.), 800., false);
        assert_eq!(at, Frame::new(10., 560., 300., 200.));
        // The page's top edge is 40 below the window's top: its bottom edge is 560 above the window's bottom.
        assert_eq!(at.y + at.height, 800. - 40.);
    }

    #[test]
    fn a_flipped_view_takes_the_box_as_it_is() {
        let bounds = Frame::new(10., 40., 300., 200.);
        assert_eq!(frame_in(bounds, 800., true), bounds);
    }

    #[test]
    fn a_box_filling_the_window_stays_filling_it() {
        assert_eq!(frame_in(Frame::new(0., 0., 1100., 820.), 820., false), Frame::new(0., 0., 1100., 820.));
    }

    #[test]
    fn a_manifest_always_gets_the_same_store_and_two_manifests_get_two() {
        assert_eq!(store_id(Some("armada")), store_id(Some("armada")));
        assert_ne!(store_id(Some("armada")), store_id(Some("sample-web")));
        assert_ne!(store_id(Some("armada")), store_id(None));
        assert_eq!(store_id(None), store_id(None));
    }

    #[test]
    fn a_store_id_is_a_well_formed_uuid_and_never_the_nil_one() {
        let id = store_id(Some("armada"));
        assert_eq!(id[6] >> 4, 8);
        assert_eq!(id[8] >> 6, 0b10);
        assert_ne!(id, [0; 16]);
    }
}
