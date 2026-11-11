//! The subsong selection, the playback position, and the pattern data.

use std::os::raw::c_int;

use crate::command::Command;
use crate::error::OpenmptError;
use crate::error::Result;
use crate::info::check_index;
use crate::module::Module;
use crate::order::OrderEntry;

impl Module {
    /// Selects one subsong.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is outside `0..num_subsongs`. The value
    /// `-1` is an error too, although libopenmpt accepts it as "all subsongs".
    /// This crate renders one subsong at a time.
    pub fn select_subsong(&mut self, index: i32) -> Result<()> {
        check_subsong(self.num_subsongs(), index)?;
        self.clear_error();
        // SAFETY: the handle is live, and the index is in range.
        let ok = unsafe { openmpt_ext_sys::openmpt_module_select_subsong(self.core(), index) };
        self.checked("select subsong", &format!("subsong {index}"), ok)
    }

    /// Returns the selected subsong.
    #[must_use]
    pub fn selected_subsong(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_selected_subsong(self.core()) }
    }

    /// Sets the number of times that the order list repeats.
    ///
    /// A count of 0 stops the render at the end of the order list.
    ///
    /// # Errors
    ///
    /// Returns an error when `count` is negative. The library reads a negative
    /// count as "repeat forever", and a render loop that waits for the song end
    /// then never stops. This crate renders one pass of the order list at most.
    pub fn set_repeat_count(&mut self, count: i32) -> Result<()> {
        check_repeat_count(count)?;
        self.clear_error();
        // SAFETY: the handle is live. The count is a plain value.
        let ok = unsafe { openmpt_ext_sys::openmpt_module_set_repeat_count(self.core(), count) };
        self.checked("set repeat count", &format!("count {count}"), ok)
    }

    /// Returns the current order.
    #[must_use]
    pub fn current_order(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_current_order(self.core()) }
    }

    /// Returns the current row.
    #[must_use]
    pub fn current_row(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_current_row(self.core()) }
    }

    /// Returns the current pattern.
    ///
    /// Returns `-1` when the current order plays no pattern. The library gives
    /// `-1` for an out-of-range order and for an order entry that holds no
    /// pattern data, so `-1` is a data value here, not an error.
    #[must_use]
    pub fn current_pattern(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_current_pattern(self.core()) }
    }

    /// Returns the current speed.
    #[must_use]
    pub fn current_speed(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_current_speed(self.core()) }
    }

    /// Returns the current tempo, in ticks per minute.
    #[must_use]
    pub fn current_tempo(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_current_tempo(self.core()) }
    }

    /// Returns the duration of the selected subsong, in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error when libopenmpt rejects the read.
    pub fn duration_seconds(&mut self) -> Result<f64> {
        self.clear_error();
        // SAFETY: the handle is live. The call only reads a counter.
        let seconds = unsafe { openmpt_ext_sys::openmpt_module_get_duration_seconds(self.core()) };
        self.check_last_error("duration", "selected subsong")?;
        Ok(seconds)
    }

    /// Returns the playback position, in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error when libopenmpt rejects the read.
    pub fn position_seconds(&mut self) -> Result<f64> {
        self.clear_error();
        // SAFETY: the handle is live. The call only reads a counter.
        let seconds = unsafe { openmpt_ext_sys::openmpt_module_get_position_seconds(self.core()) };
        self.check_last_error("position", "selected subsong")?;
        Ok(seconds)
    }

    /// Returns the order-list entry at one position.
    ///
    /// A skip ("+++") and a stop ("---") entry are order-list markers, not
    /// pattern indices. The library gives their raw sentinel values (`65534` and
    /// `65535`), so this crate reports them as their own variants.
    ///
    /// # Errors
    ///
    /// Returns an error when `order` is outside `0..num_orders`.
    pub fn order_entry(&self, order: i32) -> Result<OrderEntry> {
        check_index("order entry", "order", order, self.num_orders())?;
        // SAFETY: the handle is live, and the order is in range.
        let (raw, skip, stop) = unsafe {
            (
                openmpt_ext_sys::openmpt_module_get_order_pattern(self.core(), order),
                openmpt_ext_sys::openmpt_module_is_order_skip_entry(self.core(), order) != 0,
                openmpt_ext_sys::openmpt_module_is_order_stop_entry(self.core(), order) != 0,
            )
        };
        Ok(OrderEntry::new(raw, skip, stop))
    }

    /// Returns the number of rows in one pattern.
    ///
    /// # Errors
    ///
    /// Returns an error when `pattern` is outside `0..num_patterns`, and when
    /// the index is inside that range but holds no pattern data. A module can
    /// leave a gap, because the count of patterns is the highest valid index
    /// plus one.
    pub fn pattern_num_rows(&self, pattern: i32) -> Result<i32> {
        check_index("pattern rows", "pattern", pattern, self.num_patterns())?;
        // SAFETY: the handle is live, and the pattern is in range.
        let rows =
            unsafe { openmpt_ext_sys::openmpt_module_get_pattern_num_rows(self.core(), pattern) };
        if rows == 0 {
            return Err(OpenmptError::new(
                "pattern rows",
                format!("pattern {pattern}"),
                "this module holds no data for this pattern",
                Some("the order list may refer to a pattern index that the module does not fill"),
            ));
        }
        Ok(rows)
    }

    /// Returns one field of one pattern cell.
    ///
    /// # Errors
    ///
    /// Returns an error when `pattern` is outside `0..num_patterns`, when `row`
    /// is outside `0..pattern_num_rows`, or when `channel` is outside
    /// `0..num_channels`.
    pub fn pattern_cell(
        &self,
        pattern: i32,
        row: i32,
        channel: i32,
        command: Command,
    ) -> Result<u8> {
        let rows = self.pattern_num_rows(pattern)?;
        check_index("pattern cell", "row", row, rows)?;
        check_index("pattern cell", "channel", channel, self.num_channels())?;
        // SAFETY: the handle is live, and all three indexes are in range.
        let value = unsafe {
            openmpt_ext_sys::openmpt_module_get_pattern_row_channel_command(
                self.core(),
                pattern,
                row,
                channel,
                command.id(),
            )
        };
        Ok(value)
    }

    /// Turns a return code of zero into an error.
    fn checked(&mut self, op: &'static str, input: &str, ok: c_int) -> Result<()> {
        if ok != 0 {
            return Ok(());
        }
        self.check_last_error(op, input)
    }
}

/// Rejects a repeat count that this crate does not accept.
///
/// The library reads any negative count as "repeat forever". A render loop that
/// waits for the song end never stops with that value.
///
/// # Errors
///
/// Returns an error that names the count and the valid range.
fn check_repeat_count(count: i32) -> Result<()> {
    if count >= 0 {
        return Ok(());
    }
    Err(OpenmptError::new(
        "set repeat count",
        format!("count {count}"),
        "this crate renders one pass of the order list at most",
        Some("use a count of 0, or a positive number of repeats"),
    ))
}

/// Rejects a subsong index that this crate does not accept.
///
/// A value of `0..count` selects one subsong. Every other value is an error,
/// including `-1`, which the library reads as "all subsongs".
fn check_subsong(count: i32, index: i32) -> Result<()> {
    if index == -1 {
        return Err(OpenmptError::new(
            "select subsong",
            format!("subsong {index}"),
            "this crate renders one subsong at a time",
            Some("use an index in 0..num_subsongs, and render each subsong on its own"),
        ));
    }
    check_index("select subsong", "subsong", index, count)
}
