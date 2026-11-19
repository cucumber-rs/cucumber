//! Source-backed Gherkin AST and Pickle representation for Messages 34.2.1.
//!
//! This representation preserves the native execution AST separately. Source
//! locations, descriptions and arguments are derived from the parser's
//! immutable
//! snapshot, never from expanded scenario line numbers or reread files.

use std::{collections::HashMap, sync::LazyLock};

use serde_json::{Value, json};

use super::Error;
use crate::{
    feature::Ext as _,
    parser::{DialectCatalog, EmptyDocumentSnapshot, SourceSnapshot},
};

#[path = "markdown.rs"]
mod markdown;

/// Source-faithful AST and compiled Pickles, with native execution identities.
#[derive(Debug)]
pub struct SourceGraph {
    document: Value,
    language: String,
    pickles: Vec<Value>,
    pub(super) cases: Vec<NativeCase>,
}

#[derive(Clone, Debug)]
pub(super) struct NativeCase {
    pub scenario: gherkin::Scenario,
    pub rule: Option<gherkin::Rule>,
    pub steps: Vec<gherkin::Step>,
    pub pickle: Value,
}

impl SourceGraph {
    /// Converts an immutable parser snapshot into an official-shaped graph.
    ///
    /// The snapshot-selected dialect catalogue supplies all keyword spellings.
    /// Native execution candidates are
    /// paired with real source example rows before filtering or reporting.
    ///
    /// # Errors
    /// Returns an error for unsupported dialects or a mismatch between source
    /// structure and native execution semantics.
    pub fn from_snapshot(
        uri: &str,
        snapshot: &SourceSnapshot,
    ) -> Result<Self, Error> {
        if let Some(document) = snapshot.markdown() {
            return Self::from_markdown_document(uri, document);
        }
        Self::from_feature(
            uri,
            snapshot.source(),
            snapshot.feature(),
            snapshot.default_language(),
            snapshot.dialect_catalog(),
            snapshot.example_semantics(),
            snapshot.lexical_semantics(),
        )
    }

    /// Parses source directly using all adopted Gherkin 43 compatibility
    /// policies.
    ///
    /// This explicit opt-in compiler selects the current dialect catalogue,
    /// normalized doc strings, strict table rows and official outline
    /// expansion.
    /// Empty/comment-only documents have no feature or Pickles. This does not
    /// change Basic parser or native Feature defaults or imply untested parity.
    ///
    /// # Errors
    /// Rejects invalid native documents or source/native conversion mismatches.
    pub fn parse_gherkin43(
        uri: &str,
        source: &str,
        default_language: &str,
    ) -> Result<Self, Error> {
        let environment = gherkin::GherkinEnv::new_latest(default_language)
            .map_err(|error| Error::Source(error.to_string()))?
            .with_docstring_semantics(gherkin::DocStringSemantics::Gherkin43)
            .with_table_semantics(gherkin::TableSemantics::Gherkin43)
            .with_lexical_semantics(gherkin::LexicalSemantics::Gherkin43);
        let native =
            gherkin::Document::parse(source, environment).map_err(|error| {
                Error::Source(format!("native document parsing: {error}"))
            })?;
        if let Some(feature) = native.feature {
            Self::from_feature(
                uri,
                source,
                &feature,
                default_language,
                DialectCatalog::Gherkin43,
                crate::feature::ExampleSemantics::Gherkin43,
                gherkin::LexicalSemantics::Gherkin43,
            )
        } else {
            Ok(Self::empty_graph(
                uri,
                source,
                default_language,
                gherkin::LexicalSemantics::Gherkin43,
            ))
        }
    }

    /// Converts an immutable parser-validated featureless source generation.
    ///
    /// No filesystem read or native Feature is synthesized. The snapshot type
    /// is constructed only after complete native Document validation.
    ///
    /// # Errors
    /// Returns an error if the source path cannot be represented as a UTF-8
    /// URI.
    pub fn from_empty_document(
        snapshot: &EmptyDocumentSnapshot,
    ) -> Result<Self, Error> {
        let uri = snapshot.path().to_str().ok_or_else(|| {
            Error::Source("a UTF-8 source path is required".into())
        })?;
        if let Some(document) = snapshot.markdown() {
            return Self::from_markdown_document(uri, document);
        }
        Ok(Self::empty_graph(
            uri,
            snapshot.source(),
            snapshot.default_language(),
            snapshot.lexical_semantics(),
        ))
    }

