//! Actual expression provenance and validated argument capture layouts.

use std::fmt;

use regex::Regex;
use regex_syntax::ast::{Ast, parse::Parser};

/// A known parameter descriptor retained with the expression that uses it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDescriptor {
    /// Actual parameter type name, including the empty anonymous name.
    pub name: String,
    /// Pinned source regular expressions for this parameter type.
    pub regular_expressions: Vec<String>,
    /// Known preference in the explicitly selected expression vocabulary.
    pub prefer_for_regular_expression_match: bool,
    /// Known snippet eligibility in the selected expression vocabulary.
    pub use_for_snippets: bool,
}

/// The observed untransformed group used by a step argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentGroup {
    /// UTF-16 start offset, absent for an unmatched optional capture.
    pub start: Option<usize>,
    /// Raw captured text, absent for an unmatched optional capture.
    pub value: Option<String>,
    /// Real nested capture groups; absent for a leaf group.
    pub children: Option<Vec<ArgumentGroup>>,
}

/// One observed argument and its actual parameter name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArgument {
    /// The actual outer parameter capture and nested groups.
    pub group: ArgumentGroup,
    /// Known expression parameter name.
    pub parameter_type_name: String,
}

#[derive(Clone, Debug)]
struct GroupLayout {
    index: usize,
    children: Vec<Self>,
}

/// Original expression plus its generated, structurally faithful capture layout.
///
/// The handler matcher is unchanged. Metadata uses real regex capture groups,
/// validates against the handler match, and never infers nesting from flat ranges.
#[derive(Clone, Debug)]
pub struct ExpressionMetadata {
    source: String,
    expected_runtime: Regex,
    matcher: Regex,
    parameters: Vec<ParameterDescriptor>,
    groups: Vec<GroupLayout>,
}

/// A known raw regex and explicit mapping from captures to handler arguments.
///
/// This declares a native mapping; it does not infer JavaScript parameter types
/// or change how the callback consumes its context. Compile with this factory
/// and register its [`Self::regex`] to avoid guessing external builder settings.
#[derive(Clone, Debug)]
pub struct RegexMetadata {
    expected_runtime: Regex,
    arguments: Vec<GroupLayout>,
}

impl RegexMetadata {
    /// Compiles a pattern under default settings and retains its actual AST.
    ///
    /// Inline settings are part of the pattern. Each supplied index identifies
    /// an actual capture, in argument order; zero selects the whole match.
    /// Repeated indices intentionally repeat that real capture as an argument.
    ///
    /// # Errors
    /// Rejects invalid regexes and indices outside the real capture set.
    pub fn new(pattern: &str, argument_capture_indices: impl IntoIterator<Item = usize>)
        -> Result<Self, MetadataError>
    {
        let expected_runtime = Regex::new(pattern).map_err(|error|
            MetadataError(format!("raw regex metadata: {error}")))?;
        let ast = Parser::new().parse(pattern).map_err(|error|
            MetadataError(format!("raw regex metadata: {error}")))?;
        let root = GroupLayout { index: 0, children: layout(&ast) };
        let arguments = argument_capture_indices.into_iter().map(|index| {
            find_group(&root, index).cloned().ok_or_else(||
                MetadataError(format!("raw regex metadata capture index {index} does not exist")))
        }).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { expected_runtime, arguments })
    }

    /// The actual regex compiled with the settings this descriptor records.
    #[must_use]
    pub fn regex(&self) -> &Regex { &self.expected_runtime }

    /// Returns real argument groups using the caller's explicit capture mapping.
    ///
    /// # Errors
    /// Rejects conflicting runtime settings or inconsistent observed spans.
    pub fn arguments(&self, actual: &Regex, text: &str) -> Result<Vec<ArgumentGroup>, MetadataError> {
        let captures = runtime_captures(actual, &self.expected_runtime, text,
            "raw regex metadata does not match the actual registered matcher")?;
        let span = captures.get(0).ok_or_else(||
            MetadataError("raw regex metadata has no whole match".to_owned()))?.range();
        self.arguments.iter().map(|group| observe(group, &captures, text, Some(span.clone()))).collect()
    }
}

/// Inconsistent explicit expression metadata cannot be reported faithfully.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataError(pub(crate) String);
impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
}
impl std::error::Error for MetadataError {}

