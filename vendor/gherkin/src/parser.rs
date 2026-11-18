// Copyright (c) 2020-2026  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{cell::RefCell, iter};

use crate::{keywords::Keywords, tagexpr::TagOperation};
use crate::{Background, Examples, Feature, LineCol, Rule, Scenario, Span, Step, StepType, Table};

/// Classic token whitespace and value-trimming compatibility policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LexicalSemantics {
    /// Preserve ASCII padding, Rust value trimming, and native byte columns.
    #[default]
    Legacy,
    /// ECMAScript whitespace/trimming and scalar token columns, as in Gherkin43.
    Gherkin43,
}
impl LexicalSemantics {
    /// Whether a character is whitespace under this policy's value/tag rules.
    pub fn is_whitespace(self, character: char) -> bool {
        match self { Self::Legacy => character.is_whitespace(), Self::Gherkin43 => official_docstring_whitespace(character) }
    }
    /// Whether a character is token padding on one source line (never LF).
    pub fn is_padding(self, character: char) -> bool {
        match self { Self::Legacy => matches!(character, ' ' | '\t'), Self::Gherkin43 => character != '\n' && self.is_whitespace(character) }
    }
}

/// Selects doc-string content semantics independently from dialect vocabulary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DocStringSemantics {
    /// Preserve the original native representation, including delimiter-adjacent
    /// newlines and any opening media-type text.
    #[default]
    Legacy,
    /// Match the adopted official Gherkin 43 line-oriented content contract.
    /// Media-type text is excluded from content; `Step` keeps its existing shape.
    Gherkin43,
}

/// Selects table row sizing independently from dialect and doc-string behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TableSemantics {
    /// Preserve legacy truncation of later rows longer than the first row.
    #[default]
    Legacy,
    /// Reject any row whose cell count differs from the first row.
    Gherkin43,
}

#[derive(Debug)]
pub struct GherkinEnv {
    keywords: RefCell<Keywords<'static>>,
    pub(crate) last_error: RefCell<Option<EnvError>>,
    pub(crate) fatal_error: RefCell<Option<EnvError>>,
    last_step: RefCell<Option<StepType>>,
    last_keyword: RefCell<Option<String>>,
    line_offsets: RefCell<Vec<usize>>,
    was_escaped: RefCell<bool>,
    latest_catalogue: bool,
    docstring_semantics: DocStringSemantics,
    table_semantics: TableSemantics,
    lexical_semantics: LexicalSemantics,
}

#[derive(Debug, thiserror::Error)]
pub enum EnvError {
    #[error("Requested language '{0}' is not supported.")]
    UnsupportedLanguage(String),

    #[error("Unknown keyword: '{0}'.")]
    UnknownKeyword(String),

    #[error("Inconsistent cell count")]
    InconsistentCellCount(Vec<Vec<String>>),
}

impl GherkinEnv {
    pub fn new(language: &str) -> Result<Self, EnvError> {
        let keywords = Keywords::get(language)
            .ok_or_else(|| EnvError::UnsupportedLanguage(language.into()))?;

        Ok(Self {
            keywords: RefCell::new(keywords),
            ..Default::default()
        })
    }

    /// Uses the official Gherkin 43.0.0 dialect catalogue explicitly.
    ///
    /// Unlike `new` and `Default`, this accepts new official dialects and omits
    /// aliases removed upstream. Language headers preserve this catalogue choice.
    pub fn new_latest(language: &str) -> Result<Self, EnvError> {
        let keywords = Keywords::get_latest(language)
            .ok_or_else(|| EnvError::UnsupportedLanguage(language.into()))?;
        Ok(Self {
            keywords: RefCell::new(keywords),
            latest_catalogue: true,
            ..Default::default()
        })
    }

    /// Selects doc-string content semantics without changing dialect selection.
    /// All constructors retain `Legacy` unless this builder is used explicitly.
    pub fn with_docstring_semantics(mut self, semantics: DocStringSemantics) -> Self {
        self.docstring_semantics = semantics;
        self
    }

    /// Selects table row sizing without changing dialect or doc-string behavior.
    /// All constructors retain Legacy unless explicitly overridden.
    pub fn with_table_semantics(mut self, semantics: TableSemantics) -> Self {
        self.table_semantics = semantics;
        self
    }

    /// Selects classic lexical semantics independently of other policies.
    pub fn with_lexical_semantics(mut self, semantics: LexicalSemantics) -> Self {
        self.lexical_semantics = semantics;
        self
    }