    fn empty_graph(
        uri: &str,
        source: &str,
        default_language: &str,
        lexical: gherkin::LexicalSemantics,
    ) -> Self {
        let comments = source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                (line
                    .trim_start_matches(|character| {
                        lexical.is_whitespace(character)
                    })
                    .starts_with('#')
                    && language_directive(line, lexical).is_none())
                .then(|| {
                    json!({
                    "location": {
                    "line": index + 1,
                     "column": 1}
                    ,
                     "text": line}
                    )
                })
            })
            .collect::<Vec<_>>();
        Self {
            document: json!({"uri": uri, "comments": comments}),
            language: default_language.to_owned(),
            pickles: vec![],
            cases: vec![],
        }
    }

    fn from_feature(
        uri: &str,
        source: &str,
        native: &gherkin::Feature,
        default_language: &str,
        catalogue: DialectCatalog,
        examples: crate::feature::ExampleSemantics,
        lexical: gherkin::LexicalSemantics,
    ) -> Result<Self, Error> {
        let parsed = SourceTree::parse(
            uri,
            source,
            default_language,
            catalogue,
            lexical,
        )?;
        Self::from_tree(uri, parsed, native, examples)
    }

    fn from_tree(
        uri: &str,
        parsed: SourceTree,
        native: &gherkin::Feature,
        examples: crate::feature::ExampleSemantics,
    ) -> Result<Self, Error> {
        let expanded =
            native.clone().expand_examples_with(examples).map_err(|e| {
                Error::Source(format!("native example expansion: {e}"))
            })?;
        if parsed.feature.header.keyword != native.keyword
            || parsed.feature.header.name != native.name
        {
            return Err(Error::Source(
                "captured source does not match its native feature".into(),
            ));
        }
        if parsed.feature.scenarios.len() != native.scenarios.len()
            || parsed.feature.rules.len() != native.rules.len()
        {
            return Err(Error::Source(
                "captured source/native child structure differs".into(),
            ));
        }
        let mut graph = Self {
            document: parsed.document(),
            language: parsed.language.clone(),
            pickles: Vec::new(),
            cases: Vec::new(),
        };
        let feature_tags = parsed.feature.header.tags.clone();
        let background = parsed
            .feature
            .background
            .as_ref()
            .map(|b| b.steps.as_slice())
            .unwrap_or_default();
        for (scenario, raw) in
            parsed.feature.scenarios.iter().zip(&native.scenarios)
        {
            graph.compile_scenario(
                uri,
                scenario,
                raw,
                &expanded.scenarios,
                None,
                background,
                native
                    .background
                    .iter()
                    .flat_map(|b| &b.steps)
                    .cloned()
                    .collect(),
                &feature_tags,
            )?;
        }
        for ((rule, raw_rule), expanded_rule) in
            parsed.feature.rules.iter().zip(&native.rules).zip(&expanded.rules)
        {
            if rule.header.name != raw_rule.name
                || rule.scenarios.len() != raw_rule.scenarios.len()
            {
                return Err(Error::Source(
                    "captured source/native rule structure differs".into(),
                ));
            }
            let mut tags = feature_tags.clone();
            tags.extend(rule.header.tags.clone());
            let mut backgrounds = background.to_vec();
            backgrounds.extend(
                rule.background.iter().flat_map(|b| b.steps.iter()).cloned(),
            );
            let native_backgrounds = native
                .background
                .iter()
                .flat_map(|b| &b.steps)
                .chain(raw_rule.background.iter().flat_map(|b| &b.steps))
                .cloned()
                .collect::<Vec<_>>();
            for (scenario, raw) in
                rule.scenarios.iter().zip(&raw_rule.scenarios)
            {
                graph.compile_scenario(
                    uri,
                    scenario,
                    raw,
                    &expanded_rule.scenarios,
                    Some(raw_rule),
                    &backgrounds,
                    native_backgrounds.clone(),
                    &tags,
                )?;
            }
        }
        Ok(graph)
    }

    /// Returns the GherkinDocument payload (without its Envelope wrapper).
    #[must_use]
    pub const fn gherkin_document(&self) -> &Value {
        &self.document
    }

    /// Returns the compiled Pickle payloads in source order.
    #[must_use]
    pub fn pickles(&self) -> &[Value] {
        &self.pickles
    }

    fn compile_scenario(
        &mut self,
        uri: &str,
        scenario: &ScenarioNode,
        raw: &gherkin::Scenario,
        expanded: &[gherkin::Scenario],
        rule: Option<&gherkin::Rule>,
        backgrounds: &[StepNode],
        native_backgrounds: Vec<gherkin::Step>,
        inherited_tags: &[Value],
    ) -> Result<(), Error> {
        if scenario.header.name != raw.name
            || scenario.steps.len() != raw.steps.len()
            || scenario.examples.len() != raw.examples.len()
        {
            return Err(Error::Source(
                "captured source/native scenario structure differs".into(),
            ));
        }
        // Official Pickles omit all backgrounds when the scenario has no own
        // steps. Keep source AST backgrounds intact. Legacy runtime events for
        // those steps will visibly mismatch this official execution plan.
        let backgrounds =
            if scenario.steps.is_empty() { &[][..] } else { backgrounds };
        let native_backgrounds = if scenario.steps.is_empty() {
            Vec::new()
        } else {
            native_backgrounds
        };
        let candidates = expanded
            .iter()
            .filter(|candidate| candidate.span == raw.span)
            .collect::<Vec<_>>();
        let mut variants: Vec<(
            Option<&Value>,
            Vec<String>,
            Vec<String>,
            Vec<Value>,
        )> = Vec::new();
        if scenario.examples.is_empty() {
            variants.push((None, vec![], vec![], vec![]));
        } else {
            for (example, native_example) in
                scenario.examples.iter().zip(&raw.examples)
            {
                let Some((header, rows)) = example.rows.split_first() else {
                    continue;
                };
                let headers = cell_values(header);
                let source_rows =
                    example.rows.iter().map(cell_values).collect::<Vec<_>>();
                if native_example.table.as_ref().map(|t| &t.rows)
                    != Some(&source_rows)
                {
                    return Err(Error::Source(
                        "native Examples values differ from source table"
                            .into(),
                    ));
                }
                for row in rows {
                    variants.push((
                        Some(row),
                        headers.clone(),
                        cell_values(row),
                        example.header.tags.clone(),
                    ));
                }
            }
        }
        if variants.len() != candidates.len() {
            return Err(Error::Source(
                "native scenario expansions differ from source example rows"
                    .into(),
            ));
        }
        for ((row, headers, values, example_tags), candidate) in
            variants.into_iter().zip(candidates)
        {
            let replacements = headers.iter().zip(&values).collect::<Vec<_>>();
            let interpolate = |s: &str| interpolate(s, &replacements);
            let row_id = row.map(|r| r["id"].as_str().unwrap());
            let location = row
                .map(|r| r["location"].clone())
                .unwrap_or_else(|| scenario.header.location());
            let pickle_id = format!(
                "{uri}#pickle-{}",
                row_id.unwrap_or(&scenario.header.id)
            );
            let mut steps = Vec::new();
            let mut last_type = "Unknown";
            for (index, (step, is_background)) in backgrounds
                .iter()
                .map(|s| (s, true))
                .chain(scenario.steps.iter().map(|s| (s, false)))
                .enumerate()
            {
                if step.keyword_type() != "Conjunction" {
                    last_type = step.keyword_type();
                }
                let mut ast_ids = vec![Value::String(step.id.clone())];
                if !is_background {
                    if let Some(id) = row_id {
                        ast_ids.push(Value::String(id.to_owned()));
                    }
                }
                let mut value = json!({
                    "id": format!("{pickle_id}-step-{index}"),

                    "astNodeIds": ast_ids,
                         "text": if is_background {
                         step.text.clone() }
                         else {
                         interpolate(&step.text) }
                        ,

                    "type": last_type});
                if let Some(argument) = step.pickle_argument(if is_background {
                    &[]
                } else {
                    &replacements
                }) {
                    value["argument"] = argument;
                }
                steps.push(value);
            }
            let mut tags = inherited_tags.to_vec();
            tags.extend(scenario.header.tags.clone());
            tags.extend(example_tags);
            let tags = tags
                .iter()
                .map(|t| json!({"name": t["name"], "astNodeId": t["id"]}))
                .collect::<Vec<_>>();
            let mut ast_ids = vec![Value::String(scenario.header.id.clone())];
            if let Some(id) = row_id {
                ast_ids.push(Value::String(id.to_owned()));
            }
            let pickle = json!({
                "id": pickle_id,
                 "uri": uri,
                 "location": location,

                "name": interpolate(&scenario.header.name),
                     "language": self.language,
                     "steps": steps,

                "tags": tags, "astNodeIds": ast_ids});
            let native_steps = native_backgrounds
                .iter()
                .chain(&candidate.steps)
                .cloned()
                .collect::<Vec<_>>();
            if pickle["name"] != candidate.name
                || steps_text(&pickle)
                    != native_steps
                        .iter()
                        .map(|s| s.value.clone())
                        .collect::<Vec<_>>()
            {
                return Err(Error::Source(
                    "native substitutions differ from official Pickle text"
                        .into(),
                ));
            }
            // Source row identity is assigned above. Native synthetic positions
            // are retained only inside this execution candidate, not emitted.
            self.cases.push(NativeCase {
                scenario: candidate.clone(),
                rule: rule.cloned(),
                steps: native_steps,
                pickle: pickle.clone(),
            });
            self.pickles.push(pickle);
        }
        Ok(())
    }
}

