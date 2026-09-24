//! Helpers for reading a rendered [`tuinix::Frame`] back as text.

/// Returns the characters of `frame` on `row` as a string, in column order.
///
/// Columns the frame does not paint (or that fall past `cols`) come back as
/// spaces, so the result is always exactly `cols` characters wide.
pub fn row_text(frame: &tuinix::Frame, row: usize, cols: usize) -> String {
    let mut cells = vec![' '; cols];
    for (position, ch) in frame.chars() {
        if position.row == row && position.col < cols {
            cells[position.col] = ch.value();
        }
    }
    cells.into_iter().collect()
}
