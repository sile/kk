//! The editor's state and its handlers.
//!
//! Every handler is pure with respect to the outside world: it mutates [`State`]
//! and returns plain values. The two handlers that imply I/O -- saving and
//! reloading -- return the text to write and accept the text that was read, and
//! the edge performs the actual read and write (see [`crate::Action`]).

use crate::{
    buffer::{TextBuffer, TextPosition},
    clipboard::Clipboard,
    search_prompt::{Highlight, SearchPrompt},
};

/// Where a recenter request puts the cursor's row in the text area.
///
/// The three are the places [`C-l`](crate::Action::ViewRecenter) cycles
/// through. Which one a press asks for is read off the viewport it is pressed
/// on; see [`State::recenter_candidates()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecenterPlace {
    /// The cursor's row in the middle of the drawn rows.
    Center,

    /// The cursor's row on the first drawn row.
    Top,

    /// The cursor's row on the last drawn row.
    Bottom,
}

impl RecenterPlace {
    /// The viewport row that puts the cursor's row at this place, in a text
    /// area `available_rows` tall.
    ///
    /// Every subtraction saturates at row `0`, the file's first line, so a
    /// place near the start of the file stops at the top and nothing blank is
    /// drawn above it. There is no matching bound here at the end of the file:
    /// a place may name a row past the last line and be pulled back by the
    /// caller's cap (see [`State::adjust_viewport()`]).
    fn row(self, cursor_row: usize, available_rows: usize) -> usize {
        match self {
            Self::Center => cursor_row.saturating_sub(available_rows / 2),
            Self::Top => cursor_row,
            Self::Bottom => cursor_row.saturating_sub(available_rows.saturating_sub(1)),
        }
    }
}

/// Columns reserved on the left of the text area while a search prompt is open:
/// a three-column count (two digits and the `+` that may overflow them), the
/// separator, and a space after it.
///
/// It is a constant, not derived from the buffer: the text's left edge must not
/// move as the query finds more or fewer hits. The renderer draws the gutter
/// over these columns and [`State::text_cols()`] keeps the viewport out of them,
/// so both read the width from here.
pub(crate) const HIT_GUTTER_COLS: usize = 5;

/// Everything the editor knows: the buffer, the cursor, and the surrounding
/// mode state.
///
/// The fields are public so renderers and the I/O edge can read them directly;
/// edits should go through the `handle_*` methods so the undo history and the
/// viewport stay consistent.
#[derive(Debug)]
pub struct State {
    /// The cursor's position in [`buffer`](State::buffer).
    pub cursor: TextPosition,

    /// The top-left position of the visible text area.
    pub viewport: TextPosition,

    /// Whether the next viewport adjustment should recenter the cursor where
    /// [`C-l`](crate::Action::ViewRecenter) puts it next.
    ///
    /// `false` means no request is pending: the adjustment runs the ordinary
    /// keep-it-visible rule, scrolling by the minimum that shows the cursor.
    /// A search step leaves it `false`, which is why stepping to a hit already
    /// on screen does not move the text.
    ///
    /// Which place that is, the adjustment works out from the viewport -- it
    /// cycles rather than centering every time, so the request cannot say where
    /// to go without knowing where the reader is. A request that does name a
    /// place, the startup position, uses
    /// [`center_viewport`](State::center_viewport) instead, so it does not
    /// disturb the cycle.
    pub recenter_viewport: bool,

    /// Whether the next viewport adjustment should center the cursor without
    /// stepping the [`C-l`](crate::Action::ViewRecenter) cycle.
    ///
    /// The startup position is the request this exists for: the named position
    /// belongs in the middle of the text area, and it is not a press of `C-l`,
    /// so the reader's first press should still center rather than move on to
    /// the next place. It does not name the place in a message either -- the
    /// reader did not ask for it -- which is the other reason it is kept apart.
    pub center_viewport: bool,

    /// The text being edited.
    pub buffer: TextBuffer,

    /// A message to show once, cleared after the next render.
    pub message: Option<String>,

    /// The mark, when a region has been started.
    pub mark: Option<TextPosition>,

    /// The clipboard text cut from or copied out of the buffer.
    pub clipboard: Clipboard,

    /// The clipboard the search prompt keeps its own edits in.
    ///
    /// The prompt's `C-k` cuts from the query, and the text it removed is not
    /// buffer text, so it goes here rather than into
    /// [`clipboard`](State::clipboard). The two are kept apart so a cut made
    /// while typing a query cannot replace what was copied out of the buffer:
    /// the prompt's `C-y` reads this one and the buffer's `C-y` reads the other.
    pub search_clipboard: Clipboard,

    /// Whether an edit has already been recorded in [`history`](State::history).
    pub editing: bool,

    /// Whether the last command was a cut, which is what makes a run of cuts
    /// collect into one clipboard entry rather than replacing it.
    ///
    /// [`handle_line_cut_tail()`](State::handle_line_cut_tail) sets it, and
    /// both `start_editing` and `finish_editing` clear it, so any edit or
    /// cursor move breaks the run.
    cut_chained: bool,

    /// Undo snapshots, oldest first.
    ///
    /// Each snapshot clones the whole [`TextBuffer`](crate::TextBuffer), but a
    /// clone only shares the lines that were not edited since (see
    /// [`TextLine`](crate::TextLine)), so a snapshot costs the lines the edit
    /// touched. The history is not capped.
    pub history: Vec<(TextPosition, TextBuffer)>,

    /// How many entries of [`history`](State::history) are still reachable by
    /// undo.
    pub undo_index: usize,

    /// The active search prompt, if one is open.
    ///
    /// TODO: whether a prompt is open is also what
    /// [`Mode::Search`](crate::Mode::Search) means, so the open-or-closed fact
    /// lives in two places. It is kept as an `Option` because `State` does not
    /// know the mode; folding the two together would mean moving the mode into
    /// `State`.
    pub search_prompt: Option<SearchPrompt>,

    /// Where the cursor and viewport were when the search prompt opened.
    ///
    /// A search moves the cursor to a hit and the viewport to follow it;
    /// leaving the prompt with `C-g` puts both back, so an abandoned search
    /// leaves no trace. The prompt is always opened through
    /// [`handle_search_enter()`](State::handle_search_enter), which fills this in.
    search_return: Option<(TextPosition, TextPosition)>,