fn steps_text(pickle: &Value) -> Vec<String> {
    pickle["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["text"].as_str().unwrap().to_owned())
        .collect()
}

fn cell_values(row: &Value) -> Vec<String> {
    row["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["value"].as_str().unwrap().to_owned())
        .collect()
}

fn interpolate(text: &str, replacements: &[(&String, &String)]) -> String {
    replacements.iter().fold(text.to_owned(), |value, (header, replacement)| {
        value.replace(&format!("<{header}>"), replacement)
    })
}

#[derive(Clone, Debug)]
struct Header {
    lexical: gherkin::LexicalSemantics,
    id: String,
    line: usize,
    column: usize,
    keyword: String,
    name: String,
    description: Vec<String>,
    tags: Vec<Value>,
}

impl Header {
    fn location(&self) -> Value {
        json!({"line": self.line, "column": self.column})
    }
    fn description(&self) -> String {
        let first = self
            .description
            .iter()
            .position(|s| {
                !s.trim_matches(|character| {
                    self.lexical.is_whitespace(character)
                })
                .is_empty()
            })
            .unwrap_or(self.description.len());
        let last = self
            .description
            .iter()
            .rposition(|s| {
                !s.trim_matches(|character| {
                    self.lexical.is_whitespace(character)
                })
                .is_empty()
            })
            .map_or(first, |i| i + 1);
        self.description[first..last].join("\n")
    }
    fn json(&self) -> Value {
        json!({
        "id": self.id,
         "location": self.location(),
         "keyword": self.keyword,

        "name": self.name,
             "description": self.description(),
             "tags": self.tags}
            )
    }
}

#[derive(Clone, Debug)]
struct StepNode {
    id: String,
    line: usize,
    column: usize,
    keyword: String,
    keyword_type: &'static str,
    text: String,
    doc: Option<Value>,
    table: Option<Value>,
}

impl StepNode {
    fn keyword_type(&self) -> &'static str {
        self.keyword_type
    }
    fn json(&self) -> Value {
        let mut value = json!({
        "id": self.id,
         "location": {
        "line": self.line,
         "column": self.column}
        ,

        "keyword": self.keyword,
             "keywordType": self.keyword_type(),
             "text": self.text}
            );
        if let Some(doc) = &self.doc {
            value["docString"] = doc.clone();
        }
        if let Some(table) = &self.table {
            value["dataTable"] = table.clone();
        }
        value
    }
    fn pickle_argument(
        &self,
        replacements: &[(&String, &String)],
    ) -> Option<Value> {
        if self.doc.is_none() && self.table.is_none() {
            return None;
        }
        let mut argument = json!({});
        let both = self.doc.is_some() && self.table.is_some();
        let table_first = both
            && self.table.as_ref().unwrap()["location"]["line"].as_u64()
                < self.doc.as_ref().unwrap()["location"]["line"].as_u64();
        if let Some(doc) = &self.doc {
            let mut pickle_doc = json!({
            "content": interpolate(doc["content"].as_str().unwrap(),
             replacements)}
            );
            if let Some(media) = doc.get("mediaType") {
                pickle_doc["mediaType"] = Value::String(interpolate(
                    media.as_str().unwrap(),
                    replacements,
                ));
            }
            if both {
                pickle_doc["argumentIndex"] =
                    json!(if table_first { 2 } else { 1 });
            }
            argument["docString"] = pickle_doc;
        }
        if let Some(table) = &self.table {
            let rows = table["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let cells = row
                    ["cells"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|cell| json!({
                    "value": interpolate(cell["value"].as_str().unwrap(),
                     replacements)}
                    )).collect::<Vec<_>>();
                    json!({"cells": cells})
                })
                .collect::<Vec<_>>();
            let mut pickle_table = json!({"rows": rows});
            if both {
                pickle_table["argumentIndex"] =
                    json!(if table_first { 1 } else { 2 });
            }
            argument["dataTable"] = pickle_table;
        }
        Some(argument)
    }
}

