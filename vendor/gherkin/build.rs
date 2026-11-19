use std::{collections::BTreeMap, path::Path};

use quote::{__private::Span, quote};
use syn::Ident;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    and: Vec<String>,
    background: Vec<String>,
    but: Vec<String>,
    examples: Vec<String>,
    feature: Vec<String>,
    given: Vec<String>,
    // name: String,
    // native: String,
    rule: Vec<String>,
    scenario: Vec<String>,
    scenario_outline: Vec<String>,
    then: Vec<String>,
    when: Vec<String>,
}

fn main() {
    println!("cargo:rerun-if-changed=src/languages.json");
    println!("cargo:rerun-if-changed=src/languages-43.json");
    generate("./src/languages.json", "", false);
    generate("./src/languages-43.json", "latest-", true);
}

fn generate(source: &str, prefix: &str, latest: bool) {
    use heck::ToShoutySnakeCase as _;

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let f = std::fs::read_to_string(source).unwrap();
    let langs: BTreeMap<String, Data> = serde_json::from_str(&f).unwrap();

    let mut keyword_defs = vec![];
    let mut match_arms = vec![];

    for (lang, mut data) in langs {
        if latest {
            // Official Gherkin's compareStepKeywords prefers the longest prefix.
            // Keep legacy ordering untouched for existing parser consumers.
            for words in [
                &mut data.feature,
                &mut data.background,
                &mut data.rule,
                &mut data.scenario,
                &mut data.scenario_outline,
                &mut data.examples,
                &mut data.given,
                &mut data.when,
                &mut data.then,
                &mut data.and,
                &mut data.but,
            ] {
                words.sort_by_key(|word| std::cmp::Reverse(word.chars().count()));
            }
        }
        let lang_upper = format!("{}{}", prefix, lang).to_shouty_snake_case();
        let lang_ident: Ident = Ident::new(&lang_upper, Span::call_site());

        let Data {
            and,
            background,
            but,
            examples,
            feature,
            given,
            // name,
            // native,
            rule,
            scenario,
            scenario_outline,
            then,
            when,
        } = data;

        let keyword_def = quote! {
            const #lang_ident: Keywords<'static> = Keywords {
                feature: &[#(#feature),*],
                background: &[#(#background),*],
                rule: &[#(#rule),*],
                scenario: &[#(#scenario),*],
                scenario_outline: &[#(#scenario_outline),*],
                examples: &[#(#examples),*],
                given: &[#(#given),*],
                when: &[#(#when),*],
                then: &[#(#then),*],
                and: &[#(#and),*],
                but: &[#(#but),*],
            };
        };

        let match_arm = quote! {
            #lang => Some(#lang_ident)
        };

        keyword_defs.push(keyword_def);
        match_arms.push(match_arm);
    }

    let keyword_defs = quote! {
        #(#keyword_defs)*
    }
    .to_string();

    let match_arms = quote! {
        match key {
            #(#match_arms),*,
            _ => None
        }
    }
    .to_string();

    std::fs::write(
        out_dir.join(format!("{}keywords.gen.rs", prefix)),
        keyword_defs,
    )
    .unwrap();
    std::fs::write(out_dir.join(format!("{}match.gen.rs", prefix)), match_arms).unwrap();
}