    fn text<'a>(&self, text: &'a str) -> &'a str {
        match self.lexical_semantics {
            LexicalSemantics::Legacy => text.trim_end(),
            LexicalSemantics::Gherkin43 => text.trim_matches(official_docstring_whitespace),
        }
    }

    pub fn set_language(&self, language: &str) -> Result<(), &'static str> {
        let keywords = if self.latest_catalogue {
            Keywords::get_latest(language)
        } else {
            Keywords::get(language)
        }.ok_or_else(|| {
            self.set_fatal_error(EnvError::UnsupportedLanguage(language.into()));
            "Unsupported language"
        })?;

        *self.keywords.borrow_mut() = keywords;

        Ok(())
    }

    fn assert_no_error(&self) -> Result<(), &'static str> {
        if self.fatal_error.borrow().is_some() {
            return Err("fatal error");
        }

        Ok(())
    }

    fn set_fatal_error(&self, error: EnvError) {
        if self.fatal_error.borrow().is_some() {
            return;
        }

        *self.fatal_error.borrow_mut() = Some(error);
    }

    fn set_last_error(&self, error: EnvError) {
        *self.last_error.borrow_mut() = Some(error);
    }

    fn keywords(&self) -> std::cell::Ref<'_, Keywords<'static>> {
        self.keywords.borrow()
    }

    fn set_keyword(&self, kw: String) {
        *self.last_keyword.borrow_mut() = Some(kw);
    }

    fn clear_keyword(&self) {
        *self.last_keyword.borrow_mut() = None;
    }

    fn last_keyword(&self) -> std::cell::Ref<'_, Option<String>> {
        self.last_keyword.borrow()
    }

    fn take_keyword(&self) -> String {
        self.last_keyword.borrow_mut().take().unwrap()
    }

    fn set_last_step(&self, ty: StepType) {
        *self.last_step.borrow_mut() = Some(ty);
    }

    fn clear_last_step(&self) {
        *self.last_step.borrow_mut() = None;
    }

    fn last_step(&self) -> Option<StepType> {
        *self.last_step.borrow()
    }

    fn increment_nl(&self, offset: usize) {
        let mut line_offsets = self.line_offsets.borrow_mut();
        if !line_offsets.contains(&offset) {
            line_offsets.push(offset);
        }
    }

    fn position(&self, source: &str, offset: usize) -> LineCol {
        if self.lexical_semantics == LexicalSemantics::Gherkin43 {
            let prefix = &source[..offset];
            return LineCol { line: prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
                col: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1 };
        }
        let line_offsets = self.line_offsets.borrow();
        let line = line_offsets
            .iter()
            .position(|x| x > &offset)
            .unwrap_or(line_offsets.len());

        let col = offset - line_offsets[line - 1] + 1;

        LineCol { line, col }
    }

    fn escaped(&self) -> bool {
        *self.was_escaped.borrow()
    }

    fn set_escaped(&self, v: bool) {
        *self.was_escaped.borrow_mut() = v;
    }
}

impl Default for GherkinEnv {
    fn default() -> Self {
        GherkinEnv {
            keywords: RefCell::new(Keywords::default()),
            last_error: RefCell::new(None),
            fatal_error: RefCell::new(None),
            last_step: RefCell::new(None),
            last_keyword: RefCell::new(None),
            line_offsets: RefCell::new(vec![0]),
            was_escaped: RefCell::new(false),
            latest_catalogue: false,
            docstring_semantics: DocStringSemantics::Legacy,
            table_semantics: TableSemantics::Legacy,
            lexical_semantics: LexicalSemantics::Legacy,
        }
    }
}

// ECMAScript WhiteSpace + LineTerminator, matching the pinned JS token matcher.
// Rust's is_whitespace additionally includes NEL but excludes BOM, so it cannot
// define this compatibility boundary. Newline grammar remains line-oriented.
fn official_docstring_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

/// Content follows the pinned official line/token matcher: remove at most the
/// opening indentation, unescape the active separator on each line, and join
/// only content lines. Delimiter/header lines never become payload text.
fn official_docstring_content(lines: Vec<&str>, indent: usize, delimiter: &str) -> String {
    let escaped = if delimiter == "\"\"\"" { "\\\"\\\"\\\"" } else { "\\`\\`\\`" };
    lines.into_iter().map(|line| {
        let remove = line.chars().take(indent)
            .take_while(|character| official_docstring_whitespace(*character)).count();
        let content: String = line.chars().skip(remove).collect();
        // The tagged official JavaScript token matcher replaces the first
        // active escaped delimiter occurrence on each content line.
        content.replacen(escaped, delimiter, 1)
    }).collect::<Vec<_>>().join("\n")
}

fn official_table_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars();
    let _ = chars.next(); // initial pipe
    while let Some(character) = chars.next() {
        match character {
            '|' => {
                cells.push(cell.trim_matches(|c| matches!(c, ' ' | '\t' | '\u{000b}' | '\u{000c}' | '\r' | '\u{0085}' | '\u{00a0}')).to_owned());
                cell.clear();
            }
            '\\' => match chars.next() {
                Some('n') => cell.push('\n'),
                Some('|' ) => cell.push('|'),
                Some('\\') => cell.push('\\'),
                Some(other) => { cell.push('\\'); cell.push(other); }
                None => cell.push('\\'),
            },
            other => cell.push(other),
        }
    }
    cells
}

