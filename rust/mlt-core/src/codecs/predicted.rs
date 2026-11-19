//! Predicted vertices: residuals against a guess from the decoded vertices, in class streams, as split varints or escaped byte planes.
//!
//! Follows the vertex-buffer pipeline of <https://turtelll.github.io/Thesis/>, without its frames, shared references and reorders.

use usize_cast::IntoUsize as _;

use crate::MltError::MalformedPredictedVertices as Malformed;
use crate::{Decoder, MltResult};

/// A ring or line part, the only runs a predictor guesses in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunKind {
    Line,
    Ring,
}

/// One ring or line part: `len` vertices from `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub start: u32,
    pub len: u32,
    pub kind: RunKind,
}

/// The guess a vertex is written against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Predictor {
    /// The previous vertex.
    Delta,
    /// The fourth corner of a four-vertex ring completes the parallelogram of the first three.
    QuadCorner,
    /// From the third vertex of a line part of six or more on, the last edge is repeated.
    LineContinuation,
}

/// How the residuals are split into streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Segmentation {
    /// One stream in vertex order.
    Flat,
    /// One stream per class: run starts, delta vertices, predicted vertices.
    Class,
}

/// How a stream's zigzagged values become bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    /// One head byte per value, then the continuation bytes of all values.
    SplitVarint,
    /// A descriptor byte, then one byte per value with `0xFF` escaping a wider exception, or a fixed width as byte planes.
    Escaped,
}

/// One choice per stage, stored in the payload's first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Composition {
    pub predictor: Predictor,
    pub segmentation: Segmentation,
    pub layout: Layout,
}

const START: u8 = 0;
const INTERIOR: u8 = 1;
const PREDICTED: u8 = 2;
const CLASSES: usize = 3;
const MIN_CONTINUED_LINE: u32 = 6;
const QUAD: u32 = 4;
const ESCAPE: u8 = 0xFF;
const ESCAPE_FLAG: u8 = 0b100;

impl Composition {
    const ALL_PREDICTORS: [Predictor; 3] = [
        Predictor::Delta,
        Predictor::QuadCorner,
        Predictor::LineContinuation,
    ];

    fn header(self) -> u8 {
        let predictor = match self.predictor {
            Predictor::Delta => 0,
            Predictor::QuadCorner => 1,
            Predictor::LineContinuation => 2,
        };
        let segmentation = match self.segmentation {
            Segmentation::Flat => 0,
            Segmentation::Class => 1,
        };
        let layout = match self.layout {
            Layout::SplitVarint => 0,
            Layout::Escaped => 1,
        };
        predictor | (segmentation << 4) | (layout << 5)
    }

    fn from_header(byte: u8) -> MltResult<Self> {
        let predictor = match byte & 0x0F {
            0 => Predictor::Delta,
            1 => Predictor::QuadCorner,
            2 => Predictor::LineContinuation,
            _ => return Err(Malformed("unknown predictor")),
        };
        let segmentation = if byte & 0x10 == 0 {
            Segmentation::Flat
        } else {
            Segmentation::Class
        };
        let layout = match (byte >> 5) & 0b11 {
            0 => Layout::SplitVarint,
            1 => Layout::Escaped,
            _ => return Err(Malformed("unknown byte layout")),
        };
        if byte & 0x80 != 0 {
            return Err(Malformed("reserved header bit set"));
        }
        Ok(Self {
            predictor,
            segmentation,
            layout,
        })
    }
}

/// The composition a payload was written with.
pub fn composition(data: &[u8]) -> MltResult<Composition> {
    let (&header, _) = data.split_first().ok_or(Malformed("no header byte"))?;
    Composition::from_header(header)
}

