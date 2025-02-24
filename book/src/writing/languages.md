Spoken languages
================

The language chosen for [Gherkin] should be the same language users and domain experts use when they talk about the domain. Translating between two languages should be avoided.

This is why [Gherkin] has been translated to over [70 languages][1].

A `# language:` header on the first line of a `.feature` file tells [Cucumber] which spoken language to use (for example, `# language: fr` for French). If you omit this header, [Cucumber] will default to English (`en`).

```gherkin
# language: no

Egenskap: Dyr egenskap

  Scenario: Hvis vi mater en sulten katt, vil den ikke lenger være sulten
    Gitt en sulten katt
    Når jeg mater katten
    Så katten er ikke sulten
```
```rust
# extern crate cucumber;
# extern crate tokio;
#
# use cucumber::{World, given, then, when};
#
# #[derive(Debug, Default)]
# struct Cat {
#     pub hungry: bool,
# }
#
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, Default, World)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
#
#[given(regex = r"^en (sulten|mett) katt$")]
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "sulten" => world.cat.hungry = true,
        "mett" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("jeg mater katten")]
async fn feed_cat(world: &mut AnimalWorld) {
    world.cat.feed();
}

#[then("katten er ikke sulten")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("tests/features/book/writing/languages.feature").await;
# }
```
![record](../rec/writing_languages.gif)

> __TIP__: In case most of your `.feature` files aren't written in English and you want to avoid endless `# language:` comments, use [`Cucumber::language()`] method to override the default language globally.




[`Cucumber::language()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.language
[Cucumber]: https://cucumber.io
[Gherkin]: https://cucumber.io/docs/gherkin/reference

[1]: https://cucumber.io/docs/gherkin/languages