    /// The current search matches, used for highlighting.
    pub highlight: Highlight,
}

impl State {
    /// Builds the editor state around an already-loaded `buffer`.
    pub fn new(buffer: TextBuffer) -> Self {
        Self {
            cursor: TextPosition::default(),
            viewport: TextPosition::default(),
            recenter_viewport: false,
            center_viewport: false,
            buffer,
            message: None,
            mark: None,
            clipboard: Clipboard::default(),
            search_clipboard: Clipboard::default(),
            editing: false,
            cut_chained: false,
            history: Vec::new(),
            undo_index: 0,
            search_prompt: None,
            search_return: None,
            highlight: Highlight::default(),
        }
    }

    /// Queues `message` to be shown once, on the next render.
    pub fn set_message<S: Into<String>>(&mut self, message: S) {
        self.message = Some(message.into());
    }

    /// Returns the cursor's position relative to the visible text area, which
    /// is where the terminal cursor belongs.
    pub fn terminal_cursor_position(&self) -> tuinix::Position {
        let pos = self.cursor_position();
        let screen_row = pos.row.saturating_sub(self.viewport.row);
        let screen_col = pos.col.saturating_sub(self.viewport.col);
        tuinix::Position {
            row: screen_row,
            col: screen_col,
        }
    }

    /// Returns the cursor's position, snapped back onto a character boundary.
    pub fn cursor_position(&self) -> TextPosition {
        self.buffer.adjust_to_char_boundary(self.cursor, true)
    }

    /// Returns the last row a cursor can stand on: the buffer's last line.
    ///
    /// Every handler that bounds a cursor row draws it from here, so the bound
    /// is written once. [`TextBuffer::rows()`](crate::TextBuffer::rows) is the
    /// number of lines, one past this, and is the wrong bound for a cursor: a
    /// cursor on it would stand on a row no line backs.
    fn row_count(&self) -> usize {
        self.buffer.last_row()
    }

    /// Returns how many rows the gutter's summary rows take from a text area
    /// `available_rows` tall: zero, one, or two -- one for a total above the
    /// visible slice and one for a total below it.
    ///
    /// The two totals are measured over the rows the text is *drawn* in, the
    /// same slice [`render_text_area()`](crate::render_text_area) walks, so a
    /// hit can be neither drawn beside a count nor totalled: the rows a summary
    /// takes are rows the totals have already been measured past, and a hit on
    /// one of them falls to the bottom total. Measuring against the area's full
    /// height instead would leave the displaced rows in a gap between the last
    /// drawn line and the bottom total, counted nowhere.
    ///
    /// The drawn height is itself defined by how many summaries are drawn, so
    /// the two are settled against each other; see [`text_rows()`](State::text_rows).
    /// This is the one place that decides, so the renderer and [`adjust_viewport()`](State::adjust_viewport)
    /// cannot disagree about how many rows the text is drawn in.
    ///
    /// A total above is only drawn when the viewport is not already at the
    /// buffer's first row, since no hit can lie above row 0; a total below only
    /// when there is a row past the slice to hold one. A one-row area keeps the
    /// top total and lets the bottom go, matching the drawing.
    pub fn summary_rows(&self, available_rows: usize) -> usize {
        available_rows.saturating_sub(self.text_rows(available_rows))
    }

    /// Returns the rows left for the text itself in a text area `available_rows`
    /// tall, after the gutter's summary rows have taken theirs.
    ///
    /// This is the height [`adjust_viewport()`](State::adjust_viewport) is
    /// scrolled against while a search prompt is open: the viewport is placed so
    /// the cursor lands in the rows the text is actually drawn in, not the rows
    /// the summaries are drawn over. Handing it the area's full height instead
    /// would place the cursor on a row a summary then covers, and the cursor
    /// would be hidden exactly when a hit lies at the edge.
    ///
    /// The drawn height and the two totals are settled together, because each
    /// decides the other: the totals are measured over the drawn slice, and a
    /// total that is drawn costs the text a row, which shortens that slice. The
    /// shorter slice can only move hits from inside it to below it -- the total
    /// above is fixed by the viewport alone -- so the bottom total never goes
    /// away by shortening the slice, and the loop below reaches the one slice
    /// that both the totals and the drawn rows agree on.
    pub fn text_rows(&self, available_rows: usize) -> usize {
        if self.search_prompt.is_none() {
            return available_rows;
        }
        let start_row = self.viewport.row;
        let buffer_rows = self.buffer.rows();
        let mut text_rows = available_rows;
        for _ in 0..3 {
            let end_row = (start_row + text_rows).min(buffer_rows);
            let (above, below) = self.hits_outside(start_row, end_row);
            let mut summaries = 0;
            if start_row > 0 && above > 0 {
                summaries += 1;
            }
            // The bottom total needs a row of its own, which a one-row area
            // does not have once the top total has claimed it.
            if available_rows > summaries && below > 0 {
                summaries += 1;
            }
            let settled = available_rows.saturating_sub(summaries);
            if settled == text_rows {
                break;
            }
            text_rows = settled;
        }
        text_rows
    }

    /// Returns the columns left for the text itself in a text area
    /// `available_cols` wide, after the gutter has taken its share while a
    /// search prompt is open.
    ///
    /// This is the width [`adjust_viewport()`](State::adjust_viewport) is
    /// scrolled against. The gutter takes columns, not rows, so a cursor near
    /// the right edge would otherwise be placed in a column the gutter is drawn
    /// over and end up off screen. Unlike the height, no fixed point is needed:
    /// the gutter is drawn whenever the prompt is open, so this width does not
    /// depend on where the viewport ends up.
    pub fn text_cols(&self, available_cols: usize) -> usize {
        if self.search_prompt.is_some() {
            available_cols.saturating_sub(HIT_GUTTER_COLS)
        } else {
            available_cols
        }
    }

    /// Returns how many hits start before `start_row` and how many start at or
    /// after `end_row`.
    ///
    /// The same two totals decide how many rows the gutter's summaries take
    /// (see [`summary_rows()`](State::summary_rows)) and what they read, so the
    /// count and the space reserved for it come from one place.
    pub fn hits_outside(&self, start_row: usize, end_row: usize) -> (usize, usize) {
        self.highlight.count_outside(start_row, end_row)
    }

