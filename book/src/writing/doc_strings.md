Doc strings
===========

[Doc strings][doc] provide an ability to pass a large piece of text to a [step] definition (and so, to a [step] matching function).

The text should be offset by delimiters consisting of three double-quote marks `"""` on lines of their own:
```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
      """
      A hungry cat called Felix is rescued from a Whiskas tin in a calamitous 
      mash-up of cat food brands.
      """
    When I feed the cat
    Then the cat is not hungry
```

> __NOTE__: Indentation of the opening `"""` is unimportant, although the common practice is to indent them. The indentation inside the triple quotes, however, is significant. Each line of the [doc string][doc] will be dedented according to the opening `"""`. Indentation beyond the column of the opening `"""` will therefore be preserved.

[Doc strings][doc] also support using three backticks ` ``` ` as the delimiter, which might be familiar for those used to writing with [Markdown]:
```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry Leo it will no longer be hungry
    Given a hungry cat
      ```
      A hungry cat called Leo is rescued from a Whiskas tin in a calamitous
      mash-up of cat food brands.
      ```
    When I feed the cat
    Then the cat is not hungry
```

It’s also possible to annotate the [doc string][doc] with the type of content it contains, as follows:
```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry Simba it will no longer be hungry
    Given a hungry cat
      """markdown
      About Simba
      ===========
      A hungry cat called Simba is rescued from a Whiskas tin in a calamitous
      mash-up of cat food brands.
      """
    When I feed the cat
    Then the cat is not hungry
```

> __NOTE__: Whilst [`cucumber`] and [`gherkin`] crates support content types and backticks as the delimiter, many tools like text editors don’t (yet).

In a [step] matching function, there’s no need to find this text and match it with a pattern. Instead, it may be accessed via [`Step`] argument:
```rust,should_panic
# use std::convert::Infallible;
#
# use async_trait::async_trait;
# use cucumber::{gherkin::Step, given, then, when, World, WorldInit};
#
# #[derive(Debug)]
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
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
#
#[given(regex = r"^a (hungry|satiated) cat$")]
async fn hungry_cat(world: &mut AnimalWorld, step: &Step, state: String) {
    // Feed only Leo and Felix.
    if !step
        .docstring
        .as_ref()
        .map_or(false, |text| text.contains("Felix") || text.contains("Leo"))
    {
        panic!("Only Felix and Leo can be fed");
    }

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(!world.cat.hungry);
# }
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/doc_strings.feature").await;
# }
```
![record](../rec/writing_doc_strings.gif)




[`cucumber`]: https://docs.rs/cucumber
[`gherkin`]: https://docs.rs/gherkin 
[`Step`]: https://docs.rs/gherkin/*/gherkin/struct.Step.html
[doc]: https://cucumber.io/docs/gherkin/reference#doc-strings
[Markdown]: https://en.wikipedia.org/wiki/Markdown
[step]: https://cucumber.io/docs/gherkin/reference#steps