peg::parser! { pub(crate) grammar gherkin_parser(env: &GherkinEnv, source: &str) for str {

rule padding_char()
    = character:$([_]) {? if env.lexical_semantics.is_padding(character.chars().next().unwrap()) { Ok(()) } else { Err("token padding") } }
rule _() = quiet!{padding_char()*}
rule __() = quiet!{(padding_char() / nl())*}

rule nl0() = quiet!{"\r"? "\n"}
rule nl_no_comment() = quiet!{nl0() p:position!() {
    env.increment_nl(p);
}}
rule nl() = quiet!{comment_no_nl()? nl0() p:position!() {
    env.increment_nl(p);
}}
rule eof() = quiet!{![_]}
rule nl_eof() = quiet!{(nl() / padding_char())+ / eof()}
rule comment_no_nl() = quiet!{_ "#" $((!nl0()[_])*)}
rule comment() = quiet!{comment_no_nl() nl_eof()}
rule not_nl() -> &'input str = n:$((!nl0()[_])+) { n }

rule keyword1(list: &[&str]) -> &'input str
    = input:$([_]*<
        {list.iter().map(|x| x.chars().count()).min().unwrap()},
        {if env.latest_catalogue { env.keywords().all().iter().map(|x| x.chars().count()).max().unwrap() }
         else { list.iter().map(|x| x.chars().count()).max().unwrap() }}
    >) {?
        // println!("Input: {}", &input);
        if let Some(v) = list.iter().find(|x| input.starts_with(**x)) {
            let longer_step = if env.latest_catalogue {
                let keywords = env.keywords();
                let steps = [keywords.given, keywords.when, keywords.then, keywords.and, keywords.but];
                steps.iter().any(|group| std::ptr::eq(*group, list))
                    && steps.iter().flat_map(|group| group.iter()).any(|keyword| keyword.len() > v.len() && input.starts_with(*keyword))
            } else { false };
            if longer_step {
                env.clear_keyword();
                Err("longer step keyword has another kind")
            } else {
                env.set_keyword((*v).to_string());
                Err("success")
            }
        } else {
            // println!("Unfound: {}", &input);
            env.clear_keyword();
            env.set_last_error(EnvError::UnknownKeyword(input.into()));
            Err("unknown keyword")
        }
    }

rule keyword0(list: &[&str]) -> usize
    = keyword1(list)? {?
        match env.last_keyword().as_ref() {
            Some(v) => Ok(v.chars().count()),
            None => Err("no match")
        }
    }

pub(crate) rule keyword<'a>(list: &[&'a str]) -> &'a str
    = comment()* len:keyword0(list) [_]*<{len}> {
        let kw = env.take_keyword();
        list.iter().find(|x| **x == &*kw).unwrap()
    }

rule language_directive() -> ()
    = (padding_char() / nl_no_comment())* "#" _ "language" _ ":" _ l:$(not_nl()+) _ nl() {?
        env.set_language(if env.lexical_semantics == LexicalSemantics::Gherkin43 { env.text(l) } else { l })
    }

rule official_docstrings_enabled()
    = {? if env.docstring_semantics == DocStringSemantics::Gherkin43 {
        Ok(())
    } else { Err("official doc-string semantics not selected") } }

rule legacy_docstrings_enabled()
    = {? if env.docstring_semantics == DocStringSemantics::Legacy {
        Ok(())
    } else { Err("legacy doc-string semantics not selected") } }

rule docstring() -> String
    = official_docstrings_enabled() d:official_docstring() { d }
    / legacy_docstrings_enabled() d:legacy_docstring() { d }

// Line-oriented content is separate from the legacy permissive delimiter rule.
// The opening line contains optional media type; closing-line text is ignored,
// matching the pinned official token matcher's separator handling.
rule official_docstring_indent()
    = quiet!{['\u{0009}' | '\u{000b}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}']*}

rule official_docstring() -> String
    = pa:position!() padding:$(official_docstring_indent()) "\"\"\"" (!nl0() [_])* nl_no_comment()
      lines:((!(official_docstring_indent() "\"\"\"") line:$((!nl0() [_])*) nl_no_comment() { line })*)
      official_docstring_indent() "\"\"\"" (!nl0() [_])* nl_eof() {
        // The enclosing step/table already consumed an ASCII prefix. Count
        // newly consumed Unicode indentation in characters, not byte columns.
        official_docstring_content(lines, env.position(source, pa).col - 1 + padding.chars().count(), "\"\"\"")
    }
    / pa:position!() padding:$(official_docstring_indent()) "```" (!nl0() [_])* nl_no_comment()
      lines:((!(official_docstring_indent() "```") line:$((!nl0() [_])*) nl_no_comment() { line })*)
      official_docstring_indent() "```" (!nl0() [_])* nl_eof() {
        official_docstring_content(lines, env.position(source, pa).col - 1 + padding.chars().count(), "```")
    }

rule legacy_docstring() -> String
    = "\"\"\"" n:$((!"\"\"\"" (nl() / [_]))*) "\"\"\"" nl_eof() {
        textwrap::dedent(n)
    }
    / "```" n:$((!"```"(nl() / [_]))*) "```" nl_eof() {
        textwrap::dedent(n)
    }

rule escaped_cell_char() -> &'input str
    = "\\n" { "\n" } / "\\|" { "|" } / "\\\\" { "\\" }

rule table_cell() -> Vec<&'input str>
    = "|" _ !(nl0() / eof()) n:((escaped_cell_char() / $(!("|" / nl0())[_]))*) { n }