    /// Scrolls the viewport just far enough to keep the cursor visible.
    ///
    /// `text_area_size` is the text area's own size, before the gutter's summary
    /// rows take theirs. The rows scrolled against are
    /// [`text_rows()`](State::text_rows) of it, so the cursor is placed in the
    /// rows the text is actually drawn in, not the rows a summary then covers --
    /// a search that lands the cursor at the edge would otherwise hide it. The
    /// horizontal scroll is against the area's full width; the gutter takes
    /// columns, not rows, and does not scroll with the text.
    ///
    /// When [`recenter_viewport`](State::recenter_viewport) is set, the
    /// requested places replace the vertical rule and the request is taken. The
    /// places are read off the viewport the last adjustment left: the row is
    /// centered when the viewport is already centered on the cursor, and
    /// otherwise moved on from there -- see
    /// [`handle_view_recenter()`](State::handle_view_recenter) for the cycle.
    /// Each place's own row saturates at the file's first line; at the file's
    /// end it is capped at the last page, the same bound the automatic recenter
    /// uses, so a manual `C-l` near either end draws no blank rows. Near the
    /// file's end two of the three places collapse onto the row already shown;
    /// a place that moves nothing is stepped over for the next, so the press
    /// still reaches the one place the file's end leaves room for. The column
    /// is centered for every place. The row is sized inside the height-settling
    /// loop below, so it lands against the drawn height the summary rows leave
    /// rather than one computed before they are known.
    ///
    /// A cursor that is more than a whole text area outside the viewport is
    /// centered too, rather than pinned to the edge it came in through: the
    /// screen it left has nothing near the destination to preserve, so the
    /// middle is the most useful place to show it. This is the one automatic
    /// recenter; it is measured against
    /// [`text_rows()`](State::text_rows), like the scroll it replaces.
    pub fn adjust_viewport(&mut self, text_area_size: tuinix::Size) {
        let cursor_pos = self.cursor_position();
        let available_cols = self.text_cols(text_area_size.cols);

        // A pending request replaces the vertical rule, not the whole
        // adjustment: the loop below still settles the height with the summary
        // rows, and the place's row is computed against the height each pass
        // ends up with. Doing it once, before the loop, would size the place
        // against a height the new viewport then changes -- a `Bottom` request
        // makes a summary row appear, which costs a drawn row, which leaves the
        // cursor one row past the last one.
        //
        // The place the request asks for is read from the viewport as the last
        // adjustment left it -- the viewport the reader is looking at, before
        // any of this runs. That choice is made once, here, not inside the
        // loop: the loop overwrites the viewport on its first pass, and a
        // second pass branching on the new value would step the cycle on a
        // second time. Only the row the chosen place works out to is left for
        // the loop, which re-evaluates it against each pass's height.
        let recenter = self.recenter_viewport;
        self.recenter_viewport = false;
        let center_request = std::mem::take(&mut self.center_viewport);
        // A keypress names the places to try, in order; a plain center request
        // (from a search or a jump) names only the center. The place the press
        // lands on is settled once, here, from the viewport as the last
        // adjustment left it -- the viewport the reader is looking at, before
        // any of this runs. It is not read inside the loop: the loop overwrites
        // the viewport on its first pass, and a second pass reading the new
        // value would step the cycle on a second time. Only the row the chosen
        // place works out to is left for the loop, which re-evaluates it against
        // each pass's height.
        let recenter_place = if center_request {
            Some(RecenterPlace::Center)
        } else if recenter {
            let available_rows = self.text_rows(text_area_size.rows);
            self.recenter_place(available_rows)
        } else {
            None
        };

        // The height the text is drawn in depends on which summary rows are
        // shown, and those depend on the viewport -- so the two are settled
        // together rather than one from the other. Each pass scrolls against the
        // height the previous pass's viewport calls for, which is the fixed
        // point the renderer then draws; a couple of passes cover every case.
        for _ in 0..3 {
            let available_rows = self.text_rows(text_area_size.rows);
            let before = self.viewport.row;

            if let Some(place) = recenter_place {
                // The manual place is capped at the file's last page, the same
                // bound the automatic recenter below applies to its center. The
                // place's own `saturating_sub` already holds it at the file's
                // first line, so the start needs no separate bound; the cap is
                // the end's mirror. Without it a `Top`/`Center` near the last
                // line names a row past the file and draws blank rows under it,
                // the asymmetry with the start the cap removes. A file shorter
                // than the area has no page below `0` to move to, the cap
                // saturates to `0`, and the rows under the last line are the
                // file not filling the screen -- shown, as every editor shows
                // them, not removed.
                let last_viewport = (self.row_count() + 1).saturating_sub(available_rows);
                self.viewport.row = place.row(cursor_pos.row, available_rows).min(last_viewport);
            } else {
                // The distance from the cursor to the edge of the viewport, in
                // the rows the text is drawn in. A cursor on the row just past
                // an edge is one row out, however it got there.
                let rows_out = if cursor_pos.row < self.viewport.row {
                    self.viewport.row - cursor_pos.row
                } else {
                    cursor_pos
                        .row
                        .saturating_sub(self.viewport.row + available_rows)
                };

                // A cursor more than a screen out shares no row with the screen
                // being left, so it is centered rather than pinned to the edge
                // it came in through; nearer than that, the lines around the
                // cursor are lines the reader was just looking at, and the
                // minimum scroll keeps the connection. The center is floored by
                // the buffer's end, not by its start: a cursor near the last
                // line has too little file below it to fill the lower half of
                // the frame, and centering past the end would put the cursor
                // against the bottom edge with blank rows under it. Flooring at
                // the last page instead leaves the cursor on the last drawn
                // row, the same place the start of the file gets at the top.
                if rows_out > available_rows {
                    let centered = cursor_pos.row.saturating_sub(available_rows / 2);
                    // Two bounds, not one formula. The `Bottom` place is the
                    // floor on how far the center is pulled back: a jump near
                    // the end is pulled up until the cursor sits on the area's
                    // last drawn row, the same row a reader who then asks for
                    // "Cursor at bottom" would get. The file's end is the hard
                    // cap: the viewport cannot sit so low that a row past the
                    // last line is asked for. `max` then `min` applies the floor
                    // inside the cap. (The floor and the center are different
                    // rows by construction -- `cursor - (n - 1)` against
                    // `cursor - n / 2` -- so no single expression can be both.)
                    let bottom = RecenterPlace::Bottom.row(cursor_pos.row, available_rows);
                    let last_viewport = (self.row_count() + 1).saturating_sub(available_rows);
                    self.viewport.row = centered.max(bottom).min(last_viewport);
                } else if cursor_pos.row < self.viewport.row {
                    // Cursor is above viewport, scroll up
                    self.viewport.row = cursor_pos.row;
                } else if cursor_pos.row >= self.viewport.row + available_rows {
                    // Cursor is below viewport, scroll down
                    self.viewport.row = cursor_pos
                        .row
                        .saturating_sub(available_rows.saturating_sub(1));
                }
            }

            if self.viewport.row == before {
                break;
            }
        }

        // A `C-l` press names no place: with the places collapsing at an end of
        // the file, a line naming the place the reader asked for would report a
        // place the cursor is not at once the cap bites, and on an ordinary
        // press it would only repeat what the cursor's motion already shows.
        // The startup position and the automatic recenter never spoke.

        // A pending request centers the column as well as placing the row: the
        // place is about where the cursor sits in the text area, and a hit near
        // the right edge of a long line belongs in the middle of the width the
        // same way. Without a request, the horizontal rule is the minimum-scroll
        // one, symmetric with the vertical rule above.
        if recenter_place.is_some() {
            self.viewport.col = cursor_pos.col.saturating_sub(available_cols / 2);
        } else if cursor_pos.col < self.viewport.col {
            // Cursor is left of viewport, scroll left
            self.viewport.col = cursor_pos.col;
        } else if cursor_pos.col >= self.viewport.col + available_cols {
            // Cursor is right of viewport, scroll right
            self.viewport.col = cursor_pos
                .col
                .saturating_sub(available_cols.saturating_sub(1));
        }
    }

