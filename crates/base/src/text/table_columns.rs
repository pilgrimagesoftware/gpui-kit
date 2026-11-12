//! Column sizing for a Markdown table's default (wrapping) layout.
//!
//! The layout follows CSS auto table layout. Each column has a *min-content*
//! width (its widest piece of text that cannot wrap) and a *max-content*
//! width (its widest cell line unwrapped). A column gets its min-content
//! width first, then a share of the remaining room in proportion to how much
//! wider it would like to be. So a column of short words is never squeezed
//! while a neighbour has slack, and a word is broken only when even the
//! min-content widths do not fit.
//!
//! Every row of a table is its own flex row, so the widths cannot come from
//! the rows' content. They come from [`column_flexes`], whose flex settings
//! depend only on the measured columns, so every row resolves to the same
//! widths and the columns line up.

use std::ops::Range;

/// One column's measured content, as border-box widths in pixels (the cell's
/// padding and border included).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColumnContent {
    /// The widest piece of text that cannot wrap.
    pub(crate) min: f32,
    /// The widest cell line, unwrapped.
    pub(crate) max: f32,
}

/// How one column's cells take part in their row's flex layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColumnFlex {
    /// `flex-basis`: the column's min-content width.
    pub(crate) basis: f32,
    /// `flex-grow`: how much wider the column wants to be.
    pub(crate) grow: f32,
    /// `min-width`: how far the column may shrink once the min-content widths
    /// no longer fit, which is when words start to break.
    pub(crate) min_width: f32,
}

/// The flex settings for every column of a table.
///
/// Room beyond the min-content widths goes to each column in proportion to
/// `max - min`, which is CSS auto layout between its min- and max-content
/// totals. When no column has slack (every cell is one word, say), the room
/// is shared in proportion to `max` instead, so the table still fills its
/// frame. Below the min-content total the columns shrink in proportion to
/// their basis, down to `floor` or their min-content width, whichever is
/// smaller.
pub(crate) fn column_flexes(columns: &[ColumnContent], floor: f32) -> Vec<ColumnFlex> {
    let slack = |column: &ColumnContent| (column.max - column.min).max(0.);
    let any_slack = columns.iter().any(|column| slack(column) > 0.);
    columns
        .iter()
        .map(|column| ColumnFlex {
            basis: column.min,
            grow: if any_slack { slack(column) } else { column.max },
            min_width: floor.min(column.min),
        })
        .collect()
}

/// The byte ranges of `line` that cannot wrap.
///
/// These follow gpui's `LineWrapper`, which is what wraps the cell's text when
/// it is drawn: a min-content width measured to any other rule would let the
/// renderer break a word the measurement thought could wrap elsewhere. The
/// wrapper breaks before a word that follows a space, and before any character
/// that is not part of a word ([`is_word_char`]). A hyphen is part of a word, so
/// "Short-term" is one piece; `/` and `?` are not, so a path or URL wraps at
/// them. Spaces separate pieces and belong to none.
pub(crate) fn unbreakable_segments(line: &str) -> Vec<Range<usize>> {
    let mut segments = Vec::new();
    let mut start: Option<usize> = None;
    let mut previous = '\0';
    for (ix, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if let Some(segment_start) = start.take() {
                segments.push(segment_start..ix);
            }
        } else {
            let breaks_before = !is_word_char(ch) || previous == ' ';
            if breaks_before && let Some(segment_start) = start.take() {
                segments.push(segment_start..ix);
            }
            start.get_or_insert(ix);
        }
        previous = ch;
    }
    if let Some(segment_start) = start {
        segments.push(segment_start..line.len());
    }
    segments
}

