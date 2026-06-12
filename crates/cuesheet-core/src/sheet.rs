//! Builds the flattened `SheetManifest` (and its thumbnail assets) from a
//! parsed playlist — the offline analog of the vbs describe stage.

use std::collections::BTreeMap;

use crate::model::{Cue, Item, Location, SheetManifest, Ticks};
use crate::parse::OpenedPlaylist;

/// Markers closer together than this are merged into one contiguous range,
/// matching the vbs cut stage.
pub const MARKER_MERGE_GAP_SECS: f64 = 0.5;

/// Shown in the metadata slot where vbs reports the selected rendition;
/// offline output is generated from the playlist's own data.
pub const OFFLINE_RESOLUTION: &str = "original";

pub struct BuiltSheet {
    pub manifest: SheetManifest,
    /// Virtual path -> bytes for every thumbnail referenced by the template.
    pub assets: BTreeMap<String, Vec<u8>>,
}

pub fn build_sheet(opened: &OpenedPlaylist) -> BuiltSheet {
    let playlist = &opened.playlist;
    let mut assets = BTreeMap::new();
    let mut cues = Vec::with_capacity(playlist.items.len());

    let language_id = playlist
        .items
        .iter()
        .filter_map(|i| i.location.as_ref())
        .find_map(|l| l.meps_language)
        .unwrap_or(0);

    for (i, item) in playlist.items.iter().enumerate() {
        let index = i + 1;
        let thumbnail = extract_thumbnail(opened, item, index, &mut assets);
        cues.push(Cue {
            index,
            label: item.label.clone(),
            clip: source_identifier(item),
            thumbnail,
            duration_sec: item_duration_secs(item),
            end_action_raw: item.end_action,
        });
    }

    BuiltSheet {
        manifest: SheetManifest {
            name: playlist.name.clone(),
            language_code: "MEPS".to_string(),
            language_id,
            resolution: OFFLINE_RESOLUTION.to_string(),
            cues,
        },
        assets,
    }
}

/// Effective playable duration of an item, in seconds.
///
/// When markers exist they define the playable ranges: ranges separated by
/// less than `MARKER_MERGE_GAP_SECS` merge into one (the vbs cut stage rule)
/// and the durations of the merged ranges are summed. Without markers, the
/// trim offsets apply to the base duration; the trim columns are positions
/// (offsets from the start of the media), so end trim 0 means "play to the
/// natural end".
pub fn item_duration_secs(item: &Item) -> f64 {
    if !item.markers.is_empty() {
        return merged_marker_ranges(item)
            .iter()
            .map(|(start, end)| end - start)
            .sum();
    }

    let base = item
        .location
        .as_ref()
        .map(|l| l.base_duration_ticks)
        .or_else(|| item.image.as_ref().map(|img| img.duration_ticks))
        .unwrap_or(Ticks(0))
        .as_secs_f64();

    let start = item.start_trim_ticks.as_secs_f64();
    let end = if item.end_trim_ticks.0 > 0 {
        item.end_trim_ticks.as_secs_f64()
    } else {
        base
    };
    (end - start).max(0.0)
}

/// Marker ranges merged with the 0.5s contiguity rule, as (start, end) secs.
pub fn merged_marker_ranges(item: &Item) -> Vec<(f64, f64)> {
    let mut ranges: Vec<(f64, f64)> = item
        .markers
        .iter()
        .map(|m| {
            let start = m.start_time_ticks.as_secs_f64();
            (start, start + m.duration_ticks.as_secs_f64())
        })
        .collect();
    ranges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut merged: Vec<(f64, f64)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some((_, last_end)) if start - *last_end <= MARKER_MERGE_GAP_SECS => {
                if end > *last_end {
                    *last_end = end;
                }
            }
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// The monospace source-identifier slot (vbs shows the cut clip filename).
fn source_identifier(item: &Item) -> String {
    if let Some(img) = &item.image {
        if !img.original_filename.is_empty() {
            return img.original_filename.clone();
        }
        if !img.file_path.is_empty() {
            return img.file_path.clone();
        }
    }
    if let Some(loc) = &item.location {
        return location_identifier(loc);
    }
    String::new()
}

fn location_identifier(loc: &Location) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(sym) = &loc.key_symbol {
        if !sym.is_empty() {
            parts.push(sym.clone());
        }
    }
    match (loc.book_number, loc.chapter_number) {
        (Some(book), Some(chapter)) if book > 0 || chapter > 0 => {
            parts.push(format!("{book}:{chapter}"));
        }
        _ => {}
    }
    if let Some(doc) = loc.document_id {
        if doc > 0 {
            parts.push(format!("doc {doc}"));
        }
    }
    if let Some(track) = loc.track {
        if track > 0 {
            parts.push(format!("track {track}"));
        }
    }
    parts.join(" ")
}