rule official_tables_enabled()
    = {? if env.table_semantics == TableSemantics::Gherkin43 { Ok(()) } else { Err("official table semantics not selected") } }

pub(crate) rule table_row() -> Vec<String>
    = official_tables_enabled() row:$("|" (!nl0() [_])*) nl_eof() { official_table_cells(row) }
    / legacy_table_row()

rule legacy_table_row() -> Vec<String>
    = n:(table_cell() ** _) _ "|" _ nl_eof() {
        n.into_iter()
            .map(|s| {
                let Some((start, first)) = s.iter().enumerate()
                    .find_map(|(n, s)| {
                        let trimmed = s.trim_start_matches([' ', '\t']);
                        (!trimmed.is_empty()).then_some((n, trimmed))
                    }) else { return String::new() };
                let Some((end, last)) = s.iter().enumerate().rev()
                    .find_map(|(n, s)| {
                        let trimmed = s.trim_end_matches([' ', '\t']);
                        (!trimmed.is_empty()).then_some((n, trimmed))
                    }) else { return String::new() };

                if start == end {
                    first.trim_end_matches([' ', '\t']).to_owned()
                } else {
                    iter::once(first)
                        .chain(
                            s.into_iter().skip(start + 1)
                                .take(end.saturating_sub(start + 1))
                        )
                        .chain(iter::once(last))
                        .collect::<String>()
                }
            })
            .collect()
    }

pub(crate) rule table0() -> Vec<Vec<String>>
    = _ d:(table_row() ++ _) {
        if d.is_empty() || env.table_semantics == TableSemantics::Gherkin43 {
            d
        } else {
            let len = d[0].len();
            d.into_iter().map(|mut x| { x.truncate(len); x }).collect()
        }
    }

pub(crate) rule table() -> Table
    = pa:position!() t:table0() pb:position!() {?
        if !t.is_empty() && t.iter().skip(1).any(|x| x.len() != t[0].len()) {
            env.set_fatal_error(EnvError::InconsistentCellCount(t));
            Err("inconsistent table row sizes")
        } else {
            Ok(Table::builder()
                .span(Span { start: pa, end: pb })
                .position(env.position(source, pa))
                .rows(t)
                .build())
        }
    }

// Gherkin 43 permits one table and one doc string in either order.
// Keeping the two alternatives bounded prevents accepting a repeated kind.
rule step_arguments() -> (Option<String>, Option<Table>)
    = d:docstring() t:table()? { (Some(d), t) }
    / t:table() d:docstring()? { (d, Some(t)) }
    / { (None, None) }

pub(crate) rule step() -> Step
    = comment()* pa:position!() k:keyword((env.keywords().given)) _ n:not_nl() pb:position!() _ nl_eof() _
      arguments:step_arguments()
    {
        env.set_last_step(StepType::Given);
        Step::builder().ty(StepType::Given)
            .keyword(k.to_string())
            .value(env.text(n).to_string())
            .table(arguments.1)
            .docstring(arguments.0)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }
    / pa:position!() k:keyword((env.keywords().when)) _ n:not_nl() pb:position!() _ nl_eof() _
      arguments:step_arguments()
    {
        env.set_last_step(StepType::When);
        Step::builder().ty(StepType::When)
            .keyword(k.to_string())
            .value(env.text(n).to_string())
            .table(arguments.1)
            .docstring(arguments.0)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }
    / pa:position!() k:keyword((env.keywords().then)) _ n:not_nl() pb:position!() _ nl_eof() _
      arguments:step_arguments()
    {
        env.set_last_step(StepType::Then);
        Step::builder().ty(StepType::Then)
            .keyword(k.to_string())
            .value(env.text(n).to_string())
            .table(arguments.1)
            .docstring(arguments.0)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }
    / pa:position!() k:keyword((env.keywords().and)) _ n:not_nl() pb:position!() _ nl_eof() _
      arguments:step_arguments()
    {
        let ty = env.last_step().unwrap_or(StepType::Given);
        env.set_last_step(ty);
        Step::builder().ty(ty)
            .keyword(k.to_string())
            .value(env.text(n).to_string())
            .table(arguments.1)
            .docstring(arguments.0)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }
    / pa:position!() k:keyword((env.keywords().but)) _ n:not_nl() pb:position!() _ nl_eof() _
      arguments:step_arguments()
    {
        let ty = env.last_step().unwrap_or(StepType::Given);
        env.set_last_step(ty);
        Step::builder().ty(ty)
            .keyword(k.to_string())
            .value(env.text(n).to_string())
            .table(arguments.1)
            .docstring(arguments.0)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }

pub(crate) rule steps() -> Vec<Step>
    = s:(step() ** _) {
        env.clear_last_step();
        s
    }

rule background() -> Background
    = comment()* _ pa:position!()
      k:keyword((env.keywords().background)) ":" _ n:not_nl()? nl_eof()
      d:description((&env.keywords().excluded_background()))?
      s:steps()?
      pb:position!()
    {
        Background::builder()
            .keyword(k.into())
            .name(env.text(n.unwrap_or_default()).to_string())
            .description(d.flatten())
            .steps(s.unwrap_or_default())
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }

