//! Event handling implementation for Basic writer.

use std::{fmt::Debug, io};

use crate::{
    Event, World, Writer, event, parser,
    writer::{self, out::WriteStrExt as _},
};

use super::{basic_struct::Basic, cli::Cli};

impl<W, Out> Writer<W> for Basic<Out>
where
    W: World + Debug,
    Out: io::Write,
{
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        use event::{Cucumber, Feature};

        self.apply_cli(*cli);

        match event.map(Event::into_inner) {
            Err(err) => self.parsing_failed(&err),
            Ok(
                Cucumber::Started
                | Cucumber::ParsingFinished { .. }
                | Cucumber::Finished,
            ) => Ok(()),
            Ok(Cucumber::Feature(f, ev)) => match ev {
                Feature::Started => self.feature_started(&f),
                Feature::Scenario(sc, ev) => self.scenario(&f, &sc, &ev),
                Feature::Rule(r, ev) => self.rule(&f, &r, ev),
                Feature::Finished => Ok(()),
            },
        }
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to write to terminal: {e}");
        });
    }
}

impl<W, Val, Out> writer::Arbitrary<W, Val> for Basic<Out>
where
    W: World + Debug,
    Val: AsRef<str>,
    Out: io::Write,
{
    async fn write(&mut self, val: Val) {
        if let Err(e) = self.write_line(val.as_ref()) {
            eprintln!("Warning: Failed to write output: {e}");
        }
    }
}

impl<O: io::Write> writer::NonTransforming for Basic<O> {}