fn extract_thumbnail(
    opened: &OpenedPlaylist,
    item: &Item,
    index: usize,
    assets: &mut BTreeMap<String, Vec<u8>>,
) -> String {
    let source_path = item
        .thumbnail_path
        .as_deref()
        .filter(|p| !p.is_empty())
        .or_else(|| {
            item.image
                .as_ref()
                .map(|img| img.file_path.as_str())
                .filter(|p| !p.is_empty())
        });

    let Some(source_path) = source_path else {
        return String::new();
    };
    let Some(bytes) = opened.media_bytes(source_path) else {
        return String::new();
    };

    let ext = std::path::Path::new(source_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let virtual_path = format!("thumbs/{index:02}.{ext}");
    assets.insert(virtual_path.clone(), bytes.to_vec());
    virtual_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EmbeddedImage, Marker, TICKS_PER_SECOND};

    fn ticks(secs: f64) -> Ticks {
        Ticks((secs * TICKS_PER_SECOND as f64) as i64)
    }

    fn marker(start: f64, dur: f64) -> Marker {
        Marker {
            start_time_ticks: ticks(start),
            duration_ticks: ticks(dur),
            ..Marker::default()
        }
    }

    #[test]
    fn duration_without_markers_uses_trims_as_positions() {
        let item = Item {
            start_trim_ticks: ticks(2.0),
            end_trim_ticks: ticks(10.0),
            location: Some(Location {
                base_duration_ticks: ticks(60.0),
                ..Location::default()
            }),
            ..Item::default()
        };
        assert!((item_duration_secs(&item) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn duration_without_end_trim_plays_to_natural_end() {
        let item = Item {
            start_trim_ticks: ticks(5.0),
            location: Some(Location {
                base_duration_ticks: ticks(60.0),
                ..Location::default()
            }),
            ..Item::default()
        };
        assert!((item_duration_secs(&item) - 55.0).abs() < 1e-9);
    }

    #[test]
    fn image_duration_comes_from_embedded_media() {
        let item = Item {
            image: Some(EmbeddedImage {
                duration_ticks: ticks(8.0),
                ..EmbeddedImage::default()
            }),
            ..Item::default()
        };
        assert!((item_duration_secs(&item) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn contiguous_markers_merge_within_half_second() {
        let item = Item {
            markers: vec![marker(0.0, 10.0), marker(10.3, 5.0), marker(20.0, 2.0)],
            ..Item::default()
        };
        let ranges = merged_marker_ranges(&item);
        assert_eq!(ranges.len(), 2);
        assert!((ranges[0].0 - 0.0).abs() < 1e-9);
        assert!((ranges[0].1 - 15.3).abs() < 1e-9);
        assert!((ranges[1].0 - 20.0).abs() < 1e-9);
        // Total playable: 15.3 + 2.0
        assert!((item_duration_secs(&item) - 17.3).abs() < 1e-9);
    }

    #[test]
    fn overlapping_markers_do_not_double_count() {
        let item = Item {
            markers: vec![marker(0.0, 10.0), marker(5.0, 3.0)],
            ..Item::default()
        };
        assert!((item_duration_secs(&item) - 10.0).abs() < 1e-9);
    }
}
