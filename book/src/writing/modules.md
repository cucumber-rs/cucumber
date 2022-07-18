Modules organization
====================

When the project is started it's okay to have all the [step]s defined in a single `.feature` file. However, as the project grows, it will be more efficient to split all the [step]s into meaningful groups in different `.feature` files. This will make the project tests more logically organized and easier to maintain.




## Grouping

Technically, it doesn't matter how `.feature` files are named, or which [scenario]s are put in there. However, as the project grows, big `.feature` files becomes messy and hard to maintain. Instead, we recommend creating a separate `.rs` file for each domain concept (in a way that is meaningful to _your_ project).

Following this pattern allows us also to avoid the [feature-coupled step definitions][1] anti-pattern.




## Avoiding duplication

It's better to avoid writing similar [step] matching functions, as they can lead to clutter. While documenting [step]s helps, making use of [regular and Cucumber expressions][2] can do wonders.




## Managing growth

As the test suit grows, it may become harder to notice how minimal changes to regular expressions can lead to mismatched [step]s. 

> __TIP__: We recommend using [`Cucumber::fail_on_skipped()`] method in combination with `@allow.skipped` [tag]. The latter allows marking the [scenario]s which [step]s are explicitly allowed to be skipped.




[`Cucumber::fail_on_skipped()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.fail_on_skipped
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
[tag]: https://cucumber.io/docs/cucumber/api#tags

[1]: https://cucumber.io/docs/guides/anti-patterns/#feature-coupled-step-definitions
[2]: capturing.md
