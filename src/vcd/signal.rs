// Copyright (C) 2022 Yehowshua Immanuel
// This program is distributed under both the GPLV3 license
// and the YEHOWSHUA license, both of which can be found at
// the root of the folder containing the sources for this program.
use super::types;
use super::types::SignalIdx;
use num::BigUint;

// Index to the least significant byte of a timestamp
// value on the timeline
#[derive(Debug, Copy, Clone)]
pub struct LsbIdxOfTmstmpValOnTmln(pub(super) u32);

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SignalType {
    Event,
    Integer,
    Parameter,
    Real,
    RealTime,
    Reg,
    Str,
    Supply0,
    Supply1,
    Time,
    Tri,
    TriAnd,
    TriOr,
    TriReg,
    Tri0,
    Tri1,
    WAnd,
    Wire,
    WOr,
}

#[derive(Debug, PartialEq)]
pub enum SignalValue {
    BigUint(BigUint),
    String(String),
}

pub struct Signal<'a>(pub(super) &'a SignalEnum);

impl<'a> Signal<'a> {
    pub fn name(&self) -> String {
        let Signal(signal_enum) = &self;
        signal_enum.name()
    }

    pub fn name_with_index(&self) -> String {
        let Signal(signal_enum) = &self;
        signal_enum.name_with_index()
    }

    pub fn index(&self) -> Option<String> {
        let Signal(signal_enum) = &self;
        signal_enum.index()
    }

    pub fn path(&self) -> &[String] {
        match self.0 {
            SignalEnum::Data { path, .. } => path,
            SignalEnum::Alias { path, .. } => path,
        }
    }

    pub fn signal_type(&self) -> Option<&SignalType> {
        let Signal(signal_enum) = &self;
        signal_enum.signal_type()
    }

    pub fn real_idx(&self) -> SignalIdx {
        match self.0 {
            SignalEnum::Data { self_idx, .. } => *self_idx,
            SignalEnum::Alias { signal_alias, .. } => *signal_alias,
        }
    }

    pub fn num_bits(&self) -> Option<u32> {
        let Signal(signal_enum) = &self;
        signal_enum.bits_required()
    }

    pub fn query_string_val_on_tmln(
        &self,
        desired_time: &BigUint,
        vcd: &types::VCD,
    ) -> Result<String, SignalErrors> {
        let Signal(signal_enum) = &self;
        signal_enum
            .query_string_val_on_tmln(desired_time, &vcd.tmstmps_encoded_as_u8s, &vcd.all_signals)
            .map(|(val, _, _)| val)
    }

    pub fn query_num_val_on_tmln(
        &self,
        desired_time: &BigUint,
        vcd: &types::VCD,
    ) -> Result<BigUint, SignalErrors> {
        let Signal(signal_enum) = &self;
        signal_enum
            .query_num_val_on_tmln(desired_time, &vcd.tmstmps_encoded_as_u8s, &vcd.all_signals)
            .map(|(val, _, _)| val)
    }