rule any_directive() -> &'static str
    = k:keyword((&*env.keywords().all())) {
        k
    }

rule description_line(excluded: &[&str]) -> &'input str
    = _
      !"@" !keyword((excluded))
      _ n:not_nl() nl_eof()
    {
        n
    }

rule description(excluded: &[&str]) -> Option<String>
    = d:(description_line(excluded) ** _) __ {
        let d = d.join("\n");
        if d.trim_matches(|character| env.lexical_semantics.is_whitespace(character)).is_empty() {
            None
        } else {
            Some(d)
        }
    }

rule examples() -> Examples
    = comment()*
      _
      t:tags()
      _
      pa:position!()
      k:keyword((env.keywords().examples)) ":" _ n:not_nl()? nl_eof()
      d:description((&env.keywords().excluded_examples()))?
      tb:table()?
      pb:position!()
    {
        Examples::builder()
            .keyword(k.into())
            .name(n.map(|n| env.text(n).to_string()))
            .description(d.flatten())
            .tags(t)
            .table(tb)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }

rule scenario() -> Scenario
    = comment()*
      _
      t:tags()
      _
      pa:position!()
      k:keyword((env.keywords().scenario)) ":" _ n:not_nl()? _ nl_eof()
      d:description((&env.keywords().excluded_scenario()))?
      s:steps()?
      e:examples()*
      pb:position!()
    {
        Scenario::builder()
            .keyword(k.into())
            .name(env.text(n.unwrap_or_default()).to_string())
            .description(d.flatten())
            .tags(t)
            .steps(s.unwrap_or_default())
            .examples(e)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }
    / comment()*
      _
      t:tags()
      _
      pa:position!()
      k:keyword((env.keywords().scenario_outline)) ":" _ n:not_nl()? _ nl_eof()
      d:description((&env.keywords().excluded_scenario_outline()))?
      s:steps()?
      e:examples()*
      pb:position!()
    {
        Scenario::builder()
            .keyword(k.into())
            .name(env.text(n.unwrap_or_default()).to_string())
            .description(d.flatten())
            .tags(t)
            .steps(s.unwrap_or_default())
            .examples(e)
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }

rule tag_char() -> &'input str
    = s:$([_]) {?
        let x = s.chars().next().unwrap();
        if !env.lexical_semantics.is_whitespace(x) && x != '@' {
            Ok(s)
        } else {
            Err("tag character")
        }
    }

pub(crate) rule tag() -> String
    = "@" s:tag_char()+ { s.join("") }

rule tag_in_expr_char() -> Option<&'input str>
    = s:$([_]) {?
        let x = s.chars().next().unwrap();
        if !env.escaped() && x == '\\' {
            env.set_escaped(true);
            Ok(None)
        } else if env.escaped() {
            env.set_escaped(false);
            if "\\() ".contains(x) {
                Ok(Some(s))
            } else {
                Err("escaped non-reserved char")
            }
        } else if !x.is_whitespace() && !"@()\\".contains(x) {
            Ok(Some(s))
        } else {
            Err("tag character")
        }
    }

pub(crate) rule tag_in_expr() -> String
    = "@" s:tag_in_expr_char()+ {?
        if env.escaped() {
            env.set_escaped(false);
            Err("escaped end of line")
        } else {
            Ok(s.into_iter().flatten().collect())
        }
    }

pub(crate) rule tags() -> Vec<String>
    = t:(tag() ** __) _ nl()* { t }
    / { vec![] }

rule rule_() -> Rule
    = _
      t:tags()
      _
      pa:position!()
      k:keyword((env.keywords().rule)) ":" _ n:not_nl()? _ nl_eof()
      d:description((&env.keywords().excluded_rule()))?
      b:background()? nl()*
      s:scenarios()? nl()*
    //   e:examples()?
      pb:position!()
    {
        Rule::builder()
            .keyword(k.into())
            .name(env.text(n.unwrap_or_default()).to_string())
            .description(d.flatten())
            .tags(t)
            .background(b)
            .scenarios(s.unwrap_or_default())
            .span(Span { start: pa, end: pb })
            .position(env.position(source, pa))
            .build()
    }

rule rules() -> Vec<Rule>
    = _ r:(rule_() ** _)? { r.unwrap_or_default() }

pub(crate) rule scenarios() -> Vec<Scenario>
    = _ s:(scenario() ** _)? { s.unwrap_or_default() }

// Featureless sources are valid documents, but remain outside Feature::parse.
pub(crate) rule document() -> Option<Feature>
    = _ (!language_directive() comment() / nl_no_comment() / padding_char())* eof() {?
        env.assert_no_error().map(|()| None)
    }
    / feature:feature() { Some(feature) }

// A whitespace-only line consumed by tags may leave next-line indentation.
// Latest source mode follows the official line matcher; legacy stays unchanged.
rule latest_source_enabled()
    = {? if env.latest_catalogue || env.lexical_semantics == LexicalSemantics::Gherkin43 { Ok(()) } else { Err("latest source mode not selected") } }

rule latest_feature_padding()
    = latest_source_enabled() _ / ""