/// The cheapest payload for interleaved `(x, y)` `vertices` under `cost`.
pub fn encode(vertices: &[i32], runs: &[Run], cost: &dyn Fn(&[u8]) -> usize) -> MltResult<Vec<u8>> {
    let vertex_count = vertex_count(vertices)?;
    let quads = runs
        .iter()
        .any(|r| r.kind == RunKind::Ring && r.len == QUAD);
    let lines = runs
        .iter()
        .any(|r| r.kind == RunKind::Line && r.len >= MIN_CONTINUED_LINE);
    let mut best: Option<(usize, Vec<u8>)> = None;
    for predictor in Composition::ALL_PREDICTORS {
        let useful = match predictor {
            Predictor::Delta => true,
            Predictor::QuadCorner => quads,
            Predictor::LineContinuation => lines,
        };
        if !useful {
            continue;
        }
        let classes = classify(runs, vertex_count, predictor)?;
        let residuals = residuals(vertices, &classes, predictor);
        let segmentations: &[Segmentation] = if predictor == Predictor::Delta {
            &[Segmentation::Flat]
        } else {
            &[Segmentation::Flat, Segmentation::Class]
        };
        for &segmentation in segmentations {
            for layout in [Layout::SplitVarint, Layout::Escaped] {
                let composition = Composition {
                    predictor,
                    segmentation,
                    layout,
                };
                let payload = write(&residuals, &classes, composition);
                let size = cost(&payload);
                if best.as_ref().is_none_or(|(s, _)| size < *s) {
                    best = Some((size, payload));
                }
            }
        }
    }
    Ok(best.map_or_else(Vec::new, |(_, p)| p))
}

/// The payload for interleaved `(x, y)` `vertices` with one fixed `composition`.
#[cfg(test)]
fn encode_as(vertices: &[i32], runs: &[Run], composition: Composition) -> MltResult<Vec<u8>> {
    let vertex_count = vertex_count(vertices)?;
    let classes = classify(runs, vertex_count, composition.predictor)?;
    let residuals = residuals(vertices, &classes, composition.predictor);
    Ok(write(&residuals, &classes, composition))
}

/// Decode `vertex_count` interleaved `(x, y)` vertices over `runs`.
pub fn decode(
    data: &[u8],
    runs: &[Run],
    vertex_count: u32,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let (&header, mut rest) = data.split_first().ok_or(Malformed("no header byte"))?;
    let composition = Composition::from_header(header)?;
    let n = vertex_count.into_usize();
    dec.consume_items::<u8>(n)?;
    let classes = classify(runs, n, composition.predictor)?;
    let mut counts = [0_usize; CLASSES];
    for &c in &classes {
        counts[usize::from(c)] += 1;
    }
    let lengths = match composition.segmentation {
        Segmentation::Flat => [n, 0, 0],
        Segmentation::Class => counts,
    };
    let mut values = dec.alloc::<u32>(2 * n)?;
    let mut stream_starts = [0_usize; CLASSES];
    for (class, &len) in lengths.iter().enumerate() {
        stream_starts[class] = values.len();
        if len > 0 {
            rest = match composition.layout {
                Layout::SplitVarint => read_split_varint(rest, 2 * len, &mut values)?,
                Layout::Escaped => read_escaped(rest, 2 * len, &mut values)?,
            };
        }
    }
    if !rest.is_empty() {
        return Err(Malformed("bytes after the last stream"));
    }

    let mut out = dec.alloc::<i32>(2 * n)?;
    let mut cursor = stream_starts;
    let (mut px, mut py) = (0_i32, 0_i32);
    for (i, &class) in classes.iter().enumerate() {
        let stream = match composition.segmentation {
            Segmentation::Flat => 0,
            Segmentation::Class => usize::from(class),
        };
        let at = cursor[stream];
        cursor[stream] += 2;
        let (gx, gy) = guess(&out, i, class, composition.predictor, (px, py));
        let x = gx.wrapping_add(unzigzag(values[at]));
        let y = gy.wrapping_add(unzigzag(values[at + 1]));
        out.push(x);
        out.push(y);
        (px, py) = (x, y);
    }
    Ok(out)
}

fn vertex_count(vertices: &[i32]) -> MltResult<usize> {
    if vertices.len().is_multiple_of(2) {
        Ok(vertices.len() / 2)
    } else {
        Err(Malformed("odd vertex word count"))
    }
}

