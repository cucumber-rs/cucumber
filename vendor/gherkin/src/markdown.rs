//! Native Gherkin-in-Markdown source model and parser, pinned to Gherkin 43.
//!
//! This is an alternate Gherkin token matcher, not a CommonMark renderer.
//! Original coordinates and argument metadata survive native Feature projection.
use crate::{keywords::Keywords, LineCol, Span, StepType};
use std::path::Path;

/// A source-positioned declaration. Only the fallback Feature lacks a keyword.
#[derive(Clone, Debug)]
pub struct Header {
    /// Actual keyword, absent for the Markdown fallback Feature title.
    pub keyword: Option<String>,
    /// Declaration name.
    pub name: String,
    /// Original keyword location.
    pub position: LineCol,
    /// Original line byte range.
    pub span: Span,
    /// Source tags, including their locations.
    pub tags: Vec<Tag>,
}
/// A source tag without generated protocol identity.
#[derive(Clone, Debug)]
pub struct Tag {
    /// Tag text, including its leading @.
    pub name: String,
    /// Original tag location after its opening backtick.
    pub position: LineCol,
}
/// A source table cell.
#[derive(Clone, Debug)]
pub struct Cell {
    /// Unescaped, trimmed cell value.
    pub value: String,
    /// Original first-value location.
    pub position: LineCol,
}
/// A retained table row (Markdown separator rows are omitted).
#[derive(Clone, Debug)]
pub struct Row {
    /// Cells in source order.
    pub cells: Vec<Cell>,
    /// Original leading pipe location.
    pub position: LineCol,
    /// Original line byte range.
    pub span: Span,
}
/// Exact Markdown doc-string semantics, independent of legacy classic behavior.
#[derive(Clone, Debug)]
pub struct DocString {
    /// Dedented content, with literal backticks preserved.
    pub content: String,
    /// Exact active backtick run.
    pub delimiter: String,
    /// Untrimmed nonempty opening-fence suffix.
    pub media_type: Option<String>,
    /// Original opening-fence location.
    pub position: LineCol,
}
/// A source step; argument order is retained by original argument locations.
#[derive(Clone, Debug)]
pub struct Step {
    /// Original keyword, including trailing keyword whitespace.
    pub keyword: String,
    /// Official source classification.
    pub keyword_type: KeywordType,
    /// Trimmed step expression.
    pub text: String,
    /// Original keyword location after the bullet.
    pub position: LineCol,
    /// Original step-line byte range.
    pub span: Span,
    /// Optional source doc string.
    pub doc_string: Option<DocString>,
    /// Retained data-table rows.
    pub table: Vec<Row>,
}
/// Official semantic keyword classification, separate from native StepType.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordType {
    /// A Given keyword.
    Context,
    /// A When keyword.
    Action,
    /// A Then keyword.
    Outcome,
    /// An And/But keyword.
    Conjunction,
    /// A spelling shared by multiple keyword categories.
    Unknown,
}
/// An Examples declaration and its retained table rows.
#[derive(Clone, Debug)]
pub struct Examples {
    /// Examples declaration.
    pub header: Header,
    /// Header followed by body rows.
    pub rows: Vec<Row>,
}
/// A scenario or outline, retaining unexpanded source identity.
#[derive(Clone, Debug)]
pub struct Scenario {
    /// Scenario declaration.
    pub header: Header,
    /// Source steps.
    pub steps: Vec<Step>,
    /// Examples blocks.
    pub examples: Vec<Examples>,
}
/// A background declaration.
#[derive(Clone, Debug)]
pub struct Background {
    /// Background declaration.
    pub header: Header,
    /// Source steps.
    pub steps: Vec<Step>,
}
/// A rule and its children in grammar order.
#[derive(Clone, Debug)]
pub struct Rule {
    /// Rule declaration.
    pub header: Header,
    /// Optional rule background.
    pub background: Option<Background>,
    /// Rule scenarios.
    pub scenarios: Vec<Scenario>,
}
/// A source Feature, independent of the legacy native projection.
#[derive(Clone, Debug)]
pub struct Feature {
    /// Feature declaration or fallback title.
    pub header: Header,
    /// Optional feature background.
    pub background: Option<Background>,
    /// Scenarios preceding rules.
    pub scenarios: Vec<Scenario>,
    /// Rules in source order.
    pub rules: Vec<Rule>,
}
/// Parsed Markdown document. No Messages, JSON or Node runtime is required.
#[derive(Clone, Debug)]
pub struct MarkdownDocument {
    /// Caller-selected global dialect.
    pub language: String,
    /// Optional source Feature.
    pub feature: Option<Feature>,
}
/// A positioned Markdown parse error. Diagnostic prose is not a compatibility ABI.
#[derive(Debug, thiserror::Error)]
#[error("Markdown Gherkin at {}:{}: {message}", .position.line, .position.col)]
pub struct Error {
    /// Original source position.
    pub position: LineCol,
    /// Human-readable failure description.
    pub message: String,
}
fn error(line: usize, message: impl Into<String>) -> Error { Error { position: LineCol { line, col: 1 }, message: message.into() } }
fn ws(c: char) -> bool { matches!(c, '\t'|'\n'|'\u{b}'|'\u{c}'|'\r'|' '|'\u{a0}'|'\u{1680}'|'\u{2000}'..='\u{200a}'|'\u{2028}'|'\u{2029}'|'\u{202f}'|'\u{205f}'|'\u{3000}'|'\u{feff}') }
fn table_ws(c: char) -> bool { matches!(c, ' '|'\t'|'\u{b}'|'\u{c}'|'\r'|'\u{85}'|'\u{a0}') }
#[derive(Clone, Copy)]
enum Target { Feature, Rule, Background(Option<usize>), Scenario(Option<usize>, usize), Examples(Option<usize>, usize, usize) }
#[derive(Clone, Copy, PartialEq)]
enum Kind { Feature, Rule, Background, Scenario, Examples }
impl Feature {
    fn scenarios(&mut self, rule: Option<usize>) -> &mut Vec<Scenario> { match rule { Some(r)=>&mut self.rules[r].scenarios, None=>&mut self.scenarios } }
    fn background(&mut self, rule: Option<usize>) -> &mut Option<Background> { match rule { Some(r)=>&mut self.rules[r].background, None=>&mut self.background } }
    fn last_step(&self, target: Target) -> Option<&Step> {
        match target {
            Target::Background(rule) => {
                let background=match rule {
                    Some(r) => self.rules[r].background.as_ref(), None => self.background.as_ref(),
                };
                background.and_then(|background| background.steps.last())
            },
            Target::Scenario(rule, scenario) => {
                let scenarios=match rule {
                    Some(r) => &self.rules[r].scenarios, None => &self.scenarios,
                };
                scenarios[scenario].steps.last()
            },
            _ => None,
        }
    }
    fn steps(&mut self, target: Target, line: usize) -> Result<&mut Vec<Step>,Error> {
        match target { Target::Background(r)=>Ok(&mut self.background(r).as_mut().unwrap().steps), Target::Scenario(r,s)=>Ok(&mut self.scenarios(r)[s].steps), _=>Err(error(line,"step or argument outside scenario/background")) }
    }
}
fn tags(text: &str, line: usize, indent: usize) -> Vec<Tag> {
    let mut result=Vec::new();let mut from=0;
    while let Some(relative)=text[from..].find("`@") {
        let start=from+relative;let tail=&text[start+2..];
        if let Some(end)=tail.find('`') {
            if end>0 {result.push(Tag {name:text[start+1..start+2+end].into(),position:LineCol {line,col:indent+text[..start].encode_utf16().count()+2}});}
            from=start+3+end;
        } else {break;}
    }result
}
fn title<'a>(text: &'a str, k: &'a Keywords<'a>) -> Option<(Kind,&'a str,String,usize)> {
    let hashes=text.chars().take_while(|c|*c=='#').count();
    if !(1..=6).contains(&hashes) {return None;}
    let gap=text[hashes..].chars().next()?;if !ws(gap) {return None;}
    let prefix=hashes+gap.len_utf8();let rest=&text[prefix..];
    for (kind,words) in [(Kind::Feature,k.feature),(Kind::Background,k.background),(Kind::Rule,k.rule),(Kind::Scenario,k.scenario),(Kind::Scenario,k.scenario_outline),(Kind::Examples,k.examples)] {
        for word in words {if let Some(name)=rest.strip_prefix(word).and_then(|v|v.strip_prefix(':')) {return Some((kind,word,name.trim_matches(ws).into(),hashes+1));}}
    }None
}
fn step<'a>(text: &'a str, k: &'a Keywords<'a>) -> Option<(&'a str,KeywordType,String,usize)> {
    let marker=text.chars().next()?;if !matches!(marker,'*'|'+'|'-') {return None;}
    let rest=&text[marker.len_utf8()..];let trimmed=rest.trim_start_matches(ws);
    let prefix=1+rest.chars().count()-trimmed.chars().count();
    let lists=[(k.given,KeywordType::Context),(k.when,KeywordType::Action),(k.then,KeywordType::Outcome),(k.and,KeywordType::Conjunction),(k.but,KeywordType::Conjunction)];
    let word=lists.iter().flat_map(|(list,_)|list.iter().copied()).filter(|w|*w!="* "&&trimmed.starts_with(*w)).max_by_key(|w|w.encode_utf16().count())?;
    let kinds=lists.iter().filter(|(list,_)|list.contains(&word)).map(|(_,kind)|*kind).collect::<Vec<_>>();
    Some((word,if kinds.len()==1{kinds[0]}else{KeywordType::Unknown},trimmed[word.len()..].trim_matches(ws).into(),prefix))
}
fn row(text: &str, number: usize, indent: usize, span: Span) -> Row {
    let mut cells=Vec::new();let mut value=String::new();let mut chars=text.chars().peekable();let mut col=0;let mut start=1;let mut first=true;
    while let Some(c)=chars.next() {col+=c.len_utf16();match c {
        '|'=>{if first {first=false;} else {let left=value.trim_start_matches(table_ws);cells.push(Cell {value:left.trim_end_matches(table_ws).into(),position:LineCol {line:number,col:indent+start+value.encode_utf16().count()-left.encode_utf16().count()}});}value.clear();start=col+1;},
        '\\'=>if let Some(next)=chars.next(){col+=next.len_utf16();match next {'n'=>value.push('\n'),'|'|'\\'=>value.push(next),_=>{value.push('\\');value.push(next);}}}else{value.push('\\');},
        _=>value.push(c),
    }}
    Row {cells,position:LineCol{line:number,col:indent+1},span}
}
fn separator(row: &Row)->bool {row.cells.iter().any(|c| {let s=c.value.strip_prefix(':').unwrap_or(&c.value);let s=s.strip_suffix(':').unwrap_or(s);!s.is_empty()&&s.chars().all(|c|c=='-')})}
fn add_row(rows:&mut Vec<Row>,row:Row)->Result<(),Error>{if rows.first().is_some_and(|r|r.cells.len()!=row.cells.len()){return Err(error(row.position.line,"inconsistent cell count within the table"));}rows.push(row);Ok(())}