    pub fn query_val_on_tmln(
        &self,
        desired_time: &BigUint,
        vcd: &types::VCD,
    ) -> Result<(TimeStamp, SignalValue, Option<TimeStamp>), SignalErrors> {
        let Signal(signal_enum) = &self;
        let num_val = signal_enum.query_num_val_on_tmln(
            desired_time,
            &vcd.tmstmps_encoded_as_u8s,
            &vcd.all_signals,
        );
        let str_val = signal_enum.query_string_val_on_tmln(
            desired_time,
            &vcd.tmstmps_encoded_as_u8s,
            &vcd.all_signals,
        );

        // Both num and str will return the newest value that is closest to
        // the desired time. If both have valid values, select the most recent
        // one
        match (num_val, str_val) {
            (Ok((num_val, num_time, num_next)), Ok((str_val, str_time, str_next))) => {
                let next = match (num_next, str_next) {
                    (Some(n), Some(s)) => Some(n.min(s)),
                    (Some(n), None) => Some(n),
                    (None, Some(s)) => Some(s),
                    (None, None) => None
                };
                if num_time > str_time {
                    Ok((num_time, SignalValue::BigUint(num_val), next))
                } else {
                    Ok((str_time, SignalValue::String(str_val), next))
                }
            }
            (Ok((num_val, time, next)), Err(_)) => Ok((time, SignalValue::BigUint(num_val), next)),
            (Err(_), Ok((str_val, time, next))) => Ok((time, SignalValue::String(str_val), next)),
            (Err(e), _e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub(super) enum SignalEnum {
    Data {
        name: String,
        path: Vec<String>,
        signal_type: SignalType,
        /// The optional [start:end] part of the signal name that is sometimes
        /// added to signals
        index: Option<String>,
        /// I've seen a 0 bit signal parameter in a xilinx
        /// simulation before that gets assigned 1 bit values.
        /// I consider this to be bad behavior. We capture such
        /// errors in the following type:
        signal_error: Option<String>,
        num_bits: Option<u32>,
        num_bytes: Option<u16>,
        /// TODO : may be able to remove self_idx
        self_idx: SignalIdx,
        /// A signal may take on a new value and hold that value
        /// for sometime. We only need to record the value of a signal
        /// when it changes(the is what VCDs tend to do).
        /// A signal may need x amount of bytes to record its largest
        /// possible value, so we record every single value of a given
        /// signal as a sequence of x number of u8s.
        /// For example, we might find that `my_signal.
        /// nums_encoded_as_fixed_width_le_u8`
        /// has two 32 bit values, namely, 1 and 2, encoded as follows:
        /// my_signal.nums_encoded_as_fixed_width_le_u8 = vec![1u8, 0u8,
        /// 0u8, 0u8, 2u8, 0u8, 0u8, 0u8];
        nums_encoded_as_fixed_width_le_u8: Vec<u8>,
        string_vals: Vec<String>,
        /// we could do Vec<(LsbIdxOfTmstmpValOnTmln, u8)>, but I
        /// suspect that Vec<LsbIdxOfTmstmpValOnTmln> is more cache
        /// friendly. We use ``LsbIdxOfTmstmpValOnTmln`` to index into
        /// the LSB of a particular timestamp encoded as the
        /// minimum length u8 sequence within
        /// ``vcd.tmstmps_encoded_as_u8s``, and we use the values in
        /// ``byte_len_of_num_tmstmp_vals_on_tmln`` to determine how
        /// many u8 values a particular timestamp is composed of.
        lsb_indxs_of_num_tmstmp_vals_on_tmln: Vec<LsbIdxOfTmstmpValOnTmln>,
        byte_len_of_num_tmstmp_vals_on_tmln: Vec<u8>,
        byte_len_of_string_tmstmp_vals_on_tmln: Vec<u8>,
        lsb_indxs_of_string_tmstmp_vals_on_tmln: Vec<LsbIdxOfTmstmpValOnTmln>,
    },
    Alias {
        name: String,
        path: Vec<String>,
        signal_alias: SignalIdx,
    },
}

#[derive(Debug)]
pub enum SignalErrors {
    PreTimeline {
        desired_time: BigUint,
        timeline_start_time: BigUint,
    },
    EmptyTimeline,
    TimelineNotMultiple,
    StrTmlnLenMismatch,
    OrderingFailure {
        lhs_time: BigUint,
        mid_time: BigUint,
        rhs_time: BigUint,
    },
    PointsToAlias,
    NoNumBytes,
    Other(String),
}

// these are thin type aliases primarily to make code more readable later on
type TimeStamp = BigUint;
type SignalValNum = BigUint;

// getter functions
impl SignalEnum {
    pub fn name(&self) -> String {
        match self {
            SignalEnum::Data { name, .. } => name,
            SignalEnum::Alias { name, .. } => name,
        }
        .clone()
    }

    pub fn signal_type(&self) -> Option<&SignalType> {
        match self {
            SignalEnum::Data { signal_type, .. } => Some(signal_type),
            // TODO: Follow aliases?
            SignalEnum::Alias { .. } => None,
        }
        .clone()
    }

    pub fn name_with_index(&self) -> String {
        match self {
            SignalEnum::Data {
                name, index: None, ..
            } => format!("{name}"),
            SignalEnum::Data {
                name,
                index: Some(size),
                ..
            } => format!("{name} {size}"),
            SignalEnum::Alias { name, .. } => name.clone(),
        }
    }

    pub fn index(&self) -> Option<String> {
        match self {
            SignalEnum::Data { index, .. } => index.clone(),
            SignalEnum::Alias { .. } => None,
        }
    }
}

// helper functions ultimately used by Signal's query functions later on
impl SignalEnum {
    /// Computes the bytes required to store a signal's numerical value
    /// using the num_bits which another function would provide from
    /// the num_bits field of the Signal::Data variant.
    pub(super) fn bytes_required(num_bits: u32, name: &String) -> Result<u16, String> {
        let bytes_required = (num_bits / 8) + if (num_bits % 8) > 0 { 1 } else { 0 };
        let bytes_required = u16::try_from(bytes_required).map_err(|_| {
            format!(
                "Error near {}:{}. Signal {name} of length num_bits requires \
                        {bytes_required} > 65536 bytes.",
                file!(),
                line!()
            )
        })?;
        Ok(bytes_required)
    }
    /// This function takes an event_idx which(is used to index into the
    /// global timeline field of a VCD struct instance) and computes
    /// the time pointed at by event_idx.
    /// This function also uses the same idx to index into the
    /// string_vals field of an instance of the Signal::Data variant
    ///  and gets a string value.
    /// The function returns a tuple of the timestamp and string value.
    fn time_and_str_val_at_event_idx(
        &self,
        event_idx: usize,
        tmstmps_encoded_as_u8s: &Vec<u8>,
    ) -> Result<(TimeStamp, &str), SignalErrors> {
        let (
            string_vals,
            lsb_indxs_of_string_tmstmp_vals_on_tmln,
            byte_len_of_string_tmstmp_vals_on_tmln,
        ) = match self {
            SignalEnum::Data {
                string_vals,
                lsb_indxs_of_string_tmstmp_vals_on_tmln,
                byte_len_of_string_tmstmp_vals_on_tmln,
                ..
            } => Ok((
                string_vals,
                lsb_indxs_of_string_tmstmp_vals_on_tmln,
                byte_len_of_string_tmstmp_vals_on_tmln,
            )),
            SignalEnum::Alias { .. } => Err(SignalErrors::PointsToAlias),
        }?;

        // get index
        let LsbIdxOfTmstmpValOnTmln(timestamp_idx) =
            lsb_indxs_of_string_tmstmp_vals_on_tmln[event_idx];
        let timestamp_idx = timestamp_idx as usize;

        if byte_len_of_string_tmstmp_vals_on_tmln.is_empty() {
            return Err(SignalErrors::EmptyTimeline);
        }

        // form timestamp
        let byte_len = byte_len_of_string_tmstmp_vals_on_tmln[event_idx] as usize;
        let timestamp = &tmstmps_encoded_as_u8s[timestamp_idx..(timestamp_idx + byte_len)];
        let timestamp = BigUint::from_bytes_le(timestamp);

        // get signal value
        let signal_val = string_vals[event_idx].as_str();

        Ok((timestamp, signal_val))
    }
    /// This function takes an event_idx which(is used to index into the
    /// global timeline field of a VCD struct instance) and computes
    /// the time pointed at by event_idx.
    /// This function also uses the same idx to index into the
    /// nums_encoded_as_fixed_width_le_u8 and
    /// byte_len_of_num_tmstmp_vals_on_tmln fields of an instance
    /// of the Signal::Data variant to compute the signal's corresponding
    /// numerical value at the time pointed at by event_didx.
    /// The function returns a tuple of the timestamp and numerical
    /// value.
    fn time_and_num_val_at_event_idx(
        &self,
        event_idx: usize,
        tmstmps_encoded_as_u8s: &Vec<u8>,
    ) -> Result<(TimeStamp, SignalValNum), SignalErrors> {
        let (
            num_bytes,
            nums_encoded_as_fixed_width_le_u8,
            lsb_indxs_of_num_tmstmp_vals_on_tmln,
            byte_len_of_num_tmstmp_vals_on_tmln,
        ) = match self {
            SignalEnum::Data {
                num_bytes,
                nums_encoded_as_fixed_width_le_u8,
                lsb_indxs_of_num_tmstmp_vals_on_tmln,
                byte_len_of_num_tmstmp_vals_on_tmln,
                ..
            } => Ok((
                num_bytes,
                nums_encoded_as_fixed_width_le_u8,
                lsb_indxs_of_num_tmstmp_vals_on_tmln,
                byte_len_of_num_tmstmp_vals_on_tmln,
            )),
            SignalEnum::Alias { .. } => Err(SignalErrors::PointsToAlias),
        }?;

        // get index
        let LsbIdxOfTmstmpValOnTmln(timestamp_idx) =
            lsb_indxs_of_num_tmstmp_vals_on_tmln[event_idx];
        let timestamp_idx = timestamp_idx as usize;

        // form timestamp
        let byte_len = byte_len_of_num_tmstmp_vals_on_tmln[event_idx] as usize;
        let timestamp = &tmstmps_encoded_as_u8s[timestamp_idx..(timestamp_idx + byte_len)];
        let timestamp = BigUint::from_bytes_le(timestamp);

        // get signal value
        let bytes_per_value = num_bytes.ok_or_else(|| SignalErrors::NoNumBytes)?;
        let bytes_per_value = bytes_per_value as usize;
        let start_idx = event_idx * bytes_per_value;
        let end_idx = (event_idx + 1) * bytes_per_value;
        let signal_val = &nums_encoded_as_fixed_width_le_u8[start_idx..end_idx];
        let signal_val = BigUint::from_bytes_le(signal_val);

        Ok((timestamp, signal_val))
    }

    fn bits_required(&self) -> Option<u32> {
        match self {
            SignalEnum::Data { num_bits, .. } => *num_bits,
            // TODO: Follow aliases?
            SignalEnum::Alias { .. } => None,
        }
    }
}

// Val and string query functions.
// Function that take in a desired time on the timeline for a
// specific signal and return a numerical or string value in a Result,
// or an error in a Result.
impl SignalEnum {
    pub fn query_string_val_on_tmln(
        &self,
        desired_time: &BigUint,
        tmstmps_encoded_as_u8s: &Vec<u8>,
        all_signals: &Vec<SignalEnum>,
    ) -> Result<(String, TimeStamp, Option<TimeStamp>), SignalErrors> {
        let signal_idx = match self {
            Self::Data { self_idx, .. } => {
                let SignalIdx(idx) = self_idx;
                *idx
            }
            Self::Alias {
                name: _,
                signal_alias,
                path: _,
            } => {
                let SignalIdx(idx) = signal_alias;
                *idx
            }
        };

        // if the signal idx points to data variant of the signal,
        // extract:
        // 1. the vector of string values
        // 2. the vector of indices into timeline where events occur
        //    for this signal
        // else we propagate Err(..).
        let (string_vals, lsb_indxs_of_string_tmstmp_vals_on_tmln) = match &all_signals[signal_idx]
        {
            SignalEnum::Data {
                ref string_vals,
                ref lsb_indxs_of_string_tmstmp_vals_on_tmln,
                ..
            } => Ok((string_vals, lsb_indxs_of_string_tmstmp_vals_on_tmln)),
            SignalEnum::Alias { .. } => Err(SignalErrors::PointsToAlias),
        }?;
        // this signal should at least have some events, otherwise, trying to index into
        // an empty vector later on would fail
        if lsb_indxs_of_string_tmstmp_vals_on_tmln.is_empty() {
            return Err(SignalErrors::EmptyTimeline);
        }

        // the vector of string timeline lsb indices should have the same
        // length as the vector of string values
        if string_vals.len() != lsb_indxs_of_string_tmstmp_vals_on_tmln.len() {
            return Err(SignalErrors::StrTmlnLenMismatch);
        }

        // check if we're requesting a value that occurs before the recorded
        // start of the timeline
        let (timeline_start_time, _) =
            self.time_and_str_val_at_event_idx(0, tmstmps_encoded_as_u8s)?;
        if *desired_time < timeline_start_time {
            return Err(SignalErrors::PreTimeline {
                desired_time: desired_time.clone(),
                timeline_start_time,
            });
        }

        let mut lower_idx = 0usize;
        let mut upper_idx = lsb_indxs_of_string_tmstmp_vals_on_tmln.len() - 1;
        let (timeline_end_time, timeline_end_val) =
            self.time_and_str_val_at_event_idx(upper_idx, tmstmps_encoded_as_u8s)?;

        // check if we're requesting a value that occurs beyond the end of the timeline,
        // if so, return the last value in this timeline
        if *desired_time > timeline_end_time {
            return Ok((timeline_end_val.to_string(), timeline_end_time, None));
        }

        // This while loop is the meat of the lookup. Performance is log2(n),
        // where n is the number of events on the timeline.
        // We can assume that by the time we get here, that the desired_time
        // is an event that occurs on the timeline, given that we handle any events
        // occuring after or before the recorded tiimeline in the code above.
        while lower_idx <= upper_idx {
            let mid_idx = lower_idx + ((upper_idx - lower_idx) / 2);
            let (curr_time, curr_val) =
                self.time_and_str_val_at_event_idx(mid_idx, tmstmps_encoded_as_u8s)?;
            let ordering = curr_time.cmp(desired_time);

            match ordering {
                std::cmp::Ordering::Less => {
                    lower_idx = mid_idx + 1;
                }
                std::cmp::Ordering::Equal => {
                    let next_time = if mid_idx < lsb_indxs_of_string_tmstmp_vals_on_tmln.len()-1 {
                        Some(self.time_and_str_val_at_event_idx(mid_idx+1, tmstmps_encoded_as_u8s)?.0)
                    }
                    else {
                        None
                    };
                    return Ok((curr_val.to_string(), curr_time, next_time));
                }
                std::cmp::Ordering::Greater => {
                    upper_idx = mid_idx - 1;
                }
            }
        }

        let (left_time, left_val) =
            self.time_and_str_val_at_event_idx(lower_idx - 1, tmstmps_encoded_as_u8s)?;
        let (right_time, _) =
            self.time_and_str_val_at_event_idx(lower_idx, tmstmps_encoded_as_u8s)?;

        let ordered_left = left_time < *desired_time;
        let ordered_right = *desired_time < right_time;
        if !(ordered_left && ordered_right) {
            return Err(SignalErrors::OrderingFailure {
                lhs_time: left_time,
                mid_time: desired_time.clone(),
                rhs_time: right_time,
            });
        }

        Ok((left_val.to_string(), left_time, Some(right_time)))
    }
    pub fn query_num_val_on_tmln(
        &self,
        desired_time: &BigUint,
        tmstmps_encoded_as_u8s: &Vec<u8>,
        all_signals: &Vec<SignalEnum>,
    ) -> Result<(BigUint, TimeStamp, Option<TimeStamp>), SignalErrors> {
        let signal_idx = match self {
            Self::Data { self_idx, .. } => {
                let SignalIdx(idx) = self_idx;
                *idx
            }
            Self::Alias {
                name: _,
                path: _,
                signal_alias,
            } => {
                let SignalIdx(idx) = signal_alias;
                *idx
            }
        };

        // if the signal idx points to data variant of the signal,
        // extract:
        // 1. the vector of LE u8 compressed values
        // 2. the vector of indices into timeline where events occur
        //    for this signal
        // 3. the number of bytes per value for this signal
        // else we propagate Err(..).
        let (nums_encoded_as_fixed_width_le_u8, lsb_indxs_of_num_tmstmp_vals_on_tmln, num_bytes) =
            match &all_signals[signal_idx] {
                SignalEnum::Data {
                    num_bytes,
                    ref nums_encoded_as_fixed_width_le_u8,
                    ref lsb_indxs_of_num_tmstmp_vals_on_tmln,
                    ..
                } => {
                    if num_bytes.is_none() {
                        return Err(SignalErrors::NoNumBytes);
                    }
                    Ok((
                        nums_encoded_as_fixed_width_le_u8,
                        lsb_indxs_of_num_tmstmp_vals_on_tmln,
                        num_bytes,
                    ))
                }
                SignalEnum::Alias { .. } => Err(SignalErrors::PointsToAlias),
            }?;
        // this signal should at least have some events, otherwise, trying to index into
        // an empty vector later on would fail
        if lsb_indxs_of_num_tmstmp_vals_on_tmln.is_empty() {
            return Err(SignalErrors::EmptyTimeline);
        }

        // assertion that value_sequence is a proper multiple of
        // timeline_markers
        let bytes_required = num_bytes.ok_or_else(|| {
            SignalErrors::Other(format!(
                "Error near {}:{}. num_bytes empty.",
                file!(),
                line!()
            ))
        })?;
        if nums_encoded_as_fixed_width_le_u8.len()
            != (lsb_indxs_of_num_tmstmp_vals_on_tmln.len() * (bytes_required as usize))
        {
            return Err(SignalErrors::TimelineNotMultiple);
        }

        // check if we're requesting a value that occurs before the recorded
        // start of the timeline
        let (timeline_start_time, _) =
            self.time_and_num_val_at_event_idx(0, tmstmps_encoded_as_u8s)?;
        if *desired_time < timeline_start_time {
            return Err(SignalErrors::PreTimeline {
                desired_time: desired_time.clone(),
                timeline_start_time,
            });
        }

        let mut lower_idx = 0usize;
        let mut upper_idx = lsb_indxs_of_num_tmstmp_vals_on_tmln.len() - 1;
        let (timeline_end_time, timeline_end_val) =
            self.time_and_num_val_at_event_idx(upper_idx, tmstmps_encoded_as_u8s)?;

        // check if we're requesting a value that occurs beyond the end of the timeline,
        // if so, return the last value in this timeline
        if *desired_time > timeline_end_time {
            return Ok((timeline_end_val, timeline_end_time, None));
        }

        // This while loop is the meat of the lookup. Performance is log2(n),
        // where n is the number of events on the timeline.
        // We can assume that by the time we get here, that the desired_time
        // is an event that occurs on the timeline, given that we handle any events
        // occuring after or before the recorded tiimeline in the code above.
        while lower_idx <= upper_idx {
            let mid_idx = lower_idx + ((upper_idx - lower_idx) / 2);
            let (curr_time, curr_val) =
                self.time_and_num_val_at_event_idx(mid_idx, tmstmps_encoded_as_u8s)?;
            let ordering = curr_time.cmp(desired_time);

            match ordering {
                std::cmp::Ordering::Less => {
                    lower_idx = mid_idx + 1;
                }
                std::cmp::Ordering::Equal => {
                    let next_time = if mid_idx < lsb_indxs_of_num_tmstmp_vals_on_tmln.len() - 1 {
                        Some(self.time_and_num_val_at_event_idx(mid_idx+1, tmstmps_encoded_as_u8s)?.0)
                    }
                    else {
                        None
                    };
                    return Ok((curr_val, curr_time, next_time));
                }
                std::cmp::Ordering::Greater => {
                    upper_idx = mid_idx - 1;
                }
            }
        }

        let (left_time, left_val) =
            self.time_and_num_val_at_event_idx(lower_idx - 1, tmstmps_encoded_as_u8s)?;
        let (right_time, _) =
            self.time_and_num_val_at_event_idx(lower_idx, tmstmps_encoded_as_u8s)?;

        let ordered_left = left_time < *desired_time;
        let ordered_right = *desired_time < right_time;
        if !(ordered_left && ordered_right) {
            return Err(SignalErrors::OrderingFailure {
                lhs_time: left_time,
                mid_time: desired_time.clone(),
                rhs_time: right_time,
            });
        }

        Ok((left_val, left_time, Some(right_time)))
    }
}
