TODO

## Spoken languages

The language you choose for [Gherkin] should be the same language your users and domain experts use when they talk about the domain. Translating between two languages should be avoided.

This is why [Gherkin] has been translated to over [70 languages](https://cucumber.io/docs/gherkin/languages).

A `# language:` header on the first line of a `.feature` file tells [Cucumber] which spoken language to use (for example, `# language: fr` for French). If you omit this header, [Cucumber] will default to English (`en`).

```gherkin
# language: no
    
Egenskap: Animal feature
    
  Eksempel: If we feed a hungry cat it will no longer be hungry
    Gitt a hungry cat
    Når I feed the cat
    Så the cat is not hungry
```

<script id="asciicast-DFtCqnpcnXpKbGxtxfedkW0Ga" src="https://asciinema.org/a/DFtCqnpcnXpKbGxtxfedkW0Ga.js" async data-autoplay="true" data-rows="18"></script>

In case most of your `.feature` files aren't written in English and you want to avoid endless `# language:` comments, use [`Cucumber::language()`](https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.language) method to override the default language.