/// The class of every vertex, which only the topology decides.
fn classify(runs: &[Run], vertex_count: usize, predictor: Predictor) -> MltResult<Vec<u8>> {
    let mut classes = vec![INTERIOR; vertex_count];
    if predictor == Predictor::Delta {
        return Ok(classes);
    }
    let mut end = 0_usize;
    for run in runs {
        let start = run.start.into_usize();
        let len = run.len.into_usize();
        if start < end {
            return Err(Malformed("runs overlap or are out of order"));
        }
        if len == 0 {
            continue;
        }
        end = start
            .checked_add(len)
            .filter(|&e| e <= vertex_count)
            .ok_or(Malformed("a run ends past the last vertex"))?;
        let class = &mut classes[start..end];
        class[0] = START;
        match (predictor, run.kind) {
            (Predictor::QuadCorner, RunKind::Ring) if run.len == QUAD => class[3] = PREDICTED,
            (Predictor::LineContinuation, RunKind::Line) if run.len >= MIN_CONTINUED_LINE => {
                class[2..].fill(PREDICTED);
            }
            _ => {}
        }
    }
    Ok(classes)
}

fn guess(
    vertices: &[i32],
    i: usize,
    class: u8,
    predictor: Predictor,
    previous: (i32, i32),
) -> (i32, i32) {
    if class != PREDICTED {
        return previous;
    }
    let at = |back: usize, axis: usize| vertices[2 * (i - back) + axis];
    let component = |axis| match predictor {
        Predictor::QuadCorner => at(3, axis)
            .wrapping_add(at(1, axis))
            .wrapping_sub(at(2, axis)),
        Predictor::LineContinuation => at(1, axis).wrapping_mul(2).wrapping_sub(at(2, axis)),
        Predictor::Delta => unreachable!("a delta vertex is never predicted"),
    };
    (component(0), component(1))
}

/// Zigzagged `(x, y)` residuals, interleaved in vertex order.
fn residuals(vertices: &[i32], classes: &[u8], predictor: Predictor) -> Vec<u32> {
    let mut out = Vec::with_capacity(vertices.len());
    let mut previous = (0_i32, 0_i32);
    for (i, (&class, v)) in classes.iter().zip(vertices.as_chunks::<2>().0).enumerate() {
        let (gx, gy) = guess(vertices, i, class, predictor, previous);
        out.push(zigzag(v[0].wrapping_sub(gx)));
        out.push(zigzag(v[1].wrapping_sub(gy)));
        previous = (v[0], v[1]);
    }
    out
}

fn write(residuals: &[u32], classes: &[u8], composition: Composition) -> Vec<u8> {
    let mut out = Vec::with_capacity(residuals.len() + 8);
    out.push(composition.header());
    let mut streams: [Vec<u32>; CLASSES] = Default::default();
    match composition.segmentation {
        Segmentation::Flat => streams[0].extend_from_slice(residuals),
        Segmentation::Class => {
            for (&class, pair) in classes.iter().zip(residuals.as_chunks::<2>().0) {
                streams[usize::from(class)].extend_from_slice(pair);
            }
        }
    }
    for stream in streams.iter().filter(|s| !s.is_empty()) {
        match composition.layout {
            Layout::SplitVarint => write_split_varint(stream, &mut out),
            Layout::Escaped => write_escaped(stream, &mut out),
        }
    }
    out
}

fn zigzag(v: i32) -> u32 {
    (v.cast_unsigned() << 1) ^ (v >> 31).cast_unsigned()
}

fn unzigzag(v: u32) -> i32 {
    (v >> 1).cast_signed() ^ -((v & 1).cast_signed())
}

fn write_split_varint(values: &[u32], out: &mut Vec<u8>) {
    out.extend(values.iter().map(|&v| {
        let low = (v & 0x7F) as u8;
        if v > 0x7F { low | 0x80 } else { low }
    }));
    for &v in values.iter().filter(|&&v| v > 0x7F) {
        let mut rest = v >> 7;
        loop {
            let low = (rest & 0x7F) as u8;
            rest >>= 7;
            if rest == 0 {
                out.push(low);
                break;
            }
            out.push(low | 0x80);
        }
    }
}

