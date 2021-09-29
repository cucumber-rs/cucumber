Test Modules Organization
=========================

You can have all of your step definitions in one file, or in multiple files. When you start with your project, all your step definitions will probably be in one file. As your project grows, you should split your step definitions into meaningful groups in different files. This will make your project more logically organized and easier to maintain.




## Grouping step definitions

Technically, it doesn't matter how you name your step definition files, or which step definitions you put in a file. You could have one giant file containing all your step definitions. However, as the project grows, the file becomes messy and hard to maintain. Instead, we recommend creating a separate `.rs` file for each domain concept.

If you follow this pattern, you also avoid the [Feature-coupled step definitions](https://cucumberio/docs/guides/anti-patterns/#feature-coupled-step-definitions) anti-pattern.

Of course, how you group your step definitions is really up to you and your team. They should be grouped in a way that is meaningful to _your_ project.




## Avoid duplication

Avoid writing similar step definitions, as they can lead to clutter. While documenting your steps helps, making use of [`regex` and `FromStr`](Features.md#combining-regex-and-fromstr) can do wonders.




## Managing growth

As your test suit grows, it may become harder to notice how minimal changes to `regex`es can lead to mismatched `Step`s. To avoid this, we recommend using [`Cucumber::fail_on_skipped()`](https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.fail_on_skipped) combining with `@allow_skipped` tag. This will allow you to mark out `Scenario`s which `Step`s are allowed to skip.

And, as time goes on, total run time of all tests can become overwhelming when you only want to test small subset of `Scenario`s. At least until you discover [`Cucumber::filter_run_and_exit()`](https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.filter_run_and_exit), which will allow you run only `Scenario`s marked with custom [tags](https://cucumber.io/docs/cucumber/api/#tags).