/// Whether gpui's line wrapper keeps `c` inside a word.
///
/// A copy of `LineWrapper::is_word_char` (gpui-pre 0.3.7,
/// `text_system/line_wrapper.rs`), which is `pub(crate)` there. Keep it in step
/// when gpui is bumped: [`unbreakable_segments`] is only right while the two
/// agree, and the tests below pin the cases that matter to a table.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, '\u{00C0}'..='\u{00FF}')
        || matches!(c, '\u{0100}'..='\u{017F}')
        || matches!(c, '\u{0180}'..='\u{024F}')
        || matches!(c, '\u{0400}'..='\u{04FF}')
        || matches!(c, '\u{1E00}'..='\u{1EFF}')
        || matches!(c, '\u{0300}'..='\u{036F}')
        || matches!(c, '\u{0980}'..='\u{09FF}')
        || matches!(
            c,
            '-' | '_'
                | '.'
                | '\''
                | '’'
                | '‘'
                | '$'
                | '%'
                | '@'
                | '#'
                | '^'
                | '~'
                | ','
                | '='
                | ':'
                | ';'
        )
        || matches!(c, '!' | ')' | ']' | '}' | '"' | '”' | '»' | '…')
        || matches!(c, '⋯')
        || matches!(c, '\u{202F}' | '\u{00A0}' | '\u{2011}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(line: &str) -> Vec<&str> {
        unbreakable_segments(line)
            .into_iter()
            .map(|range| &line[range])
            .collect()
    }

    #[test]
    fn a_line_wraps_where_gpui_wraps_it() {
        assert_eq!(pieces("Observability"), ["Observability"]);
        assert_eq!(pieces("Separate finding"), ["Separate", "finding"]);
        assert_eq!(pieces("  padded\tline  "), ["padded", "line"]);
        assert_eq!(pieces(""), Vec::<&str>::new());
    }

    /// The rule Knot #411's review caught: gpui never breaks at a hyphen, so
    /// a hyphenated word is one piece, however it is spelled.
    #[test]
    fn a_hyphenated_word_is_one_piece() {
        assert_eq!(pieces("Short-term plan"), ["Short-term", "plan"]);
        assert_eq!(pieces("--release"), ["--release"]);
        assert_eq!(pieces("x--y"), ["x--y"]);
        assert_eq!(pieces("a - b"), ["a", "-", "b"]);
    }

    /// Characters gpui does not keep in a word start a new piece, so a path
    /// or a URL wraps at its separators.
    #[test]
    fn a_path_or_url_wraps_before_its_separators() {
        assert_eq!(pieces("src/text/node.rs"), ["src", "/text", "/node.rs"]);
        assert_eq!(pieces("foo?b=2"), ["foo", "?b=2"]);
        assert_eq!(
            pieces("see)"),
            ["see)"],
            "closing punctuation stays with its word"
        );
    }

    #[test]
    fn segments_are_byte_ranges_on_multibyte_text() {
        assert_eq!(pieces("café-crème au lait"), ["café-crème", "au", "lait"]);
    }

    /// The table from #400 in Knot: a short-word column beside a long one.
    #[test]
    fn room_goes_to_the_column_that_wants_it() {
        let label = ColumnContent {
            min: 110.,
            max: 110.,
        };
        let prose = ColumnContent {
            min: 90.,
            max: 900.,
        };
        let flexes = column_flexes(&[label, prose], 48.);

        assert_eq!(
            flexes[0].basis, 110.,
            "the label column starts at its widest word"
        );
        assert_eq!(flexes[0].grow, 0., "and has no slack to grow into");
        assert_eq!(flexes[1].basis, 90.);
        assert_eq!(flexes[1].grow, 810., "the prose column takes the room");
    }

    #[test]
    fn a_table_with_no_slack_still_shares_room_by_width() {
        let flexes = column_flexes(
            &[
                ColumnContent { min: 60., max: 60. },
                ColumnContent {
                    min: 120.,
                    max: 120.,
                },
            ],
            48.,
        );
        assert_eq!(
            flexes.iter().map(|flex| flex.grow).collect::<Vec<_>>(),
            [60., 120.]
        );
    }

    #[test]
    fn a_column_shrinks_no_further_than_the_floor_or_its_content() {
        let flexes = column_flexes(
            &[
                ColumnContent {
                    min: 200.,
                    max: 300.,
                },
                ColumnContent { min: 30., max: 30. },
            ],
            48.,
        );
        assert_eq!(flexes[0].min_width, 48.);
        assert_eq!(flexes[1].min_width, 30.);
    }
}