impl ExpressionMetadata {
    /// Constructs a descriptor emitted by the expression macro implementation.
    ///
    /// This is generated-code glue. A supplied descriptor is checked against
    /// the actual runtime regex and observed captures before reporting.
    ///
    /// # Errors
    /// Rejects malformed regexes or a layout without one outer capture per type.
    #[doc(hidden)]
    pub fn from_generated(source: String, runtime_pattern: &str, capture_pattern: &str,
        parameters: Vec<ParameterDescriptor>) -> Result<Self, MetadataError>
    {
        let error = |message| MetadataError(format!("expression metadata: {message}"));
        let expected_runtime = Regex::new(runtime_pattern).map_err(|e| error(e.to_string()))?;
        let matcher = Regex::new(capture_pattern).map_err(|e| error(e.to_string()))?;
        let ast = Parser::new().parse(capture_pattern).map_err(|e| error(e.to_string()))?;
        let groups = layout(&ast);
        if groups.len() != parameters.len() {
            return Err(error("outer capture count does not match parameter count".to_owned()));
        }
        if parameters.iter().any(|p| p.regular_expressions.is_empty()) {
            return Err(error("parameter descriptor has no source regex".to_owned()));
        }
        Ok(Self { source, expected_runtime, matcher, parameters, groups })
    }

    /// The actual original Cucumber Expression retained at registration.
    #[must_use]
    pub fn source(&self) -> &str { &self.source }

    /// Parameter descriptors in argument order, including repeated types.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterDescriptor] { &self.parameters }

    /// Resolves the raw argument tree after verifying the actual handler match.
    ///
    /// Unmatched captures omit text and start; matched empty groups retain both.
    /// Byte positions are converted to JavaScript UTF-16 string positions.
    ///
    /// # Errors
    /// Fails visibly if explicit metadata does not describe the registered
    /// matcher, if observed captures disagree, or if the group spans are invalid.
    pub fn arguments(&self, actual: &Regex, text: &str) -> Result<Vec<MatchArgument>, MetadataError> {
        let mismatch = || MetadataError("expression metadata does not match the actual registered matcher".to_owned());
        let observed = runtime_captures(actual, &self.expected_runtime, text,
            "expression metadata does not match the actual registered matcher")?;
        let captures = self.matcher.captures(text).ok_or_else(mismatch)?;
        let span = observed.get(0).ok_or_else(mismatch)?.range();
        if captures.get(0).map(|m| m.range()) != Some(span.clone()) { return Err(mismatch()); }
        self.groups.iter().zip(&self.parameters).map(|(group, parameter)| {
            Ok(MatchArgument { group: observe(group, &captures, text, Some(span.clone()))?,
                parameter_type_name: parameter.name.clone() })
        }).collect()
    }
}

fn runtime_captures<'text>(actual: &Regex, expected_runtime: &Regex, text: &'text str,
    message: &str) -> Result<regex::Captures<'text>, MetadataError>
{
    let mismatch = || MetadataError(message.to_owned());
    if actual.as_str() != expected_runtime.as_str() { return Err(mismatch()); }
    let observed = actual.captures(text).ok_or_else(mismatch)?;
    let expected = expected_runtime.captures(text).ok_or_else(mismatch)?;
    if observed.len() != expected.len() || (0..observed.len()).any(|index| {
        observed.get(index).map(|m| m.range()) != expected.get(index).map(|m| m.range())
    }) { return Err(mismatch()); }
    Ok(observed)
}

fn find_group(root: &GroupLayout, index: usize) -> Option<&GroupLayout> {
    if root.index == index { Some(root) }
    else { root.children.iter().find_map(|child| find_group(child, index)) }
}

fn layout(ast: &Ast) -> Vec<GroupLayout> {
    match ast {
        Ast::Group(group) => {
            let children = layout(&group.ast);
            if let Some(index) = group.capture_index() {
                vec![GroupLayout { index: index as usize, children }]
            } else { children }
        }
        Ast::Concat(concat) => concat.asts.iter().flat_map(layout).collect(),
        Ast::Alternation(alternation) => alternation.asts.iter().flat_map(layout).collect(),
        Ast::Repetition(repetition) => layout(&repetition.ast),
        _ => Vec::new(),
    }
}

fn observe(layout: &GroupLayout, captures: &regex::Captures<'_>, text: &str,
    parent: Option<std::ops::Range<usize>>) -> Result<ArgumentGroup, MetadataError>
{
    let observed = captures.get(layout.index);
    let range = observed.as_ref().map(|m| m.range());
    if let Some(range) = &range {
        if !parent.as_ref().is_some_and(|p| p.start <= range.start && range.end <= p.end)
            || !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
            return Err(MetadataError("expression metadata capture is outside its actual parent span".to_owned()));
        }
    }
    let children = if layout.children.is_empty() { None } else {
        Some(layout.children.iter().map(|child| observe(child, captures, text, range.clone()))
            .collect::<Result<Vec<_>, _>>()?)
    };
    Ok(ArgumentGroup {
        start: range.as_ref().map(|range| text[..range.start].encode_utf16().count()),
        value: observed.map(|m| m.as_str().to_owned()), children,
    })
}