#[derive(Debug)]
struct ExamplesNode {
    header: Header,
    rows: Vec<Value>,
}
#[derive(Debug)]
struct ScenarioNode {
    header: Header,
    steps: Vec<StepNode>,
    examples: Vec<ExamplesNode>,
}
#[derive(Debug)]
struct BackgroundNode {
    header: Header,
    steps: Vec<StepNode>,
}
#[derive(Debug)]
struct RuleNode {
    header: Header,
    background: Option<BackgroundNode>,
    scenarios: Vec<ScenarioNode>,
}
#[derive(Debug)]
struct FeatureNode {
    header: Header,
    background: Option<BackgroundNode>,
    scenarios: Vec<ScenarioNode>,
    rules: Vec<RuleNode>,
}
#[derive(Debug)]
struct SourceTree {
    uri: String,
    language: String,
    feature: FeatureNode,
    comments: Vec<Value>,
}

#[derive(Clone, Copy, Debug)]
enum Target {
    Feature,
    Rule(usize),
    Background(Option<usize>),
    Scenario(Option<usize>, usize),
    Examples(Option<usize>, usize, usize),
}

impl ExamplesNode {
    fn json(&self) -> Value {
        let mut value = self.header.json();
        value["tableBody"] =
            json!(self.rows.iter().skip(1).collect::<Vec<_>>());
        if let Some(header) = self.rows.first() {
            value["tableHeader"] = header.clone();
        }
        value
    }
}
impl ScenarioNode {
    fn json(&self) -> Value {
        let mut value = self.header.json();
        value["steps"] =
            json!(self.steps.iter().map(StepNode::json).collect::<Vec<_>>());
        value["examples"] = json!(
            self.examples.iter().map(ExamplesNode::json).collect::<Vec<_>>()
        );
        value
    }
}
impl BackgroundNode {
    fn json(&self) -> Value {
        let mut value = self.header.json();
        value.as_object_mut().unwrap().remove("tags");
        value["steps"] =
            json!(self.steps.iter().map(StepNode::json).collect::<Vec<_>>());
        value
    }
}
impl RuleNode {
    fn json(&self) -> Value {
        let mut value = self.header.json();
        let mut children = Vec::new();
        if let Some(background) = &self.background {
            children.push(json!({"background": background.json()}));
        }
        children.extend(
            self.scenarios.iter().map(|s| json!({"scenario": s.json()})),
        );
        value["children"] = json!(children);
        value
    }
}

