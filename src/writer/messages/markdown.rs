//! Adapter for the standalone native Markdown source model.
//! The classic source scanner is deliberately not involved.
use super::*;
use crate::gherkin::markdown as md;

impl SourceGraph {
    /// Parses original Markdown using the native Gherkin43 matcher and
    /// compiler.
    ///
    /// The configured dialect applies globally; Markdown has no language
    /// directive.
    /// No Node runtime or filesystem access is involved.
    ///
    /// # Errors
    /// Returns positioned native Markdown errors or source/projection
    /// mismatches.
    pub fn parse_markdown_gherkin43(
        uri: &str,
        source: &str,
        default_language: &str,
    ) -> Result<Self, Error> {
        let document = md::MarkdownDocument::parse(source, default_language)
            .map_err(|e| Error::Source(e.to_string()))?;
        Self::from_markdown_document(uri, &document)
    }

    /// Converts a typed native Markdown document without reading its source
    /// again.
    ///
    /// # Errors
    /// Rejects inconsistent public source nodes or native projection/compiler
    /// errors.
    pub fn from_markdown_document(
        uri: &str,
        document: &md::MarkdownDocument,
    ) -> Result<Self, Error> {
        let Some(feature) = &document.feature else {
            return Ok(Self {
                document: json!({"uri":uri,"comments":[]}),
                language: document.language.clone(),
                pickles: vec![],
                cases: vec![],
            });
        };
        let tree = SourceTree {
            uri: uri.into(),
            language: document.language.clone(),
            comments: vec![],
            feature: FeatureNode {
                header: header(uri, &feature.header),
                background: feature
                    .background
                    .as_ref()
                    .map(|b| background(uri, b)),
                scenarios: feature
                    .scenarios
                    .iter()
                    .map(|s| scenario(uri, s))
                    .collect(),
                rules: feature
                    .rules
                    .iter()
                    .map(|r| RuleNode {
                        header: header(uri, &r.header),
                        background: r
                            .background
                            .as_ref()
                            .map(|b| background(uri, b)),
                        scenarios: r
                            .scenarios
                            .iter()
                            .map(|s| scenario(uri, s))
                            .collect(),
                    })
                    .collect(),
            },
        };
        let native = document.to_feature().expect("typed Feature projects");
        let mut graph = Self::from_tree(
            uri,
            tree,
            &native,
            crate::feature::ExampleSemantics::Gherkin43,
        )?;
        if feature.header.keyword.is_none() {
            graph.document["feature"]
                .as_object_mut()
                .unwrap()
                .remove("keyword");
        }
        Ok(graph)
    }
}
fn header(uri: &str, h: &md::Header) -> Header {
    Header {
        lexical: gherkin::LexicalSemantics::Legacy,
        id: node_id(uri, "node", h.position.line, h.position.col),
        line: h.position.line,
        column: h.position.col,
        keyword: h.keyword.clone().unwrap_or_default(),
        name: h.name.clone(),
        description: vec![],
        tags: h
            .tags
            .iter()
            .map(|t| {
                json!({
                "id":node_id(uri,
                "tag",
                t.position.line,
                t.position.col),
                "name":t.name,
                "location":{
                "line":t.position.line,
                "column":t.position.col}
                }
                )
            })
            .collect(),
    }
}
fn rows(uri: &str, rows: &[md::Row]) -> Vec<Value> {
    rows.iter()
        .map(|r| {
            json!({
            "id":node_id(uri,
            "row",
            r.position.line,
            r.position.col),
            "location":{
            "line":r.position.line,
            "column":r.position.col}
            ,
            "cells":r.cells.iter().map(|c|json!({
            "value":c.value,
            "location":{
            "line":c.position.line,
            "column":c.position.col}
            }
            )).collect::<Vec<_>>()}
            )
        })
        .collect()
}
fn steps(uri: &str, steps: &[md::Step]) -> Vec<StepNode> {
    steps
        .iter()
        .map(|s| {
            let doc = s.doc_string.as_ref().map(|d| {
                let mut v = json!({
                "location":{
                "line":d.position.line,
                "column":d.position.col}
                ,
                "content":d.content,
                "delimiter":d.delimiter}
                );
                if let Some(media) = &d.media_type {
                    v["mediaType"] = json!(media);
                }
                v
            });
            let table = s.table.first().map(|r| {
                json!({
                "location":{
                "line":r.position.line,
                "column":r.position.col}
                ,
                "rows":rows(uri,
                &s.table)}
                )
            });
            StepNode {
                id: node_id(uri, "step", s.position.line, s.position.col),
                line: s.position.line,
                column: s.position.col,
                keyword: s.keyword.clone(),
                keyword_type: match s.keyword_type {
                    md::KeywordType::Context => "Context",
                    md::KeywordType::Action => "Action",
                    md::KeywordType::Outcome => "Outcome",
                    md::KeywordType::Conjunction => "Conjunction",
                    md::KeywordType::Unknown => "Unknown",
                },
                text: s.text.clone(),
                doc,
                table,
            }
        })
        .collect()
}
fn background(uri: &str, b: &md::Background) -> BackgroundNode {
    BackgroundNode {
        header: header(uri, &b.header),
        steps: steps(uri, &b.steps),
    }
}
fn scenario(uri: &str, s: &md::Scenario) -> ScenarioNode {
    ScenarioNode {
        header: header(uri, &s.header),
        steps: steps(uri, &s.steps),
        examples: s
            .examples
            .iter()
            .map(|e| ExamplesNode {
                header: header(uri, &e.header),
                rows: rows(uri, &e.rows),
            })
            .collect(),
    }
}