impl MarkdownDocument {
    /// Parses the original Markdown bytes using the official43 dialect catalogue.
    pub fn parse(source: &str, default_language: &str) -> Result<Self,Error> {
        let keywords=Keywords::get_latest(default_language).ok_or_else(||error(1,format!("unsupported language {default_language}")))?;
        let mut document=Self {language:default_language.into(),feature:None};
        let raw_lines=source.split('\n').collect::<Vec<_>>();
        let mut lines=raw_lines.iter().enumerate().map(|(i,l)| if i+1<raw_lines.len(){l.strip_suffix('\r').unwrap_or(l)}else{*l}).collect::<Vec<_>>();
        // TokenScanner discards exactly one final whitespace-only line.
        if lines.last().is_some_and(|line|line.trim_matches(ws).is_empty()){lines.pop();}
        let mut offsets=Vec::new();let mut offset=0;for line in source.split_inclusive('\n'){offsets.push(offset);offset+=line.len();}
        let mut target=Target::Feature;let mut rule=None;let mut pending=Vec::new();let mut index=0;
        let mut last_argument_table=false;
        while index<lines.len() {
            let line=lines[index];let number=index+1;let trimmed=line.trim_start_matches(ws);let indent=line.chars().count()-trimmed.chars().count();
            let span=Span {start:offsets[index],end:offsets[index]+line.len()};
            let found_tags=tags(trimmed,number,indent);
            let found_step=step(trimmed,&keywords);
            let found_row=trimmed.starts_with('|').then(||row(trimmed,number,indent,span));
            let ticks=trimmed.chars().take_while(|c|*c=='`').count();
            // The official grammar tries executable tokens before TagLine in
            // step/argument and Examples-table states. A backtick tag embedded
            // in their payload is literal; pending declaration tags still win.
            let current_step=document.feature.as_ref().and_then(|feature|feature.last_step(target));
            let executable_step=matches!(target,Target::Scenario(..)|Target::Background(..))&&found_step.is_some();
            let row_slot=current_step.is_some_and(|step|step.table.is_empty()||last_argument_table)
                ||matches!(target,Target::Examples(..));
            let executable_row=row_slot&&(2..=5).contains(&indent)
                &&found_row.as_ref().is_some_and(|row|!separator(row));
            let executable_fence=current_step.is_some_and(|step|step.doc_string.is_none())&&ticks>=3;
            let literal_token=pending.is_empty()&&(executable_step||executable_row||executable_fence);
            if !found_tags.is_empty()&&!literal_token{pending.extend(found_tags);index+=1;continue;}
            let heading=title(trimmed,&keywords);
            if document.feature.is_none() {
                let (keyword,name,column)=match heading {Some((Kind::Feature,word,name,prefix))=>(Some(word.into()),name,indent+prefix+1),_=>(None,trimmed.to_owned(),indent+1)};
                document.feature=Some(Feature {header:Header {keyword,name,position:LineCol{line:number,col:column},span,tags:std::mem::take(&mut pending)},background:None,scenarios:vec![],rules:vec![]});index+=1;continue;
            }
            let feature=document.feature.as_mut().unwrap();
            if let Some((kind,word,name,prefix))=heading {
                if kind==Kind::Feature {index+=1;continue;}
                let header=Header {keyword:Some(word.into()),name,position:LineCol{line:number,col:indent+prefix+1},span,tags:std::mem::take(&mut pending)};
                match kind {
                    Kind::Rule=>{let r=feature.rules.len();feature.rules.push(Rule {header,background:None,scenarios:vec![]});rule=Some(r);target=Target::Rule;},
                    Kind::Background=>{if !header.tags.is_empty()||!feature.scenarios(rule).is_empty()||feature.background(rule).is_some(){return Err(error(number,"background must precede scenarios and cannot have tags"));}*feature.background(rule)=Some(Background{header,steps:vec![]});target=Target::Background(rule);},
                    Kind::Scenario=>{let scenarios=feature.scenarios(rule);let s=scenarios.len();scenarios.push(Scenario{header,steps:vec![],examples:vec![]});target=Target::Scenario(rule,s);},
                    Kind::Examples=>{let (r,s)=match target {Target::Scenario(r,s)|Target::Examples(r,s,_)=>(r,s),_=>return Err(error(number,"Examples outside scenario"))};let examples=&mut feature.scenarios(r)[s].examples;let e=examples.len();examples.push(Examples{header,rows:vec![]});target=Target::Examples(r,s,e);},
                    Kind::Feature=>unreachable!(),
                }
                last_argument_table=false;index+=1;continue;
            }
            if let Some((word,kind,text,prefix))=found_step {
                if !pending.is_empty(){return Err(error(number,"tags must precede a declaration"));}
                feature.steps(target,number)?.push(Step {keyword:word.into(),keyword_type:kind,text,position:LineCol{line:number,col:indent+prefix+1},span,doc_string:None,table:vec![]});
                last_argument_table=false;index+=1;continue;
            }
            if let Some(candidate)=found_row {
                if separator(&candidate){index+=1;continue;}
                if (2..=5).contains(&indent) {
                    if !pending.is_empty(){return Err(error(number,"tags before table"));}
                    if let Target::Examples(r,s,e)=target {add_row(&mut feature.scenarios(r)[s].examples[e].rows,candidate)?;}
                    else {let step=feature.steps(target,number)?.last_mut().ok_or_else(||error(number,"table without step"))?;if !step.table.is_empty()&&!last_argument_table{return Err(error(number,"duplicate data table"));}add_row(&mut step.table,candidate)?;}
                    last_argument_table=true;
                }
                index+=1;continue;
            }
            if ticks>=3 {
                if !pending.is_empty(){return Err(error(number,"tags before doc string"));}
                let step=feature.steps(target,number)?.last_mut().ok_or_else(||error(number,"doc string without step"))?;
                if step.doc_string.is_some(){return Err(error(number,"duplicate doc string"));}
                let delimiter="`".repeat(ticks);let media=trimmed[ticks..].to_owned();let mut content=Vec::new();index+=1;
                while index<lines.len()&&lines[index].trim_start_matches(ws)!=delimiter {
                    let body=lines[index];let body_indent=body.chars().take_while(|c|ws(*c)).count();
                    content.push(body.chars().skip(indent.min(body_indent)).collect::<String>());index+=1;
                }
                if index==lines.len(){return Err(error(number,"unterminated Markdown doc string"));}
                step.doc_string=Some(DocString {content:content.join("\n"),delimiter,media_type:(!media.is_empty()).then_some(media),position:LineCol {line:number,col:indent+1}});
                last_argument_table=false;index+=1;continue;
            }
            // The Markdown matcher classifies all remaining prose as Empty.
            index+=1;
        }
        if !pending.is_empty(){return Err(error(lines.len().max(1),"tags without a declaration"));}
        Ok(document)
    }