impl SourceTree {
    fn document(&self) -> Value {
        let mut feature = self.feature.header.json();
        feature.as_object_mut().unwrap().remove("id");
        feature["language"] = json!(self.language);
        let mut children = Vec::new();
        if let Some(background) = &self.feature.background {
            children.push(json!({"background": background.json()}));
        }
        children.extend(
            self.feature
                .scenarios
                .iter()
                .map(|s| json!({"scenario": s.json()})),
        );
        children.extend(
            self.feature.rules.iter().map(|r| json!({"rule": r.json()})),
        );
        feature["children"] = json!(children);
        json!({"uri": self.uri, "feature": feature, "comments": self.comments})
    }

    fn header_mut(&mut self, target: Target) -> &mut Header {
        match target {
            Target::Feature => &mut self.feature.header,
            Target::Rule(r) => &mut self.feature.rules[r].header,
            Target::Background(r) => &mut self.background_mut(r).header,
            Target::Scenario(r, s) => &mut self.scenarios_mut(r)[s].header,
            Target::Examples(r, s, e) => {
                &mut self.scenarios_mut(r)[s].examples[e].header
            }
        }
    }
    fn scenarios_mut(&mut self, rule: Option<usize>) -> &mut Vec<ScenarioNode> {
        if let Some(rule) = rule {
            &mut self.feature.rules[rule].scenarios
        } else {
            &mut self.feature.scenarios
        }
    }
    fn background_mut(&mut self, rule: Option<usize>) -> &mut BackgroundNode {
        if let Some(rule) = rule {
            self.feature.rules[rule].background.as_mut().unwrap()
        } else {
            self.feature.background.as_mut().unwrap()
        }
    }
    fn steps_mut(
        &mut self,
        target: Target,
    ) -> Result<&mut Vec<StepNode>, Error> {
        match target {
            Target::Background(r) => Ok(&mut self.background_mut(r).steps),
            Target::Scenario(r, s) => Ok(&mut self.scenarios_mut(r)[s].steps),
            _ => Err(Error::Source(
                "step or argument outside a scenario/background".into(),
            )),
        }
    }