// A directive/comment preamble can leave padding before the first tag line.
// Broaden this boundary only with the explicit lexical policy.
rule official_lexical_enabled()
    = {? if env.lexical_semantics == LexicalSemantics::Gherkin43 { Ok(()) } else { Err("official lexical policy not selected") } }

rule lexical_feature_padding()
    = official_lexical_enabled() _ / ""

pub(crate) rule feature() -> Feature
    = _ language_directive()?
      nl()*
      lexical_feature_padding()
      t:tags()
      latest_feature_padding()
      pa:position!()
      k:keyword((env.keywords().feature)) ":" _ n:not_nl()? _ nl_eof()
      d:description((&env.keywords().excluded_feature()))?
      b:background()? nl()*
      s:scenarios() nl()*
      r:rules() pb:position!()
      nl()*
    {?
        if let Err(e) = env.assert_no_error() {
            Err(e)
        } else {
            Ok(Feature::builder()
                .keyword(k.into())
                .tags(t)
                .name(env.text(n.unwrap_or_default()).to_string())
                .description(d.flatten())
                .background(b)
                .scenarios(s)
                .rules(r)
                .span(Span { start: pa, end: pb })
                .position(env.position(source, pa))
                .build())
        }
    }

pub(crate) rule tag_operation() -> TagOperation = precedence!{
    x:(@) _ "or" _ y:@ { TagOperation::Or(Box::new(x), Box::new(y)) }
    --
    x:(@) _ "and" _ y:@ { TagOperation::And(Box::new(x), Box::new(y)) }
    --
    "not" _ x:(@) { TagOperation::Not(Box::new(x)) }
    --
    "(" _ t:tag_operation() _ ")" _ { t }
    t:tag_in_expr() { TagOperation::Tag(t) }
}

}}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, fs};

    use super::*;

    const FOO: &str = "# language: formal\r\n
@hot-stuff
Section: 4.2. The thing we care about
A description just jammed in here for no reason
@lol @a @rule     @with-spaces
Rule: All gubbins must be placed in the airlock

@bad_idea
Evidence: A gubbins in an airlock
    Given a gubbins
    \"\"\"
    That's a gubbins
    and that is
    and so is that
    \"\"\"
    When a gubbins is forced into this weird corner
    | a | b | c |
    | 1 | 2 | 3 |
    | 4 | 5 | 6 |
    Then a gubbins is proven to be in an airlock
";

    // From Gherkin 6 documentation
    const RULE_WITH_BACKGROUND: &str = "
Feature: Overdue tasks
  Let users know when tasks are overdue, even when using other
  features of the app

  Rule: Users are notified about overdue tasks on first use of the day
    Background:
      Given I have overdue tasks

    Example: First use of the day
      Given I last used the app yesterday
      When I use the app
      Then I am notified about overdue tasks

    Example: Already used today
      Given I last used the app earlier today
      When I use the app
      Then I am not notified about overdue tasks
";

    const DOCSTRING: &str = r#"
Feature: Meow

Scenario: Meow
  Given meow
    """
    Docstring life!
    """
"#;

    const DOCSTRING2: &str = r#"
Feature: Meow

Scenario: Meow
  Given meow
    ```
    Docstring life!
    ```