fn read_split_varint<'a>(data: &'a [u8], n: usize, out: &mut Vec<u32>) -> MltResult<&'a [u8]> {
    let (heads, mut tail) = data
        .split_at_checked(n)
        .ok_or(Malformed("split varint heads are cut short"))?;
    for &head in heads {
        let mut value = u32::from(head & 0x7F);
        if head & 0x80 != 0 {
            let mut shift = 7;
            loop {
                let (&byte, rest) = tail
                    .split_first()
                    .ok_or(Malformed("split varint tail is cut short"))?;
                tail = rest;
                let bits = u32::from(byte & 0x7F);
                if shift > 28 || (shift == 28 && bits > 0x0F) {
                    return Err(Malformed("split varint exceeds 32 bits"));
                }
                value |= bits << shift;
                if byte & 0x80 == 0 {
                    if byte == 0 {
                        return Err(Malformed("split varint is not minimal"));
                    }
                    break;
                }
                shift += 7;
            }
        }
        out.push(value);
    }
    Ok(tail)
}

/// The byte width code of `max`: `0` for one byte, `1` for two, `2` for four.
fn width_code(max: u32) -> u8 {
    match max {
        0..=0xFF => 0,
        0x100..=0xFFFF => 1,
        _ => 2,
    }
}

fn width_of(code: u8) -> usize {
    1 << code
}

fn write_planes(values: &[u32], width: usize, out: &mut Vec<u8>) {
    for byte in 0..width {
        #[expect(clippy::cast_possible_truncation, reason = "one byte plane")]
        out.extend(values.iter().map(|v| (v >> (8 * byte)) as u8));
    }
}

fn write_escaped(values: &[u32], out: &mut Vec<u8>) {
    let n = values.len();
    let max = values.iter().copied().max().unwrap_or(0);
    let code = width_code(max);
    if code > 0 {
        let exceptions = values.iter().filter(|&&v| v >= u32::from(ESCAPE)).count();
        let exception_code = width_code(max - u32::from(ESCAPE));
        if n + exceptions * width_of(exception_code) < n * width_of(code) {
            out.push(ESCAPE_FLAG | (exception_code << 3));
            #[expect(clippy::cast_possible_truncation, reason = "below the escape marker")]
            out.extend(values.iter().map(|&v| v.min(u32::from(ESCAPE)) as u8));
            let wide: Vec<u32> = values
                .iter()
                .filter_map(|&v| v.checked_sub(u32::from(ESCAPE)))
                .collect();
            write_planes(&wide, width_of(exception_code), out);
            return;
        }
    }
    out.push(code);
    write_planes(values, width_of(code), out);
}

fn read_planes(data: &[u8], n: usize, width: usize, out: &mut Vec<u32>) -> MltResult<usize> {
    let len = n
        .checked_mul(width)
        .filter(|&len| len <= data.len())
        .ok_or(Malformed("byte planes are cut short"))?;
    let start = out.len();
    out.extend(data[..n].iter().map(|&b| u32::from(b)));
    for byte in 1..width {
        let plane = &data[byte * n..(byte + 1) * n];
        for (value, &b) in out[start..].iter_mut().zip(plane) {
            *value |= u32::from(b) << (8 * byte);
        }
    }
    Ok(len)
}