    fn parse(
        uri: &str,
        source: &str,
        default_language: &str,
        catalogue: DialectCatalog,
        lexical: gherkin::LexicalSemantics,
    ) -> Result<Self, Error> {
        let lines = source
            .lines()
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .collect::<Vec<_>>();
        let language = lines
            .iter()
            .take_while(|line| {
                let line = line.trim_start_matches(|character| {
                    lexical.is_whitespace(character)
                });
                line.is_empty() || line.starts_with('#')
            })
            .find_map(|line| language_directive(line, lexical))
            .unwrap_or(default_language);
        let dialect = Dialect::new(catalogue, language)?;
        let mut tree: Option<Self> = None;
        let mut target = Target::Feature;
        let mut description = None;
        let mut rule = None;
        let mut pending_tags = Vec::new();
        let mut comments = Vec::new();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            let token_trimmed = line
                .trim_start_matches(|character| lexical.is_padding(character));
            // Doc-string policy remains independent; other tokens use the
            // snapshot policy.
            let doc_trimmed = line.trim_start_matches(docstring_whitespace);
            let trimmed = if doc_trimmed.starts_with("\"\"\"")
                || doc_trimmed.starts_with("```")
            {
                doc_trimmed
            } else {
                token_trimmed
            };
            let column = line.chars().count() - trimmed.chars().count() + 1;
            let number = index + 1;
            if trimmed.starts_with('#') {
                if tree.is_some() || language_directive(line, lexical).is_none()
                {
                    comments.push(json!({
                    "location": {
                    "line": number,
                     "column": 1}
                    ,
                     "text": line}
                    ));
                }
                index += 1;
                continue;
            }
            if trimmed.starts_with('@') {
                // Official tags split on @, permitting adjacent tags. Only a
                // whitespace-prefixed # starts a trailing comment.
                let mut previous_space = false;
                let comment_start = trimmed
                    .char_indices()
                    .find_map(|(index, ch)| {
                        let found = ch == '#' && previous_space;
                        previous_space = docstring_whitespace(ch);
                        found.then_some(index)
                    })
                    .unwrap_or(trimmed.len());
                let uncommented = &trimmed[..comment_start];
                let mut tag_column = column;
                for item in uncommented.split('@') {
                    let text = item.trim_end_matches(docstring_whitespace);
                    if text.is_empty() {
                        continue;
                    }
                    if text.chars().any(docstring_whitespace) {
                        return Err(Error::Source(
                            "whitespace inside tag".into(),
                        ));
                    }
                    pending_tags.push(json!({
                    "id": node_id(uri,
                     "tag",
                     number,
                     tag_column),

                    "name": format!("@{text}"),
                         "location": {
                        "line": number,
                         "column": tag_column}
                        }
                        ));
                    tag_column += item.chars().count() + 1;
                }
                description = None;
                index += 1;
                continue;
            }
            if let Some((kind, keyword, name)) = dialect.header_line(trimmed) {
                let header = Header {
                    lexical,
                    id: node_id(uri, "node", number, column),
                    line: number,
                    column,
                    keyword: keyword.to_owned(),
                    name: name
                        .trim_matches(|character| {
                            lexical.is_whitespace(character)
                        })
                        .to_owned(),
                    description: vec![],
                    tags: std::mem::take(&mut pending_tags),
                };
                match kind {
                    HeaderKind::Feature => {
                        if tree.is_some() {
                            return Err(Error::Source(
                                "multiple features in one source".into(),
                            ));
                        }
                        tree = Some(Self {
                            uri: uri.to_owned(),
                            language: language.to_owned(),
                            feature: FeatureNode {
                                header,
                                background: None,
                                scenarios: vec![],
                                rules: vec![],
                            },
                            comments: vec![],
                        });
                        target = Target::Feature;
                    }
                    HeaderKind::Rule => {
                        let tree = tree.as_mut().ok_or_else(|| {
                            Error::Source("rule before feature".into())
                        })?;
                        let r = tree.feature.rules.len();
                        tree.feature.rules.push(RuleNode {
                            header,
                            background: None,
                            scenarios: vec![],
                        });
                        rule = Some(r);
                        target = Target::Rule(r);
                    }
                    HeaderKind::Background => {
                        let tree = tree.as_mut().ok_or_else(|| {
                            Error::Source("background before feature".into())
                        })?;
                        let slot = if let Some(r) = rule {
                            &mut tree.feature.rules[r].background
                        } else {
                            &mut tree.feature.background
                        };
                        if slot.is_some() || !header.tags.is_empty() {
                            return Err(Error::Source(
                                "invalid background declaration".into(),
                            ));
                        }
                        *slot = Some(BackgroundNode { header, steps: vec![] });
                        target = Target::Background(rule);
                    }
                    HeaderKind::Examples => {
                        let (r, s) = match target {
                            Target::Scenario(r, s)
                            | Target::Examples(r, s, _) => (r, s),
                            _ => {
                                return Err(Error::Source(
                                    "examples outside scenario".into(),
                                ));
                            }
                        };
                        let tree = tree.as_mut().unwrap();
                        let examples = &mut tree.scenarios_mut(r)[s].examples;
                        let e = examples.len();
                        examples.push(ExamplesNode { header, rows: vec![] });
                        target = Target::Examples(r, s, e);
                    }
                    HeaderKind::Scenario => {
                        let tree = tree.as_mut().ok_or_else(|| {
                            Error::Source("scenario before feature".into())
                        })?;
                        let scenarios = tree.scenarios_mut(rule);
                        let s = scenarios.len();
                        scenarios.push(ScenarioNode {
                            header,
                            steps: vec![],
                            examples: vec![],
                        });
                        target = Target::Scenario(rule, s);
                    }
                }
                description = Some(target);
                index += 1;
                continue;
            }
            if let Some((keyword, keyword_type, text)) =
                dialect.step_line(trimmed)
            {
                let tree = tree.as_mut().ok_or_else(|| {
                    Error::Source("step before feature".into())
                })?;
                tree.steps_mut(target)?.push(StepNode {
                    id: node_id(uri, "step", number, column),
                    line: number,
                    column,
                    keyword: keyword.to_owned(),
                    keyword_type,
                    text: text
                        .trim_matches(|character| {
                            lexical.is_whitespace(character)
                        })
                        .to_owned(),
                    doc: None,
                    table: None,
                });
                description = None;
                index += 1;
                continue;
            }
            if trimmed.starts_with('|') {
                let row = table_row(uri, line, number)?;
                let tree = tree.as_mut().ok_or_else(|| {
                    Error::Source("table before feature".into())
                })?;
                if let Target::Examples(r, s, e) = target {
                    tree.scenarios_mut(r)[s].examples[e].rows.push(row);
                } else {
                    let step = tree.steps_mut(target)?.last_mut().ok_or_else(
                        || Error::Source("table without step".into()),
                    )?;
                    let table = step.table.get_or_insert_with(|| {
                        json!({
                        "location": {
                        "line": number,
                         "column": column}
                        ,
                         "rows": []}
                        )
                    });
                    table["rows"].as_array_mut().unwrap().push(row);
                }
                description = None;
                index += 1;
                continue;
            }
            if trimmed.starts_with("\"\"\"") || trimmed.starts_with("```") {
                let delimiter = if trimmed.starts_with("\"\"\"") {
                    "\"\"\""
                } else {
                    "```"
                };
                let media_type =
                    trimmed[3..].trim_matches(docstring_whitespace);
                let mut content = Vec::new();
                index += 1;
                while index < lines.len()
                    && !lines[index]
                        .trim_start_matches(docstring_whitespace)
                        .starts_with(delimiter)
                {
                    let line = lines[index];
                    let remove = line
                        .chars()
                        .take(column - 1)
                        .take_while(|c| docstring_whitespace(*c))
                        .count();
                    let text = line.chars().skip(remove).collect::<String>();
                    content.push(if delimiter == "\"\"\"" {
                        text.replacen("\\\"\\\"\\\"", "\"\"\"", 1)
                    } else {
                        text.replacen("\\`\\`\\`", "```", 1)
                    });
                    index += 1;
                }
                if index == lines.len() {
                    return Err(Error::Source(
                        "unterminated doc string".into(),
                    ));
                }
                let mut doc = json!({
                "location": {
                "line": number,
                 "column": column}
                ,
                 "delimiter": delimiter,
                 "content": content.join("\n")}
                );
                if !media_type.is_empty() {
                    doc["mediaType"] = json!(media_type);
                }
                let tree = tree.as_mut().ok_or_else(|| {
                    Error::Source("doc string before feature".into())
                })?;
                let step =
                    tree.steps_mut(target)?.last_mut().ok_or_else(|| {
                        Error::Source("doc string without step".into())
                    })?;
                if step.doc.replace(doc).is_some() {
                    return Err(Error::Source("duplicate doc string".into()));
                }
                description = None;
                index += 1;
                continue;
            }
            if let Some(target) = description {
                tree.as_mut()
                    .unwrap()
                    .header_mut(target)
                    .description
                    .push(line.to_owned());
            } else if !trimmed.is_empty() {
                return Err(Error::Source(format!(
                    "unrepresented source content at line {number}"
                )));
            }
            index += 1;
        }
        let mut tree = tree.ok_or_else(|| {
            Error::Source("source contains no feature".into())
        })?;
        if !pending_tags.is_empty() {
            return Err(Error::Source("dangling tags".into()));
        }
        tree.comments = comments;
        Ok(tree)
    }
}