"#;

    #[test]
    fn smoke() {
        let env = GherkinEnv::default();
        assert!(gherkin_parser::feature(FOO, &env, FOO).is_ok());
    }

    #[test]
    fn smoke2() {
        let env = GherkinEnv::default();
        let d = env!("CARGO_MANIFEST_DIR");
        let s = fs::read_to_string(format!("{}/tests/test.feature", d)).unwrap();
        assert!(gherkin_parser::feature(&s, &env, &s).is_ok());
    }

    #[test]
    fn rule_with_background() {
        let env = GherkinEnv::default();
        assert!(
            gherkin_parser::feature(RULE_WITH_BACKGROUND, &env, RULE_WITH_BACKGROUND).is_ok(),
            "RULE_IN_BACKGROUND was not parsed correctly!"
        );
    }

    #[test]
    fn docstring() {
        let env = GherkinEnv::default();
        assert!(
            gherkin_parser::feature(DOCSTRING, &env, DOCSTRING).is_ok(),
            "DOCSTRING was not parsed correctly!"
        );
    }

    #[test]
    fn docstring2() {
        let env = GherkinEnv::default();
        assert!(
            gherkin_parser::feature(DOCSTRING2, &env, DOCSTRING2).is_ok(),
            "DOCSTRING2 was not parsed correctly!"
        );
    }

    #[test]
    fn feature_name_and_scenario() {
        let env = GherkinEnv::default();
        let input = r#"Feature: Basic functionality
        here's some text
        really
Scenario: Hello
  Given a step"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        println!("{:#?}", feature);
        assert_eq!(feature.scenarios.len(), 1);
        assert!(feature.description.is_some());
        assert!(feature.scenarios[0].steps[0].position.line != 0);
    }

    #[test]
    fn correct_line_numbers() {
        let env = GherkinEnv::default();
        let input = r#"
# language: en
Feature: Basic functionality
        here's some text
     really
@tag
Scenario: Hello
  Given a step
  Then a step
@tag
Scenario: Hello
  Given a step

  And more

# comment
Rule: rule
    @tag
    Scenario Outline: Hello
        Given <step>
        """
        Doc String
        """

    Examples:
        | step |
        | 1    |
        | 2    |


    @tag
Rule: rule
    #comment
    Scenario: Hello
        Given a step
"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        assert_eq!(feature.scenarios.len(), 2);
        assert!(feature.description.is_some());
        assert_eq!(feature.position.line, 3);
        assert_eq!(feature.scenarios[0].position.line, 7);
        assert_eq!(feature.scenarios[0].steps[0].position.line, 8);
        assert_eq!(feature.scenarios[0].steps[1].position.line, 9);
        assert_eq!(feature.scenarios[1].position.line, 11);
        assert_eq!(feature.scenarios[1].steps[0].position.line, 12);
        assert_eq!(feature.scenarios[1].steps[1].position.line, 14);
        assert_eq!(feature.rules[0].position.line, 17);
        assert_eq!(feature.rules[0].position.line, 17);
        assert_eq!(feature.rules[0].scenarios[0].position.line, 19);
        assert_eq!(feature.rules[0].scenarios[0].steps[0].position.line, 20);
        assert_eq!(feature.rules[0].scenarios[0].examples[0].position.line, 25);
        assert_eq!(
            feature.rules[0].scenarios[0].examples[0]
                .table
                .as_ref()
                .unwrap()
                .position
                .line,
            26,
        );
        assert_eq!(
            feature.rules[0].scenarios[0].examples[0]
                .table
                .as_ref()
                .unwrap()
                .rows
                .len(),
            3,
        );
        assert_eq!(feature.rules[1].position.line, 32);
        assert_eq!(feature.rules[1].scenarios[0].position.line, 34);
        assert_eq!(feature.rules[1].scenarios[0].steps[0].position.line, 35);
    }

    #[test]
    fn and_as_first_step() {
        let env = GherkinEnv::default();
        let input = r#"
Feature: First step with And
  Scenario: And as first step
    And a step
"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        assert_eq!(feature.scenarios.len(), 1);
        assert_eq!(feature.scenarios[0].steps.len(), 1);
        assert_eq!(feature.scenarios[0].steps[0].ty, StepType::Given);
    }

    #[test]
    fn but_as_first_step() {
        let env = GherkinEnv::default();
        let input = r#"
Feature: First step with But
  Scenario: But as first step
    But a step
"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        assert_eq!(feature.scenarios.len(), 1);
        assert_eq!(feature.scenarios[0].steps.len(), 1);
        assert_eq!(feature.scenarios[0].steps[0].ty, StepType::Given);
    }

    #[test]
    fn scenario_description() {
        let env = GherkinEnv::default();
        let input = r#"
Feature: First step with Besides
  Scenario: Besides as first step
    Besides a step
"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        assert_eq!(feature.scenarios.len(), 1);
        assert_eq!(feature.scenarios[0].steps.len(), 0);
    }

    #[test]
    fn feature_only() {
        let env = GherkinEnv::default();
        let input = r#"Feature: Basic functionality
        "#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();
        println!("{:#?}", feature);
        assert_eq!(feature.scenarios.len(), 0);
        assert!(feature.description.is_none());
    }

    #[test]
    fn escape_examples() {
        let env = GherkinEnv::default();
        let input = r#"
Feature: Foo
  Scenario: Bar
    Examples:
      | value |
      |  alu  |
      |   l   |
      |       |
      | \n    |
      | \|    |
      | \\    |
      | \\n   |
      | \\\\  |
"#;
        let feature = gherkin_parser::feature(input, &env, input).unwrap();

        assert_eq!(
            feature.scenarios[0].examples[0]
                .table
                .as_ref()
                .unwrap()
                .rows,
            vec![
                vec!["value".to_owned()],
                vec!["alu".to_owned()],
                vec!["l".to_owned()],
                vec!["".to_owned()],
                vec!["\n".to_owned()],
                vec!["|".to_owned()],
                vec!["\\".to_owned()],
                vec!["\\n".to_owned()],
                vec!["\\\\".to_owned()],
            ]
        );
    }

    #[test]
    fn reject_invalid_example_escape_sequence() {
        let env = GherkinEnv::default();
        let input = r#"
Feature: Foo
  Scenario: Bar
    Examples:
      | value |
      | \     |
"#;
        assert!(gherkin_parser::feature(input, &env, input).is_err());
    }

    #[test]
    fn fixture_good() {
        // We cannot handle missing features very well yet.
        let skip = ["empty.feature", "incomplete_feature_3.feature"];
        let mut failed = 0;

        let d = env!("CARGO_MANIFEST_DIR");
        let files = fs::read_dir(format!("{}/tests/fixtures/data/good/", d)).unwrap();
        for file in files {
            let file = file.unwrap();
            let filename = file.file_name();
            let filename = filename.to_str().unwrap();
            if filename.ends_with(".feature") {
                if skip.contains(&filename) {
                    continue;
                }
                let res = std::panic::catch_unwind(|| {
                    let env = GherkinEnv::default();
                    let input =
                        fs::read_to_string(format!("{}/tests/fixtures/data/good/{}", d, filename,))
                            .unwrap();
                    let feature = gherkin_parser::feature(&input, &env, &input).unwrap();
                    let fixture = fs::read_to_string(format!(
                        "{}/tests/fixtures/data/good/{}.ast.ndjson",
                        d, filename,
                    ))
                    .unwrap();

                    check_ast(&feature, &fixture);
                });
                if res.is_err() {
                    failed += 1;
                    println!("{}", filename);
                }
            }
        }

        assert_eq!(failed, 0, "{} fixtures have failed", failed);
    }

    #[test]
    fn fixture_fail() {
        let d = env!("CARGO_MANIFEST_DIR");
        let files = fs::read_dir(format!("{}/tests/fixtures/data/bad/", d)).unwrap();
        for file in files {
            let file = file.unwrap();
            let filename = file.file_name();
            let filename = filename.to_str().unwrap();
            if filename.ends_with(".feature") {
                let res = std::panic::catch_unwind(|| {
                    let env = GherkinEnv::default();
                    let input =
                        fs::read_to_string(format!("{}/tests/fixtures/data/bad/{}", d, filename,))
                            .unwrap();
                    gherkin_parser::feature(&input, &env, &input).unwrap()
                });

                assert!(res.is_err(), "{}: {:?}", filename, res.unwrap());
            }
        }
    }

    // TODO: Actually generate `.ndjson` file from the AST to fully assert
    //       fixtures.
    fn check_ast(parsed: &Feature, ast_parsed: &str) {
        let d: HashMap<String, serde_json::Value> = serde_json::from_str(ast_parsed).unwrap();

        let document = d
            .get("gherkinDocument")
            .expect("There is no document in the file");
        let feature = document
            .get("feature")
            .expect("There is no feature in the document");
        let children = feature.get("children");

        if children.is_none() {
            assert!(parsed.background.is_none());
            assert_eq!(parsed.scenarios.len(), 0);
            assert_eq!(parsed.rules.len(), 0);
            return;
        }

        let mut backgrounds = 0;
        let mut scenarios = 0;
        let mut rules = 0;

        let children = children.unwrap().as_array().unwrap();
        for child in children {
            if let Some(background) = child.get("background") {
                let parsed_background = parsed.background.as_ref().unwrap();
                let name = background.get("name").unwrap().as_str().unwrap();

                assert_eq!(&parsed_background.name, name);

                let steps = background.get("steps");

                if steps.is_none() {
                    assert_eq!(parsed_background.steps.len(), 0);
                    continue;
                }

                let steps = steps
                    .expect("steps")
                    .as_array()
                    .expect("Steps must be an array");
                assert!(
                    check_steps(&parsed_background.steps, steps),
                    "Background steps are different from fixture",
                );

                backgrounds += 1;
            } else if let Some(json_scenario) = child.get("scenario") {
                assert!(
                    check_scenario(&parsed.scenarios, json_scenario),
                    "Scenario steps are different from fixture",
                );

                scenarios += 1;
            } else if let Some(json_rule) = child.get("rule") {
                let json_rule_scenarios = json_rule
                    .get("children")
                    .and_then(serde_json::Value::as_array)
                    .map(|children| {
                        children
                            .iter()
                            .filter_map(|child| child.get("scenario"))
                            .collect::<Vec<_>>()
                    });
                let json_rule_name = json_rule.get("name").unwrap().as_str().unwrap();

                let exists = parsed.rules.iter().any(|rule| {
                    if rule.name == json_rule_name {
                        return if let Some(json_scenarios) = &json_rule_scenarios {
                            json_scenarios.len() == rule.scenarios.len()
                                && json_scenarios.iter().all(|json_scenario| {
                                    check_scenario(&rule.scenarios, json_scenario)
                                })
                        } else {
                            rule.scenarios.is_empty()
                        };
                    }
                    false
                });

                assert!(exists, "Scenario rules are different from fixture");

                rules += 1;
            } else {
                panic!("Unknown child type: {:#?}", child);
            }
        }

        if parsed.background.is_some() {
            assert_eq!(1, backgrounds);
        } else {
            assert_eq!(0, backgrounds);
        }

        assert_eq!(parsed.scenarios.len(), scenarios);
        assert_eq!(parsed.rules.len(), rules);
    }

    fn check_scenario(parsed: &[Scenario], json: &serde_json::Value) -> bool {
        let json_steps = json.get("steps").and_then(serde_json::Value::as_array);
        let json_scenario_name = json.get("name").unwrap().as_str().unwrap();

        parsed.iter().any(|parsed| {
            if parsed.name == json_scenario_name {
                return if let Some(json_steps) = json_steps {
                    check_steps(&parsed.steps, json_steps)
                } else {
                    parsed.steps.is_empty()
                };
            }
            false
        })
    }

    fn check_steps(parsed: &[Step], json: &[serde_json::Value]) -> bool {
        if parsed.len() != json.len() {
            return false;
        }

        for (step, json) in parsed.iter().zip(json) {
            if step.keyword != json.get("keyword").unwrap().as_str().unwrap() {
                return false;
            }
            if step.value != json.get("text").unwrap().as_str().unwrap() {
                return false;
            }
        }

        true
    }
}
