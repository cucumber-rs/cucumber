use std::{fmt::Display, iter, str, vec};

use either::Either;
use nom::{AsChar, InputIter};

use crate::{
    Alternation, Alternative, Expression, Optional, Parameter,
    SingleAlternation, SingleExpression,
};

#[test]
fn spec() {
    let string = Expression::parse("test(opt)/\\{/\\(/[")
        .unwrap()
        .into_regex_char_iter()
        .collect::<String>();
    println!("{}", string);
}

trait IntoRegexCharIter {
    type Iter: Iterator<Item = char>;

    fn into_regex_char_iter(self) -> Self::Iter;
}

impl<Input> IntoRegexCharIter for Expression<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = iter::Chain<
        iter::Chain<
            iter::Once<char>,
            iter::FlatMap<
                vec::IntoIter<SingleExpression<Input>>,
                <SingleExpression<Input> as IntoRegexCharIter>::Iter,
                fn(
                    SingleExpression<Input>,
                )
                    -> <SingleExpression<Input> as IntoRegexCharIter>::Iter,
            >,
        >,
        iter::Once<char>,
    >;

    fn into_regex_char_iter(self) -> Self::Iter {
        let into_regex_char_iter: fn(_) -> _ =
            IntoRegexCharIter::into_regex_char_iter;
        iter::once('^')
            .chain(self.0.into_iter().flat_map(into_regex_char_iter))
            .chain(iter::once('$'))
    }
}

impl<Input> IntoRegexCharIter for SingleExpression<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = Either<
        <Alternation<Input> as IntoRegexCharIter>::Iter,
        Either<
            <Optional<Input> as IntoRegexCharIter>::Iter,
            Either<
                <Parameter<Input> as IntoRegexCharIter>::Iter,
                Either<
                    EscapeForRegex<
                        iter::Map<
                            <Input as InputIter>::IterElem,
                            fn(<Input as InputIter>::Item) -> char,
                        >,
                    >,
                    iter::Once<char>,
                >,
            >,
        >,
    >;

    fn into_regex_char_iter(self) -> Self::Iter {
        use Either::{Left, Right};

        match self {
            Self::Alternation(alt) => Left(alt.into_regex_char_iter()),
            Self::Optional(opt) => Right(Left(opt.into_regex_char_iter())),
            Self::Parameter(p) => Right(Right(Left(p.into_regex_char_iter()))),
            Self::Text(t) => Right(Right(Right(Left(EscapeForRegex::new(
                t.iter_elements().map(AsChar::as_char),
            ))))),
            Self::Whitespace => Right(Right(Right(Right(iter::once(' '))))),
        }
    }
}

impl<Input> IntoRegexCharIter for Alternation<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = SkipLast<
        iter::FlatMap<
            vec::IntoIter<SingleAlternation<Input>>,
            iter::Chain<
                iter::Chain<
                    str::Chars<'static>,
                    iter::FlatMap<
                        vec::IntoIter<Alternative<Input>>,
                        <Alternative<Input> as IntoRegexCharIter>::Iter,
                        fn(
                            Alternative<Input>,
                        )
                            -> <Alternative<Input> as IntoRegexCharIter>::Iter,
                    >,
                >,
                str::Chars<'static>,
            >,
            fn(
                SingleAlternation<Input>,
            ) -> iter::Chain<
                iter::Chain<
                    str::Chars<'static>,
                    iter::FlatMap<
                        vec::IntoIter<Alternative<Input>>,
                        <Alternative<Input> as IntoRegexCharIter>::Iter,
                        fn(
                            Alternative<Input>,
                        )
                            -> <Alternative<Input> as IntoRegexCharIter>::Iter,
                    >,
                >,
                str::Chars<'static>,
            >,
        >,
    >;

    fn into_regex_char_iter(self) -> Self::Iter {
        let single_alt: fn(SingleAlternation<Input>) -> _ = |alt| {
            let into_regex_char_iter: fn(_) -> _ =
                IntoRegexCharIter::into_regex_char_iter;

            "(?:"
                .chars()
                .chain(alt.into_iter().flat_map(into_regex_char_iter))
                .chain(")|".chars())
        };

        SkipLast::new(self.0.into_iter().flat_map(single_alt))
    }
}

impl<Input> IntoRegexCharIter for Alternative<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = Either<
        <Optional<Input> as IntoRegexCharIter>::Iter,
        EscapeForRegex<
            iter::Map<
                <Input as InputIter>::IterElem,
                fn(<Input as InputIter>::Item) -> char,
            >,
        >,
    >;

    fn into_regex_char_iter(self) -> Self::Iter {
        use Either::{Left, Right};

        let as_char: fn(<Input as InputIter>::Item) -> char = AsChar::as_char;
        match self {
            Self::Optional(opt) => Left(opt.into_regex_char_iter()),
            Self::Text(text) => {
                Right(EscapeForRegex::new(text.iter_elements().map(as_char)))
            }
        }
    }
}

impl<Input> IntoRegexCharIter for Optional<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = iter::Chain<
        iter::Chain<
            str::Chars<'static>,
            EscapeForRegex<
                iter::Map<
                    <Input as InputIter>::IterElem,
                    fn(<Input as InputIter>::Item) -> char,
                >,
            >,
        >,
        str::Chars<'static>,
    >;

    fn into_regex_char_iter(self) -> Self::Iter {
        let as_char: fn(<Input as InputIter>::Item) -> char = AsChar::as_char;
        "(?:"
            .chars()
            .chain(EscapeForRegex::new(self.0.iter_elements().map(as_char)))
            .chain(")?".chars())
    }
}

impl<Input> IntoRegexCharIter for Parameter<Input>
where
    Input: Display + InputIter,
    <Input as InputIter>::Item: AsChar,
{
    type Iter = str::Chars<'static>;

    fn into_regex_char_iter(self) -> Self::Iter {
        "(.*)".chars()
    }
}

struct SkipLast<Iter: Iterator> {
    iter: iter::Peekable<Iter>,
}

impl<Iter: Iterator> SkipLast<Iter> {
    fn new(iter: Iter) -> Self {
        Self {
            iter: iter.peekable(),
        }
    }
}

impl<Iter> Iterator for SkipLast<Iter>
where
    Iter: Iterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        (self.iter.peek().is_some()).then(|| next).flatten()
    }
}

struct EscapeForRegex<Iter: Iterator> {
    iter: iter::Peekable<Iter>,
    was_escaped: Option<Iter::Item>,
}

impl<Iter: Iterator> EscapeForRegex<Iter> {
    fn new(iter: Iter) -> Self {
        Self {
            iter: iter.peekable(),
            was_escaped: None,
        }
    }
}

impl<Iter> Iterator for EscapeForRegex<Iter>
where
    Iter: Iterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let should_be_escaped = |c| "^$[](){}.|?*+".contains(c);

        if let Some(c) = self.was_escaped.take() {
            return Some(c);
        }

        loop {
            return match self.iter.next() {
                Some('\\') => {
                    let c = *self.iter.peek()?;
                    if should_be_escaped(c) {
                        self.was_escaped = self.iter.next();
                        return Some('\\');
                    }
                    continue;
                }
                Some(c) if should_be_escaped(c) => {
                    self.was_escaped = Some(c);
                    Some('\\')
                }
                Some(c) => Some(c),
                None => None,
            };
        }
    }
}