fn language_directive(
    line: &str,
    lexical: gherkin::LexicalSemantics,
) -> Option<&str> {
    let whitespace = |character| lexical.is_whitespace(character);
    let comment = line
        .trim_start_matches(whitespace)
        .strip_prefix('#')?
        .trim_start_matches(whitespace);
    Some(
        comment
            .strip_prefix("language")?
            .trim_start_matches(whitespace)
            .strip_prefix(':')?
            .trim_matches(whitespace),
    )
}

#[derive(Clone, Copy, Debug)]
enum HeaderKind {
    Feature,
    Rule,
    Background,
    Scenario,
    Examples,
}

struct Dialect {
    headers: Vec<(HeaderKind, &'static str)>,
    steps: Vec<(&'static str, &'static str)>,
}

impl Dialect {
    fn new(catalogue: DialectCatalog, language: &str) -> Result<Self, Error> {
        // These are the same immutable files compiled by the vendored parser.
        // Never merge latest aliases into the legacy compatibility catalogue.
        static LEGACY: LazyLock<Value> = LazyLock::new(|| {
            serde_json::from_str(include_str!(
                "../../../vendor/gherkin/src/languages.json"
            ))
            .expect("embedded legacy catalogue")
        });
        static LATEST: LazyLock<Value> = LazyLock::new(|| {
            serde_json::from_str(include_str!(
                "../../../vendor/gherkin/src/languages-43.json"
            ))
            .expect("embedded Gherkin 43 catalogue")
        });
        let registry: &'static Value = match catalogue {
            DialectCatalog::Legacy => &LEGACY,
            DialectCatalog::Gherkin43 => &LATEST,
        };
        let dialect = registry.get(language).ok_or_else(|| {
            Error::Source(format!(
                "dialect {language:?} is absent from the captured catalogue"
            ))
        })?;
        let keywords = |key: &str| {
            dialect[key]
                .as_array()
                .expect("catalogue keyword array")
                .iter()
                .map(|value| value.as_str().expect("catalogue keyword"))
                .collect::<Vec<_>>()
        };
        let mut headers = Vec::new();
        for (key, kind) in [
            ("feature", HeaderKind::Feature),
            ("rule", HeaderKind::Rule),
            ("background", HeaderKind::Background),
            ("scenario", HeaderKind::Scenario),
            ("scenarioOutline", HeaderKind::Scenario),
            ("examples", HeaderKind::Examples),
        ] {
            headers.extend(keywords(key).into_iter().map(|word| (kind, word)));
        }
        headers
            .sort_by_key(|(_, word)| std::cmp::Reverse(word.chars().count()));
        let mut mappings: HashMap<&'static str, Vec<&'static str>> =
            HashMap::new();
        for (key, kind) in [
            ("given", "Context"),
            ("when", "Action"),
            ("then", "Outcome"),
            ("and", "Conjunction"),
            ("but", "Conjunction"),
        ] {
            for word in keywords(key) {
                mappings.entry(word).or_default().push(kind);
            }
        }
        // The official matcher marks any multiply registered spelling Unknown,
        // even if two registrations happen to have the same semantic type.
        let mut steps = mappings
            .into_iter()
            .map(|(word, kinds)| {
                (word, if kinds.len() == 1 { kinds[0] } else { "Unknown" })
            })
            .collect::<Vec<_>>();
        steps.sort_by_key(|(word, _)| std::cmp::Reverse(word.chars().count()));
        Ok(Self { headers, steps })
    }