fn read_escaped<'a>(data: &'a [u8], n: usize, out: &mut Vec<u32>) -> MltResult<&'a [u8]> {
    let (&descriptor, body) = data
        .split_first()
        .ok_or(Malformed("escaped stream has no descriptor"))?;
    let code = descriptor & 0b11;
    let exception_code = (descriptor >> 3) & 0b11;
    if code == 3 || exception_code == 3 || descriptor & 0xE0 != 0 {
        return Err(Malformed("unknown escaped stream descriptor"));
    }
    if descriptor & ESCAPE_FLAG == 0 {
        if exception_code != 0 {
            return Err(Malformed("exception width without the escape flag"));
        }
        let used = read_planes(body, n, width_of(code), out)?;
        return Ok(&body[used..]);
    }
    if code != 0 {
        return Err(Malformed("escaped values wider than one byte"));
    }
    let main = body
        .get(..n)
        .ok_or(Malformed("escaped stream is cut short"))?;
    #[expect(
        clippy::naive_bytecount,
        reason = "one count per stream needs no bytecount dependency"
    )]
    let escapes = main.iter().filter(|&&b| b == ESCAPE).count();
    let mut exceptions = Vec::with_capacity(escapes);
    let used = read_planes(
        &body[n..],
        escapes,
        width_of(exception_code),
        &mut exceptions,
    )?;
    let mut wide = exceptions.into_iter();
    for &b in main {
        out.push(if b == ESCAPE {
            wide.next()
                .and_then(|e| e.checked_add(u32::from(ESCAPE)))
                .ok_or(Malformed("escaped exception exceeds 32 bits"))?
        } else {
            u32::from(b)
        });
    }
    Ok(&body[n + used..])
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::__private::dec;

    const RECTANGLE: [i32; 8] = [100, 200, 130, 200, 130, 240, 100, 240];

    fn ring(start: u32, len: u32) -> Run {
        Run {
            start,
            len,
            kind: RunKind::Ring,
        }
    }

    fn line(start: u32, len: u32) -> Run {
        Run {
            start,
            len,
            kind: RunKind::Line,
        }
    }

    fn compositions() -> Vec<Composition> {
        let mut all = Vec::new();
        for predictor in Composition::ALL_PREDICTORS {
            for segmentation in [Segmentation::Flat, Segmentation::Class] {
                for layout in [Layout::SplitVarint, Layout::Escaped] {
                    all.push(Composition {
                        predictor,
                        segmentation,
                        layout,
                    });
                }
            }
        }
        all
    }

    fn round_trip(vertices: &[i32], runs: &[Run], composition: Composition) -> Vec<i32> {
        let payload = encode_as(vertices, runs, composition).unwrap();
        assert_eq!(super::composition(&payload).unwrap(), composition);
        let count = u32::try_from(vertices.len() / 2).unwrap();
        decode(&payload, runs, count, &mut dec()).unwrap()
    }

    #[test]
    fn a_rectangle_corner_costs_two_zero_bytes() {
        let composition = Composition {
            predictor: Predictor::QuadCorner,
            segmentation: Segmentation::Class,
            layout: Layout::Escaped,
        };
        let payload = encode_as(&RECTANGLE, &[ring(0, 4)], composition).unwrap();
        assert_eq!(
            payload,
            [
                0x31, 0x04, 0xC8, 0xFF, 0x91, 0x00, 0x3C, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00
            ]
        );
    }

    #[test]
    fn split_varint_moves_continuation_bytes_behind_the_heads() {
        let composition = Composition {
            predictor: Predictor::Delta,
            segmentation: Segmentation::Flat,
            layout: Layout::SplitVarint,
        };
        let payload = encode_as(&[0, 150, 2, 150], &[], composition).unwrap();
        assert_eq!(payload, [0x00, 0x00, 0xAC, 0x04, 0x00, 0x02]);
    }

    #[test]
    fn line_continuation_predicts_from_the_third_vertex_of_a_long_part() {
        let vertices = [0, 0, 10, 2, 21, 4, 31, 7, 40, 10, 50, 12];
        let composition = Composition {
            predictor: Predictor::LineContinuation,
            segmentation: Segmentation::Class,
            layout: Layout::SplitVarint,
        };
        let payload = encode_as(&vertices, &[line(0, 6)], composition).unwrap();
        assert_eq!(
            payload,
            [
                0x12, 0x00, 0x00, 0x14, 0x04, 0x02, 0x00, 0x01, 0x02, 0x01, 0x00, 0x02, 0x01
            ]
        );
    }

    #[rstest]
    fn every_composition_round_trips_mixed_runs(
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] index: usize,
    ) {
        let composition = compositions()[index];
        let mut vertices = vec![5, 5];
        vertices.extend(RECTANGLE);
        vertices.extend([0, 0, 10, 2, 21, 4, 31, 7, 40, 10, 50, 12, 70, 20]);
        vertices.extend([i32::MIN, i32::MAX, i32::MAX, i32::MIN, 0, 0, -1, 1]);
        vertices.extend([300, -70_000, 1 << 30, -(1 << 30)]);
        let runs = [ring(1, 4), line(5, 7), ring(12, 4), line(16, 2)];
        assert_eq!(round_trip(&vertices, &runs, composition), vertices);
    }

    #[test]
    fn the_cheapest_composition_is_kept() {
        let square = [0, 0, 1000, 0, 1000, 1000, 0, 1000];
        let payload = encode(&square, &[ring(0, 4)], &|p: &[u8]| p.len()).unwrap();
        assert_eq!(
            composition(&payload).unwrap(),
            Composition {
                predictor: Predictor::QuadCorner,
                segmentation: Segmentation::Flat,
                layout: Layout::SplitVarint,
            }
        );
        assert_eq!(payload.len(), 11);
    }

    #[rstest]
    #[case::no_header(&[], "no header byte")]
    #[case::unknown_predictor(&[0x03, 0x00, 0x00], "unknown predictor")]
    #[case::unknown_layout(&[0x40, 0x00, 0x00], "unknown byte layout")]
    #[case::reserved_bit(&[0x80, 0x00, 0x00], "reserved header bit set")]
    #[case::trailing_bytes(&[0x00, 0x00, 0x00, 0x00], "bytes after the last stream")]
    #[case::cut_heads(&[0x00, 0x00], "split varint heads are cut short")]
    #[case::cut_tail(&[0x00, 0x80, 0x00], "split varint tail is cut short")]
    #[case::not_minimal(&[0x00, 0x80, 0x00, 0x00], "split varint is not minimal")]
    #[case::too_wide(&[0x00, 0x80, 0x00, 0xFF, 0xFF, 0xFF, 0x7F], "split varint exceeds 32 bits")]
    #[case::unknown_descriptor(&[0x20, 0x03, 0x00, 0x00], "unknown escaped stream descriptor")]
    #[case::stray_exception_width(&[0x20, 0x08, 0x00, 0x00], "exception width without the escape flag")]
    #[case::wide_escaped_values(&[0x20, 0x05, 0x00, 0x00], "escaped values wider than one byte")]
    #[case::cut_planes(&[0x20, 0x01, 0x00, 0x00], "byte planes are cut short")]
    #[case::cut_exceptions(&[0x20, 0x0C, 0xFF, 0x00, 0x01], "byte planes are cut short")]
    #[case::exception_overflow(&[0x20, 0x14, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF], "escaped exception exceeds 32 bits")]
    fn malformed_payloads_are_rejected(#[case] data: &[u8], #[case] reason: &str) {
        let err = decode(data, &[], 1, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[rstest]
    #[case::overlapping(&[ring(0, 4), line(2, 6)], "runs overlap or are out of order")]
    #[case::past_the_end(&[line(0, 9)], "a run ends past the last vertex")]
    fn bad_runs_are_rejected(#[case] runs: &[Run], #[case] reason: &str) {
        let err = decode(&[0x01], runs, 8, &mut dec()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("predicted vertex stream is malformed: {reason}")
        );
    }

    #[test]
    fn empty_runs_are_skipped() {
        let composition = Composition {
            predictor: Predictor::QuadCorner,
            segmentation: Segmentation::Class,
            layout: Layout::SplitVarint,
        };
        let runs = [line(0, 0), ring(0, 4), ring(4, 0)];
        assert_eq!(round_trip(&RECTANGLE, &runs, composition), RECTANGLE);
    }

    fn arb_layer() -> impl Strategy<Value = (Vec<i32>, Vec<Run>)> {
        prop::collection::vec((0_u32..12, any::<bool>(), 0_u32..3), 0..12).prop_flat_map(|shapes| {
            let mut runs = Vec::new();
            let mut start = 0;
            for (len, ring, gap) in shapes {
                start += gap;
                let kind = if ring { RunKind::Ring } else { RunKind::Line };
                runs.push(Run { start, len, kind });
                start += len;
            }
            let words = 2 * start.into_usize();
            (prop::collection::vec(any::<i32>(), words), Just(runs))
        })
    }

    proptest! {
        #[test]
        fn any_layer_round_trips_every_composition((vertices, runs) in arb_layer()) {
            for composition in compositions() {
                prop_assert_eq!(round_trip(&vertices, &runs, composition), vertices.clone());
            }
        }

        #[test]
        fn any_layer_round_trips_the_cheapest_composition((vertices, runs) in arb_layer()) {
            let payload = encode(&vertices, &runs, &|p: &[u8]| p.len()).unwrap();
            let count = u32::try_from(vertices.len() / 2).unwrap();
            prop_assert_eq!(decode(&payload, &runs, count, &mut dec()).unwrap(), vertices);
        }
    }
}
