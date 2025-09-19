//! OutputFormatter trait implementation for Basic writer.

use std::io;

use super::basic_struct::Basic;

impl<Out: io::Write> crate::writer::common::OutputFormatter for Basic<Out> {
    type Output = Out;

    fn output_mut(&mut self) -> &mut Self::Output {
        &mut self.output
    }
}

