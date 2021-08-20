use std::{convert::Infallible, fmt::Debug};

use async_trait::async_trait;
use cucumber_rust::{
    self as cucumber, event, given, then, when, WorldInit, Writer,
};

#[derive(Default)]
struct DebugWriter(String);

#[derive(Debug, Default, WorldInit)]
struct World(usize);

#[given(regex = r"foo is (\d+)")]
#[when(regex = r"foo is (\d+)")]
#[then(regex = r"foo is (\d+)")]
fn step(w: &mut World, num: usize) {
    assert_eq!(w.0, num);
    w.0 += 1;
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World::default())
    }
}

#[async_trait(?Send)]
impl<World: 'static + Debug> Writer<World> for DebugWriter {
    async fn handle_event(&mut self, ev: event::Cucumber<World>) {
        self.0.push_str(&format!("{:?}", ev));
    }
}

#[cfg(test)]
mod spec {
    use std::fs;

    use cucumber_rust::{WorldInit as _, WriterExt as _};
    use globwalk::GlobWalkerBuilder;

    use super::{DebugWriter, World};

    #[tokio::test]
    async fn test() {
        let walker =
            GlobWalkerBuilder::new("tests/features/output", "*.feature")
                .case_insensitive(true)
                .build()
                .unwrap();
        let files = walker
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_str().unwrap().to_owned())
            .collect::<Vec<String>>();

        for file in files {
            let out = fs::read_to_string(format!(
                "tests/features/output/{}.out",
                file
            ))
            .unwrap_or_default()
            .replace("\n", "");
            let normalized = World::cucumber()
                .with_writer(DebugWriter::default().normalized())
                .run(format!("tests/features/output/{}", file))
                .await;

            assert_eq!(normalized.writer.0, out, "file: {}", file);
        }
    }
}