    /// Projects source nodes into the existing native execution AST.
    /// An absent fallback Feature keyword projects to an empty string.
    pub fn to_feature(&self) -> Option<crate::Feature> {
        let f=self.feature.as_ref()?;Some(crate::Feature {keyword:f.header.keyword.clone().unwrap_or_default(),name:f.header.name.clone(),description:None,background:f.background.as_ref().map(native_background),scenarios:f.scenarios.iter().map(native_scenario).collect(),rules:f.rules.iter().map(|r|crate::Rule {keyword:r.header.keyword.clone().unwrap_or_default(),name:r.header.name.clone(),description:None,background:r.background.as_ref().map(native_background),scenarios:r.scenarios.iter().map(native_scenario).collect(),tags:native_tags(&r.header),span:r.header.span,position:r.header.position}).collect(),tags:native_tags(&f.header),span:f.header.span,position:f.header.position,path:None})
    }
    /// Projects a document while retaining the caller's original source identity.
    pub fn to_feature_at(&self,path:impl AsRef<Path>)->Option<crate::Feature>{self.to_feature().map(|mut f|{f.path=Some(path.as_ref().to_path_buf());f})}
}
fn native_tags(h:&Header)->Vec<String>{h.tags.iter().map(|t|t.name.strip_prefix('@').unwrap_or(&t.name).into()).collect()}
fn native_table(rows:&[Row])->Option<crate::Table>{let first=rows.first()?;Some(crate::Table{rows:rows.iter().map(|r|r.cells.iter().map(|c|c.value.clone()).collect()).collect(),span:Span{start:first.span.start,end:rows.last().unwrap().span.end},position:first.position})}
fn native_steps(steps:&[Step])->Vec<crate::Step>{let mut ty=StepType::Given;steps.iter().map(|s|{ty=match s.keyword_type{KeywordType::Context=>StepType::Given,KeywordType::Action=>StepType::When,KeywordType::Outcome=>StepType::Then,_=>ty};crate::Step{keyword:s.keyword.trim_end_matches(ws).into(),ty,value:s.text.clone(),docstring:s.doc_string.as_ref().map(|d|d.content.clone()),table:native_table(&s.table),span:s.span,position:s.position}}).collect()}
fn native_background(b:&Background)->crate::Background{crate::Background{keyword:b.header.keyword.clone().unwrap_or_default(),name:b.header.name.clone(),description:None,steps:native_steps(&b.steps),span:b.header.span,position:b.header.position}}
fn native_scenario(s:&Scenario)->crate::Scenario{crate::Scenario{keyword:s.header.keyword.clone().unwrap_or_default(),name:s.header.name.clone(),description:None,steps:native_steps(&s.steps),examples:s.examples.iter().map(|e|crate::Examples{keyword:e.header.keyword.clone().unwrap_or_default(),name:(!e.header.name.is_empty()).then(||e.header.name.clone()),description:None,table:native_table(&e.rows),tags:native_tags(&e.header),span:e.header.span,position:e.header.position}).collect(),tags:native_tags(&s.header),span:s.header.span,position:s.header.position}}
