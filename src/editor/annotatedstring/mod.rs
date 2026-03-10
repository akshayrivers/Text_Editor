use super::ByteIdx;
use super::{Annotation, AnnotationType};
use std::{
    cmp::{max, min},
    fmt::{self, Display},
};
mod annotatedstringpart;
use annotatedstringpart::AnnotatedStringPart;
mod annotatedstringiterator;
use annotatedstringiterator::AnnotatedStringIterator;

#[derive(Default, Debug)]
pub struct AnnotatedString {
    string: String,
    annotations: Vec<Annotation>,
}

impl AnnotatedString {
    pub fn from(string: &str) -> Self {
        Self {
            string: String::from(string),
            annotations: Vec::new(),
        }
    }
    pub fn add_annotation(
        &mut self,
        annotation_type: AnnotationType,
        start_byte_idx: ByteIdx,
        end_byte_idx: ByteIdx,
    ) {
        debug_assert!(start_byte_idx <= end_byte_idx);
        self.annotations.push(Annotation {
            annotation_type,
            start_byte_idx,
            end_byte_idx,
        });
    }
    pub fn truncate_left_until(&mut self, until: ByteIdx) {
        self.replace(0, until, "");
    }
    pub fn truncate_right_from(&mut self, from: ByteIdx) {
        self.replace(from, self.string.len(), "");
    }

    pub fn replace(&mut self, start_byte_idx: ByteIdx, end_byte_idx: ByteIdx, new_string: &str) {
        debug_assert!(start_byte_idx <= end_byte_idx);
        debug_assert!(start_byte_idx < self.string.len());
        let end_byte_idx = min(end_byte_idx, self.string.len());
        if start_byte_idx > end_byte_idx {
            return;
        }
        self.string
            .replace_range(start_byte_idx..end_byte_idx, new_string);

        let replaced_range_len = end_byte_idx.saturating_sub(start_byte_idx); //this is the range we want to replace
        let shortend = new_string.len() < replaced_range_len;
        let len_difference = new_string.len().abs_diff(replaced_range_len); // this is how much longer or shorter the new range is.
        if len_difference == 0 {
            //no adjustment of annotations needed in case the replacement didn't result in a change in length.
            return;
        }

        self.annotations.iter_mut().for_each(|annotation| {
            annotation.start_byte_idx = if annotation.start_byte_idx >= end_byte_idx {
                // for annotations starting after the replaced range, we move from the start index by the diff in length
                if shortend {
                    annotation.start_byte_idx.saturating_sub(len_difference)
                } else {
                    annotation.start_byte_idx.saturating_add(len_difference)
                }
            } else if annotation.start_byte_idx >= start_byte_idx {
                // For annotations starting within the replaced range, we move from the start index by the diff in len, constrained to the beginning or end of the replaced range.
                if shortend {
                    max(
                        start_byte_idx,
                        annotation.start_byte_idx.saturating_sub(len_difference),
                    )
                } else {
                    min(
                        end_byte_idx,
                        annotation.start_byte_idx.saturating_add(len_difference),
                    )
                }
            } else {
                annotation.start_byte_idx
            };
            annotation.end_byte_idx = if annotation.end_byte_idx >= end_byte_idx {
                // for annotations ending after the replaced range, we move the end by the differnce in length.
                if shortend {
                    annotation.end_byte_idx.saturating_sub(len_difference)
                } else {
                    annotation.end_byte_idx.saturating_add(len_difference)
                }
            } else if annotation.end_byte_idx > start_byte_idx {
                // For annotations ending within the replaced range, we move the end index by the difference in length, constrained to the beginning or end of the replaced range.
                if shortend {
                    max(
                        start_byte_idx,
                        annotation.end_byte_idx.saturating_sub(len_difference),
                    )
                } else {
                    min(
                        end_byte_idx,
                        annotation.end_byte_idx.saturating_add(len_difference),
                    )
                }
            } else {
                annotation.end_byte_idx
            }
        });

        //Filter out empty annotations, in case the previous step resulted in any.
        self.annotations.retain(|annotation| {
            annotation.start_byte_idx < annotation.end_byte_idx
                && annotation.start_byte_idx < self.string.len()
        });
    }
}
impl Display for AnnotatedString {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.string)
    }
}

impl<'a> IntoIterator for &'a AnnotatedString {
    type Item = AnnotatedStringPart<'a>;
    type IntoIter = AnnotatedStringIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AnnotatedStringIterator {
            annotated_string: self,
            current_idx: 0,
        }
    }
}
