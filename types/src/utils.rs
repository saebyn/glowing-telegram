use std::convert::From;

use crate::{CutList, CutListClass};

impl From<CutListClass> for CutList {
    fn from(cut_list: CutListClass) -> Self {
        CutList {
            input_media: cut_list.input_media,
            output_track: cut_list.output_track,
            overlay_tracks: cut_list.overlay_tracks,
            version: cut_list.version,
        }
    }
}
