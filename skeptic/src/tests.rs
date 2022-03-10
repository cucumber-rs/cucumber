use unindent::unindent;

use super::*;

#[test]
fn test_omitted_lines() {
    let lines = unindent(
        r###"
        # use std::collections::BTreeMap as Map;
        #
        #[allow(dead_code)]
        fn main() {
            let map = Map::new();
            #
            # let _ = map;
        }"###,
    );

    let expected = unindent(
        r###"
        use std::collections::BTreeMap as Map;

        #[allow(dead_code)]
        fn main() {
            let map = Map::new();

        let _ = map;
        }
        "###,
    );

    assert_eq!(create_test_input(&get_lines(lines)), expected);
}

#[test]
fn test_markdown_files_of_directory() {
    let files = vec![
        "../testing/tests/hashtag-test.md",
        "../testing/tests/section-names.md",
        "../testing/tests/should-panic-test.md",
    ];
    let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    assert_eq!(markdown_files_of_directory("../testing/tests/"), files);
}

#[test]
fn test_sanitization_of_testnames() {
    assert_eq!(sanitize_test_name("My_Fun"), "my_fun");
    assert_eq!(sanitize_test_name("__my_fun_"), "my_fun");
    assert_eq!(sanitize_test_name("^$@__my@#_fun#$@"), "my_fun");
    assert_eq!(
        sanitize_test_name("my_long__fun___name___with____a_____lot______of_______spaces",),
        "my_long_fun_name_with_a_lot_of_spaces"
    );
    assert_eq!(sanitize_test_name("Löwe 老虎 Léopard"), "l_we_l_opard");
}

#[test]
fn line_numbers_displayed_are_for_the_beginning_of_each_code_block() {
    let lines = unindent(
        r###"
        Rust code that should panic when running it.

        ```rust,should_panic",/
        fn main() {
            panic!(\"I should panic\");
        }
        ```

        Rust code that should panic when compiling it.

        ```rust,no_run,should_panic",//
        fn add(a: u32, b: u32) -> u32 {
            a + b
        }

        fn main() {
            add(1);
        }
        ```"###,
    );

    let tests =
        extract_tests_from_string(&create_test_input(&get_lines(lines)), &String::from("blah"));

    let test_names: Vec<String> = tests
        .0
        .into_iter()
        .map(get_line_number_from_test_name)
        .collect();

    assert_eq!(test_names, vec!["3", "11"]);
}

#[test]
fn line_numbers_displayed_are_for_the_beginning_of_each_section() {
    let lines = unindent(
        r###"
        ## Test Case  Names   With    weird     spacing       are        generated      without        error.

        ```rust", /
        struct Person<'a>(&'a str);
        fn main() {
          let _ = Person(\"bors\");
        }
        ```

        ## !@#$ Test Cases )(() with {}[] non alphanumeric characters ^$23 characters are \"`#`\" generated correctly @#$@#$  22.

        ```rust", //
        struct Person<'a>(&'a str);
        fn main() {
          let _ = Person(\"bors\");
        }
        ```

        ## Test cases with non ASCII ö_老虎_é characters are generated correctly.

        ```rust",//
        struct Person<'a>(&'a str);
        fn main() {
          let _ = Person(\"bors\");
        }
        ```"###,
    );

    let tests =
        extract_tests_from_string(&create_test_input(&get_lines(lines)), &String::from("blah"));

    let test_names: Vec<String> = tests
        .0
        .into_iter()
        .map(get_line_number_from_test_name)
        .collect();

    assert_eq!(test_names, vec!["3", "12", "21"]);
}

#[test]
fn old_template_is_returned_for_old_skeptic_template_format() {
    let lines = unindent(
        r###"
        ```rust,skeptic-template
        ```rust,ignore
        use std::path::PathBuf;

        fn main() {{
            {}
        }}
        ```
        ```
        "###,
    );
    let expected = unindent(
        r###"
        ```rust,ignore
        use std::path::PathBuf;

        fn main() {{
            {}
        }}
        "###,
    );
    let tests =
        extract_tests_from_string(&create_test_input(&get_lines(lines)), &String::from("blah"));
    assert_eq!(tests.1, Some(expected));
}

#[test]
fn old_template_is_not_returned_if_old_skeptic_template_is_not_specified() {
    let lines = unindent(
        r###"
        ```rust", /
        struct Person<'a>(&'a str);
        fn main() {
          let _ = Person(\"bors\");
        }
        ```
        "###,
    );
    let tests =
        extract_tests_from_string(&create_test_input(&get_lines(lines)), &String::from("blah"));
    assert_eq!(tests.1, None);
}

fn get_line_number_from_test_name(test: Test) -> String {
    String::from(
        test.name
            .split('_')
            .last()
            .expect("There were no underscores!"),
    )
}

fn get_lines(lines: String) -> Vec<String> {
    lines
        .split('\n')
        .map(|string_slice| format!("{}\n", string_slice)) //restore line endings since they are removed by split.
        .collect()
}