    fn start_editing(&mut self) {
        // Any edit that starts here breaks a run of cuts, including the ones
        // that continue an undo run and so never call `finish_editing`.
        // `handle_line_cut_tail` reads the flag before this and re-arms it
        // after, which is what lets one cut chain onto the next.
        self.cut_chained = false;

        if self.editing {
            return;
        }

        self.history.push((self.cursor, self.buffer.clone()));
        self.undo_index = self.history.len();

        self.editing = true;
    }

    /// Ends the current edit run, so the next edit records a new snapshot.
    ///
    /// Also ends a run of cuts. Together with `start_editing`, which clears the
    /// same flag, this covers every break: the edits that only start an edit
    /// (inserting, deleting a character) and the moves and commands that only
    /// end one (moving the cursor, undoing).
    /// [`handle_line_cut_tail()`](State::handle_line_cut_tail) reads the flag
    /// before its own `start_editing` and re-arms it after its
    /// `finish_editing`, which is what chains one cut onto the next.
    pub fn finish_editing(&mut self) {
        self.editing = false;
        self.cut_chained = false;
    }

    /// Moves the cursor up one row.
    pub fn handle_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(1);
        self.finish_editing();
    }

    /// Moves the cursor to `row`/`col` of the buffer and centers it.
    ///
    /// Both are absolute and 0-based. Out-of-range positions are clamped rather
    /// than refused: the row is clamped to the buffer and the column to the
    /// target line, and the column is then snapped back onto a character
    /// boundary.
    ///
    /// The cursor is centered by the next
    /// [`adjust_viewport()`](State::adjust_viewport), not left for the generic
    /// keep-it-visible rule. This is the startup path, where the named position
    /// is the reason the file was opened and belongs in the middle of the text
    /// area rather than against an edge.
    ///
    /// The request is a plain center, not a `C-l` press: the next adjustment
    /// centers the cursor, and the `C-l` after that reads the centered viewport
    /// and moves on to the top, as a first press would.
    pub fn handle_cursor_to_position(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.row_count());
        self.cursor.col = self.buffer.cols(self.cursor.row).min(col);
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        self.center_viewport = true;
        self.finish_editing();
    }

    /// Moves the cursor to the character a click at `row`/`col` of the visible
    /// text area landed on.
    ///
    /// Both are relative to the text area, not the terminal, so the viewport is
    /// added here rather than by the caller.
    pub fn handle_cursor_to_screen_position(&mut self, row: usize, col: usize) {
        self.cursor.row = (self.viewport.row + row).min(self.row_count());
        self.cursor.col = self
            .buffer
            .cols(self.cursor.row)
            .min(self.viewport.col + col);
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        self.finish_editing();
    }

    /// Scrolls the viewport `rows` lines down, or up when `rows` is negative.
    ///
    /// This is the wheel: it drives the view, not the cursor. The cursor keeps
    /// its screen row and rides along, so the text slides under a cursor that
    /// appears to stand still. That is what makes a notch scroll a screen that
    /// has room to move, rather than doing nothing until the cursor reaches an
    /// edge the way a run of [`handle_cursor_down()`](State::handle_cursor_down)
    /// would.
    ///
    /// The cursor moves with the viewport because
    /// [`adjust_viewport()`](State::adjust_viewport) pulls the viewport back to
    /// the cursor on the next render, so a viewport moved on its own would snap
    /// right back; moving the cursor by the same rows leaves it inside the newly
    /// shown slice and the adjustment alone.
    ///
    /// `text_area_size` is the text area's own size, like the one
    /// [`adjust_viewport()`](State::adjust_viewport) takes: the viewport is
    /// clamped against [`text_rows()`](State::text_rows), so a notch during an
    /// open search prompt counts the gutter's summary rows out of the height and
    /// moves the text by *drawn* rows.
    ///
    /// Only the bottom of the file breaks the rule that the cursor keeps its
    /// screen row. When the viewport comes to rest on the last line and the
    /// cursor still has screen rows left under it, keeping them would stop the
    /// view a screen short of the file's end, so the cursor gives them up and
    /// goes to the last row. That is what lets the reader bring the last line to
    /// the frame's last row. Once both are at that edge a further notch down
    /// changes nothing.
    ///
    /// The top needs no such exception: the cursor's screen row is measured from
    /// the top of the text area, so a viewport at row `0` already draws the
    /// cursor where its screen row says, with no rows to give up.
    pub fn handle_scroll(&mut self, rows: isize, text_area_size: tuinix::Size) {
        let drawn_rows = self.text_rows(text_area_size.rows);
        let last_row = self.row_count();
        let last_viewport = (last_row + 1).saturating_sub(drawn_rows);
        let start_row = self.viewport.row;

        // The cursor's row on screen before the notch. A cursor already off
        // screen (which the ordinary rules should not produce) saturates to the
        // top edge and the next `adjust_viewport` corrects it.
        let screen_row = self.cursor.row.saturating_sub(start_row);

        let target = start_row.saturating_add_signed(rows);
        self.viewport.row = target.min(last_viewport);

        // Phase one: the cursor keeps its screen row, so the text slides under
        // it. `screen_row` is measured from the top of the text area, so at the
        // top edge the clamp to `0` already lands the cursor where its screen
        // row puts it and there is nothing further to give up.
        self.cursor.row = self.viewport.row + screen_row;

        // Phase two, at the bottom edge only: the viewport clamped at
        // `last_viewport` while the cursor still had screen rows left under it,
        // so keeping them would leave the cursor short of the last line and the
        // view stopped a screen short of the file's end. The cursor gives them
        // up and goes to the last row, which is what lets the last line reach
        // the frame's last row.
        if target > last_viewport {
            self.cursor.row = last_row;
        }

        self.cursor.row = self.cursor.row.min(last_row);
        self.finish_editing();
    }

    /// Moves the cursor down one row.
    pub fn handle_cursor_down(&mut self) {
        self.cursor.row = self.cursor.row.saturating_add(1).min(self.row_count());
        self.finish_editing();
    }

    /// Moves the cursor left one column, wrapping to the previous line's end.
    ///
    /// While a search prompt is open, this moves the query's insertion cursor
    /// instead.
    pub fn handle_cursor_left(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            search.move_cursor_left();
            return;
        }

        if self.cursor.col > 0 {
            self.cursor.col = self.cursor.col.saturating_sub(1);
            self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        } else if self.cursor.row > 0 {
            // Move to end of previous line
            self.cursor.row = self.cursor.row.saturating_sub(1);
            self.cursor.col = self.buffer.cols(self.cursor.row);
        }
        self.finish_editing();
    }

    /// Moves the cursor right one column, wrapping to the next line's start.
    ///
    /// While a search prompt is open, this moves the query's insertion cursor
    /// instead.
    pub fn handle_cursor_right(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            search.move_cursor_right();
            return;
        }

        let current_cols = self.buffer.cols(self.cursor.row);
        if self.cursor.col < current_cols {
            self.cursor.col = self.cursor.col.saturating_add(1);
            self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, false);
        } else if self.cursor.row < self.row_count() {
            // Move to beginning of next line
            self.cursor.row = self.cursor.row.saturating_add(1);
            self.cursor.col = 0;
        }
        self.finish_editing();
    }

    /// Moves the cursor to its line's first column.
    pub fn handle_cursor_line_start(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            search.move_cursor_to_start();
            return;
        }

        self.cursor.col = 0;
        self.finish_editing();
    }

    /// Moves the cursor to its line's end.
    pub fn handle_cursor_line_end(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            search.move_cursor_to_end();
            return;
        }

        self.cursor.col = self.buffer.cols(self.cursor.row);
        self.finish_editing();
    }

    /// Moves the cursor to the start of the buffer.
    pub fn handle_cursor_buffer_start(&mut self) {
        self.cursor = TextPosition::default();
        self.finish_editing();
    }

    /// Moves the cursor to the last line of the buffer.
    pub fn handle_cursor_buffer_end(&mut self) {
        self.cursor.row = self.row_count();
        self.cursor.col = 0;
        self.finish_editing();
    }

    /// Deletes the character before the cursor.
    ///
    /// At the start of the line this joins it onto the previous one, so a
    /// press there deletes the newline. At the buffer's first row there is no
    /// previous line and nothing to delete.
    ///
    /// While a search prompt is open, this deletes from the query instead and
    /// re-runs it.
    pub fn handle_char_delete_backward(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            if search.delete_char_backward() {
                self.rerun_query();
            }
            return;
        }

        self.start_editing();
        if let Some(new_pos) = self.buffer.delete_char_before(self.cursor) {
            self.cursor = new_pos;
        }
    }

    /// Deletes the character under the cursor.
    ///
    /// While a search prompt is open, this deletes from the query instead and
    /// re-runs it.
    pub fn handle_char_delete_forward(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            if search.delete_char_forward() {
                self.rerun_query();
            }
            return;
        }

        self.start_editing();
        self.buffer.delete_char_at(self.cursor);
    }

    /// Recomputes the highlight for the query typed so far.
    ///
    /// It only refreshes the matches: the cursor stays where it is until `C-s`
    /// or `C-r` asks for a hit, so typing a query does not drag the buffer
    /// around under the prompt.
    fn rerun_query(&mut self) {
        let Some(search) = &self.search_prompt else {
            return;
        };
        self.highlight = search.search(&self.buffer);
    }

    /// Renders the buffer for saving and returns the text to persist.
    ///
    /// The caller (the I/O edge) writes it and then calls [`Self::report_saved`]
    /// once the write succeeded; the core never touches the file system.
    pub fn handle_buffer_save(&mut self) -> String {
        self.set_message("Saving");
        self.buffer.to_text()
    }

    /// Reports that the edge wrote `chars` characters out.
    pub fn report_saved(&mut self, chars: usize) {
        self.set_message(format!("Saved {chars} chars"));
    }

    /// Replaces the buffer with `text`, as the edge would after reading the file
    /// back, and adjusts the cursor so it stays on a valid position.
    pub fn handle_buffer_reload(&mut self, text: &str) {
        self.finish_editing();
        self.start_editing();

        self.buffer = TextBuffer::new(text);

        // Try to preserve the cursor position, but adjust if the file has
        // changed. The row is clamped to the buffer's last line, so a cursor
        // that survives keeps whatever fits of its column on that line; a
        // cursor pulled back from past the end lands on the last line's end
        // rather than at column 0 of nowhere.
        self.cursor.row = self.cursor.row.min(self.row_count());

        let max_col = self.buffer.cols(self.cursor.row);
        self.cursor.col = self.cursor.col.min(max_col);

        // Adjust cursor to proper character boundary
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);

        self.set_message("Reloaded");
        self.finish_editing();
    }

    /// Reports that the extension mode has been entered.
    ///
    /// Its own chords drop the `C-x` prefix, so the message names the prefix
    /// the user is under.
    pub fn handle_ext_enter(&mut self) {
        self.set_message("C-x");
    }

    /// Inserts `ch` at the cursor.
    ///
    /// While a search prompt is open, the character enters the query instead
    /// and re-runs it.
    pub fn handle_char_insert(&mut self, ch: char) {
        if let Some(search) = &mut self.search_prompt {
            search.insert_char(ch);
            self.rerun_query();
            return;
        }

        self.start_editing();
        self.cursor = self.buffer.insert_char_at(self.cursor, ch);
    }

    /// Splits the line at the cursor.
    pub fn handle_newline_insert(&mut self) {
        self.finish_editing();
        self.start_editing();
        self.cursor = self.buffer.insert_newline_at(self.cursor);
        self.finish_editing();
    }

    /// Restores the buffer and cursor from the previous history snapshot.
    ///
    /// An edit still in progress is closed first: the snapshot it already
    /// pushed is the state to come back to, so nothing new is recorded.
    /// Reports `Nothing to undo` when there is nothing left to restore.
    pub fn handle_buffer_undo(&mut self) {
        self.finish_editing();

        let Some(i) = self.undo_index.checked_sub(1) else {
            self.set_message("Nothing to undo");
            return;
        };

        let (cursor, buffer) = self.history[i].clone();
        self.cursor = cursor;
        self.buffer = buffer;
        self.undo_index = i;
        self.set_message(format!("Undo ({})", self.history.len() - i));
    }

    /// Sets the mark at the cursor, or clears it when already there.
    pub fn handle_mark_set(&mut self) {
        self.finish_editing();

        let cursor_pos = self.cursor_position();
        if self.mark == Some(cursor_pos) {
            // If mark is already at cursor position, deactivate it
            self.mark = None;
            self.set_message("Mark deactivated");
        } else {
            // Set mark at current cursor position
            self.mark = Some(cursor_pos);
            self.set_message("Mark set");
        }
    }

    /// Copies the region between the mark and the cursor to the clipboard and
    /// deletes it, leaving the cursor at the region's start.
    ///
    /// The mark is cleared either way. Reports `No mark set` when there is no
    /// mark, and `Nothing to cut` when the region is empty.
    pub fn handle_mark_cut(&mut self) {
        self.finish_editing();

        if let Some((start, end, text)) = self.take_mark_region("No mark set", "Nothing to cut") {
            // Open the edit run only once there is something to delete, so
            // an empty region records no snapshot. The whole cut is one
            // step: every change `delete_range` makes lands between these
            // two calls.
            self.start_editing();
            self.buffer.delete_range(start, end);
            self.cursor = start;
            self.mark = None;
            self.finish_editing();

            self.clipboard.write(&text);
            self.set_message(format!("Cut {} characters", text.chars().count()));
        }
    }

    /// Copies the region between the mark and the cursor to the clipboard,
    /// leaving the buffer and the region in place.
    ///
    /// This is kk's `copy`: the one command that also hands the text to the
    /// terminal's own clipboard, so it can be pasted outside kk. The mark is
    /// cleared and the cursor moves to the region's start, exactly as
    /// [`handle_mark_cut()`](State::handle_mark_cut) leaves them, so a cut and a
    /// copy differ only in whether the region is deleted. Reports `No mark set`
    /// when there is no mark, and `Nothing to copy` when the region is empty.
    pub fn handle_mark_copy(&mut self) {
        self.finish_editing();

        if let Some((start, _end, text)) = self.take_mark_region("No mark set", "Nothing to copy") {
            self.clipboard.write(&text);
            self.cursor = start;
            self.mark = None;
            self.set_message(format!("Copied {} characters", text.chars().count()));
        }
    }

    /// Takes the mark and returns the region it and the cursor bound, as the
    /// start and end positions and the text between them.
    ///
    /// The mark is cleared. When there is no mark, or the region is empty, it
    /// reports `no_mark_message` or `empty_message` and returns `None`; the two
    /// are arguments so a cut talks about cutting and a copy about copying.
    fn take_mark_region(
        &mut self,
        no_mark_message: &str,
        empty_message: &str,
    ) -> Option<(TextPosition, TextPosition, String)> {
        let mark_pos = match self.mark.take() {
            Some(mark_pos) => mark_pos,
            None => {
                self.set_message(no_mark_message);
                return None;
            }
        };
        let cursor_pos = self.cursor_position();
        let (start, end) = if mark_pos <= cursor_pos {
            (mark_pos, cursor_pos)
        } else {
            (cursor_pos, mark_pos)
        };

        match self.buffer.text_in_range(start, end) {
            Some(text) if !text.is_empty() => Some((start, end, text)),
            _ => {
                self.set_message(empty_message);
                None
            }
        }
    }

    /// Inserts the clipboard's contents at the cursor.
    ///
    /// Newlines in the contents become line breaks. While a search prompt is
    /// open, the contents enter the query instead and re-run it; the prompt
    /// reads [`search_clipboard`](State::search_clipboard), so what it pastes is
    /// what its own `C-k` cut and never the buffer's.
    pub fn handle_clipboard_paste(&mut self) {
        if let Some(search) = &mut self.search_prompt {
            let text = self.search_clipboard.read();

            if text.is_empty() {
                self.set_message("Clipboard is empty");
                return;
            }

            // Insert clipboard text at current cursor position in search query
            for ch in text.chars() {
                // Skip control characters and newlines in search query
                if !ch.is_control() {
                    search.insert_char(ch);
                }
            }

            // Re-run the search with updated query
            self.rerun_query();
            return;
        };

        self.finish_editing();

        let text = self.clipboard.read();

        if text.is_empty() {
            self.set_message("Clipboard is empty");
            return;
        }

        // Split text into lines
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            self.set_message("Nothing to paste");
            return;
        }
        self.start_editing();

        // Insert the text
        if lines.len() == 1 {
            // Single line paste
            let line = lines[0];
            for ch in line.chars() {
                self.cursor = self.buffer.insert_char_at(self.cursor, ch);
            }
            self.set_message(format!("Pasted {} characters", line.chars().count()));
        } else {
            // Multi-line paste
            let mut total_chars = 0;

            // Insert first line
            for ch in lines[0].chars() {
                self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                total_chars += 1;
            }

            // Insert newline and subsequent lines
            for line in &lines[1..] {
                self.cursor = self.buffer.insert_newline_at(self.cursor);
                total_chars += 1; // Count the newline

                for ch in line.chars() {
                    self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                    total_chars += 1;
                }
            }

            self.set_message(format!(
                "Pasted {} characters across {} lines",
                total_chars,
                lines.len()
            ));
        }

        self.finish_editing();
    }

    /// Asks the next viewport adjustment to put the cursor somewhere, cycling
    /// `C-l` through the places there are to put it.
    ///
    /// The cycle is center, top, bottom, center, and the place is worked out
    /// when the request is applied, from where the viewport already is -- not
    /// from a place remembered here. Pressing the key repeatedly therefore
    /// visits the three places in turn: a viewport that is already centered is
    /// moved to the top, one on the top row to the bottom, and anything else is
    /// centered. Centering alone is idempotent, so without the cycle a second
    /// press would do nothing observable.
    ///
    /// Working the place out from the viewport also keeps the cycle honest
    /// about what the reader is looking at: a search step or a cut between two
    /// presses moves the view, and the next press continues from there rather
    /// than from a place that is no longer on screen.
    ///
    /// This does not place anything itself, because the text area's height is
    /// not known until render time: the request is what
    /// [`adjust_viewport()`](State::adjust_viewport) reads, and that is where
    /// the place is worked out. The press names no place, so it says nothing on
    /// the message line: near an end of the file the cap can land the cursor
    /// somewhere other than the place the press asked for, and a line naming
    /// the request would then be false.
    pub fn handle_view_recenter(&mut self) {
        self.finish_editing();
        self.recenter_viewport = true;
    }

    /// The place the next recenter should land on, read off where the viewport
    /// is now.
    ///
    /// The cycle is center, top, bottom, center. Which place comes next turns on
    /// which of the three the viewport is already at, and the three are told
    /// apart by the row the viewport sits on:
    ///
    /// - on the top row already -- the viewport's row is the one
    ///   [`RecenterPlace::Top`] would use -- so the place after it,
    ///   [`RecenterPlace::Bottom`], is tried first;
    /// - centered already, so [`RecenterPlace::Top`] is tried first;
    /// - anywhere else, including a viewport the buffer's first row clamped, so
    ///   the center is tried first.
    ///
    /// The places are tried in the cycle order center, top, bottom, starting
    /// from the one after the place the viewport is already on, and the first
    /// that the cap at the file's end leaves on a row other than the one the
    /// viewport is on is the place the press lands on. The cap collapses a place
    /// onto the row already shown near the file's end -- two of the three, with
    /// a file that fits the whole area, collapse all three -- and a place that
    /// would move nothing is stepped over rather than landed on. Reading the
    /// place off the cap's own result, not off the place's name, is what keeps a
    /// press near the file's end from landing on a fixed point and sticking
    /// there; the cap is applied here the same way
    /// [`adjust_viewport()`](State::adjust_viewport) applies it.
    ///
    /// The top is tested first when telling the places apart because they
    /// overlap: a cursor in the middle of the drawn rows sits on the top row
    /// *and* is centered at the same time when the area's height has not changed
    /// since the last adjustment. Testing the center first would leave the cycle
    /// there, and the third press would never reach the bottom.
    ///
    /// The result is `None` when every place collapses onto the row the viewport
    /// is already on. That is a file that fits the whole area, where there is
    /// nowhere else for the cursor to be drawn and the key does nothing.
    fn recenter_place(&self, available_rows: usize) -> Option<RecenterPlace> {
        use RecenterPlace::{Bottom, Center, Top};

        let cursor_row = self.cursor.row;
        let last_viewport = (self.row_count() + 1).saturating_sub(available_rows);
        let center_row = Center.row(cursor_row, available_rows);
        let top_row = Top.row(cursor_row, available_rows);

        // The place after the one the viewport is on, then the other two in
        // cycle order. The bottom is tested before the center for a viewport
        // that is neither: a file's first line clamps where the start places
        // would go, and the bottom is the one place the start cannot shadow.
        let order = if self.viewport.row == top_row {
            [Bottom, Center, Top]
        } else if self.viewport.row == center_row {
            [Top, Bottom, Center]
        } else {
            [Center, Top, Bottom]
        };

        order.into_iter().find(|place| {
            place.row(cursor_row, available_rows).min(last_viewport) != self.viewport.row
        })
    }

    /// Deletes from the cursor to the end of the line, or joins the next line
    /// when the cursor is already at the end.
    ///
    /// The removed text goes to the clipboard. A cut that follows another one
    /// without a break in between is chained onto the entry the previous cut
    /// left rather than replacing it, so a run of `C-k` collects what it
    /// removed into one entry (see [`finish_editing()`](State::finish_editing) for
    /// what counts as a break).
    pub fn handle_line_cut_tail(&mut self) {
        let append = self.cut_chained;
        self.start_editing();

        let cursor_pos = self.cursor_position();
        let current_line_cols = self.buffer.cols(cursor_pos.row);

        if cursor_pos.col >= current_line_cols {
            // Cursor is at or past end of line - delete the newline (merge with next line)
            if self.buffer.line(cursor_pos.row + 1).is_some() {
                // Copy the newline to clipboard
                if append {
                    self.clipboard.append("\n");
                } else {
                    self.clipboard.write("\n");
                }

                self.buffer.join_next_line(cursor_pos.row);
                if append {
                    self.set_message("Appended newline");
                } else {
                    self.set_message("Cut newline");
                }
            }
        } else {
            // Delete from cursor to end of line and copy to clipboard
            let cut_text = self.buffer.cut_line_tail(cursor_pos.row, cursor_pos.col);

            match cut_text {
                Some(cut_text) if !cut_text.is_empty() => {
                    // Copy to clipboard
                    if append {
                        self.clipboard.append(&cut_text);
                    } else {
                        self.clipboard.write(&cut_text);
                    }

                    let chars = cut_text.chars().count();
                    if append {
                        self.set_message(format!("Appended {chars} characters"));
                    } else {
                        self.set_message(format!("Cut {chars} characters"));
                    }
                }
                _ => self.set_message("Nothing to cut"),
            }
        }

        // Re-arm the flag so the next cut chains onto this one. This runs for a
        // no-op cut too: what chains is the command, not its effect.
        self.finish_editing();
        self.cut_chained = true;
    }

    /// Drops the mark and the search state, and reports `Canceled`.
    ///
    /// This is the `C-g` of the modes that hold no prompt; a prompt is left
    /// through [`handle_search_cancel()`](State::handle_search_cancel).
    pub fn handle_cancel(&mut self) {
        self.clear_transient_state();
        self.set_message("Canceled");
    }

    /// Drops the mark and every trace of a search, prompt included.
    ///
    /// Commands that leave the editing mode behind -- cancelling, saving --
    /// call this, so a search cannot outlive the mode that opened it. What
    /// the cursor does is the caller's business: cancelling puts it back and
    /// saving leaves it where it is.
    pub fn clear_transient_state(&mut self) {
        self.search_return = None;
        self.finish_editing();
        self.mark = None;
        self.search_prompt = None;
        self.highlight = Highlight::default();
    }

    /// Cuts from the query cursor to the end of the query.
    ///
    /// The removed text goes to [`search_clipboard`](State::search_clipboard)
    /// rather than to the buffer's clipboard, so the prompt's cuts stay its
    /// own. Nothing is written when there is nothing after the cursor, so a
    /// prompt cut empty does not wipe what an earlier cut left behind.
    ///
    /// Does nothing when no search prompt is open.
    pub fn handle_search_cut_query(&mut self) {
        let Some(search) = &mut self.search_prompt else {
            return;
        };

        let cut = search.cut_to_end();

        if cut.is_empty() {
            self.set_message("Nothing to cut");
            return;
        }

        self.search_clipboard.write(&cut);
        self.rerun_query();
        self.set_message(format!("Cut {} characters", cut.chars().count()));
    }

    /// Opens the search prompt with an empty query.
    ///
    /// The cursor and viewport are remembered first, so
    /// [`handle_search_cancel()`](State::handle_search_cancel) can put the buffer
    /// back where the search found it. The mark and the previous highlight are
    /// dropped: they belong to the editing that the prompt interrupts.
    pub fn handle_search_enter(&mut self) {
        self.clear_transient_state();
        self.search_return = Some((self.cursor, self.viewport));
        self.search_prompt = Some(SearchPrompt::new());
        self.set_message("Entered the search prompt");
    }

    /// Abandons the search prompt and returns to where it was opened.
    ///
    /// The cursor and viewport go back to their remembered positions, so a
    /// search that was merely looked at leaves the buffer untouched. The query
    /// itself is kept in [`search_clipboard`](State::search_clipboard), exactly
    /// as [`handle_search_finish()`](State::handle_search_finish) keeps it: what
    /// an abandoned search leaves behind is the word it looked for, never a
    /// change to the buffer. Does nothing when no prompt is open.
    pub fn handle_search_cancel(&mut self) {
        if self.search_prompt.is_none() {
            return;
        }

        self.save_query_to_search_clipboard();

        if let Some((cursor, viewport)) = self.search_return.take() {
            self.cursor = cursor;
            self.viewport = viewport;
        }

        self.clear_transient_state();
        self.set_message("Canceled");
    }

    /// Finishes the search prompt and stays on the hit the cursor sits on.
    ///
    /// Only the prompt and the highlight go away: the cursor keeps the position
    /// the hit commands gave it. The query is put in
    /// [`search_clipboard`](State::search_clipboard), so the word that was
    /// searched for is there for a later `C-y`. Does nothing when no prompt is
    /// open.
    pub fn handle_search_finish(&mut self) {
        if self.search_prompt.is_none() {
            return;
        }

        self.save_query_to_search_clipboard();
        self.clear_transient_state();
    }

    /// Copies the open prompt's query to
    /// [`search_clipboard`](State::search_clipboard).
    ///
    /// An empty query is not written, so leaving a prompt that was never typed
    /// into does not wipe the word an earlier search left.
    fn save_query_to_search_clipboard(&mut self) {
        let Some(search) = &self.search_prompt else {
            return;
        };

        let query = search.query();
        if !query.is_empty() {
            self.search_clipboard.write(&query);
        }
    }

    /// Moves the cursor to the next match after it, wrapping to the first.
    ///
    /// Does nothing when no search prompt is open.
    ///
    /// The cursor is moved with the ordinary keep-it-visible rule rather than
    /// centered: a hit already on screen must not scroll the text out from
    /// under the reader.
    pub fn handle_search_next_hit(&mut self) {
        if self.search_prompt.is_none() {
            return;
        }

        self.finish_editing();

        let current_pos = self.cursor_position();

        // Find the next highlight item after the current cursor position
        if let Some(next_item) = self
            .highlight
            .items()
            .iter()
            .find(|item| item.start_position > current_pos)
        {
            self.cursor = next_item.start_position;
        } else if let Some(first_item) = self.highlight.first() {
            // Wrap around to the first item
            self.cursor = first_item.start_position;
        }
    }

    /// Moves the cursor to the previous match before it, wrapping to the last.
    ///
    /// Does nothing when no search prompt is open.
    ///
    /// Like [`handle_search_next_hit()`](State::handle_search_next_hit), the
    /// cursor follows the ordinary keep-it-visible rule rather than centering,
    /// so stepping back through hits the reader just passed does not shuffle
    /// the text each time.
    pub fn handle_search_prev_hit(&mut self) {
        if self.search_prompt.is_none() {
            return;
        }

        self.finish_editing();

        let current_pos = self.cursor_position();

        // Find the previous highlight item before the current cursor position
        if let Some(prev_item) = self
            .highlight
            .items()
            .iter()
            .rev()
            .find(|item| item.start_position < current_pos)
        {
            self.cursor = prev_item.start_position;
        } else if let Some(last_item) = self.highlight.last() {
            // Wrap around to the last item
            self.cursor = last_item.start_position;
        }
    }
}