    fn header_line<'a>(
        &self,
        line: &'a str,
    ) -> Option<(HeaderKind, &'static str, &'a str)> {
        self.headers.iter().find_map(|(kind, keyword)| {
            line.strip_prefix(*keyword)
                .and_then(|rest| rest.strip_prefix(':'))
                .map(|name| (*kind, *keyword, name))
        })
    }

    fn step_line<'a>(
        &self,
        line: &'a str,
    ) -> Option<(&'static str, &'static str, &'a str)> {
        self.steps.iter().find_map(|(keyword, kind)| {
            line.strip_prefix(*keyword).map(|text| (*keyword, *kind, text))
        })
    }
}

fn node_id(uri: &str, kind: &str, line: usize, column: usize) -> String {
    format!("{uri}#{kind}-{line}-{column}")
}

fn table_row(uri: &str, line: &str, number: usize) -> Result<Value, Error> {
    let chars = line.chars().collect::<Vec<_>>();
    let first = chars
        .iter()
        .position(|c| *c == '|')
        .ok_or_else(|| Error::Source("table row lacks separator".into()))?;
    let mut cells = Vec::new();
    let mut start = first + 1;
    let mut index = start;
    while index < chars.len() {
        if chars[index] == '\\'
            && index + 1 < chars.len()
            && matches!(chars[index + 1], '\\' | '|' | 'n')
        {
            index += 2;
            continue;
        }
        if chars[index] == '|' {
            let raw = chars[start..index].iter().collect::<String>();
            let trimmed = raw.trim_matches(table_whitespace);
            let leading = raw
                .chars()
                .take_while(|c| table_whitespace(*c))
                .map(char::len_utf16)
                .sum::<usize>();
            // GherkinLine counts indentation as scalars, then advances the raw
            // table prefix in JavaScript string units. Keep escaped delimiters
            // in that prefix; decoded cell values cannot recover source
            // columns.
            let column = first
                + chars[first..start]
                    .iter()
                    .map(|character| character.len_utf16())
                    .sum::<usize>()
                + leading
                + 1;
            cells.push(json!({
            "location": {
            "line": number,
             "column": column}
            ,
             "value": unescape_cell(trimmed)}
            ));
            start = index + 1;
        }
        index += 1;
    }
    // Official table cells end at a pipe; trailing text is not a cell.
    Ok(json!({
    "id": node_id(uri,
     "row",
     number,
     first + 1),
     "location": {
    "line": number,
     "column": first + 1}
    ,
     "cells": cells}
    ))
}

fn unescape_cell(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('|') => {
                    chars.next();
                    out.push('|');
                }
                Some('\\') => {
                    chars.next();
                    out.push('\\');
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// Exact ECMAScript whitespace, as used by Gherkin 43's JavaScript token
// matcher.
// Rust is_whitespace includes NEL and excludes BOM, unlike that source
// contract.
fn docstring_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn table_whitespace(character: char) -> bool {
    matches!(
        character,
        ' ' | '\t' | '\u{000b}' | '\u{000c}' | '\r' | '\u{0085}' | '\u{00a0}'
    )
}
