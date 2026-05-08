use crate::srt::Cue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl Segment {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

pub fn build(
    cues: &[Cue],
    pad_ms: u32,
    gap_ms: u32,
    total_duration_ms: Option<u64>,
) -> Vec<Segment> {
    let pad = pad_ms as u64;
    let max_end = total_duration_ms.unwrap_or(u64::MAX);

    let mut raw: Vec<Segment> = cues
        .iter()
        .map(|c| Segment {
            start_ms: c.start_ms.saturating_sub(pad),
            end_ms: (c.end_ms + pad).min(max_end),
        })
        .filter(|s| s.end_ms > s.start_ms)
        .collect();

    raw.sort_by_key(|s| s.start_ms);
    merge(raw, gap_ms as u64)
}

fn merge(sorted: Vec<Segment>, gap_ms: u64) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::with_capacity(sorted.len());
    for seg in sorted {
        match out.last_mut() {
            Some(last) if seg.start_ms <= last.end_ms.saturating_add(gap_ms) => {
                last.end_ms = last.end_ms.max(seg.end_ms);
            }
            _ => out.push(seg),
        }
    }
    out
}

pub fn total_duration_ms(segments: &[Segment]) -> u64 {
    segments.iter().map(|s| s.duration_ms()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(start: u64, end: u64) -> Cue {
        Cue {
            start_ms: start,
            end_ms: end,
            text: "x".into(),
        }
    }

    #[test]
    fn pads_clamps_to_zero() {
        let segs = build(&[cue(100, 500)], 500, 0, None);
        assert_eq!(
            segs,
            vec![Segment {
                start_ms: 0,
                end_ms: 1000
            }]
        );
    }

    #[test]
    fn pads_clamps_to_total_duration() {
        let segs = build(&[cue(0, 9_500)], 500, 0, Some(10_000));
        assert_eq!(
            segs,
            vec![Segment {
                start_ms: 0,
                end_ms: 10_000
            }]
        );
    }

    #[test]
    fn merges_overlapping_padded_ranges() {
        // cue1: 1000-2000 padded -> 500-2500
        // cue2: 2400-3000 padded -> 1900-3500 -> overlaps
        let segs = build(&[cue(1000, 2000), cue(2400, 3000)], 500, 0, None);
        assert_eq!(
            segs,
            vec![Segment {
                start_ms: 500,
                end_ms: 3500
            }]
        );
    }

    #[test]
    fn merges_with_gap_close() {
        // padded ranges: 500-2500 and 3000-5000 — gap 500ms
        let segs = build(&[cue(1000, 2000), cue(3500, 4500)], 500, 600, None);
        assert_eq!(segs.len(), 1);
        assert_eq!(
            segs[0],
            Segment {
                start_ms: 500,
                end_ms: 5000
            }
        );
    }

    #[test]
    fn does_not_merge_when_gap_exceeds_threshold() {
        let segs = build(&[cue(1000, 2000), cue(5000, 6000)], 500, 200, None);
        assert_eq!(segs.len(), 2);
    }

    #[test]
    fn unsorted_input_handled() {
        let segs = build(&[cue(5000, 6000), cue(1000, 2000)], 100, 0, None);
        assert_eq!(segs.len(), 2);
        assert!(segs[0].start_ms < segs[1].start_ms);
    }
}
