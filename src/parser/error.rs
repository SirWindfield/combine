//! Parsers which cause errors or modifies the returned error on parse failure.

use crate::lib::marker::PhantomData;

use crate::error::{Info, ParseError, ParseResult, StreamError, Tracked};
use crate::parser::ParseMode;
use crate::{Parser, Stream, StreamOnce};

use crate::error::ParseResult::*;

#[derive(Clone)]
pub struct Unexpected<I, T>(Info<I::Item, I::Range>, PhantomData<fn(I) -> (I, T)>)
where
    I: Stream;
impl<Input, T> Parser<Input> for Unexpected<Input, T>
where
    Input: Stream,
{
    type Output = T;
    type PartialState = ();
    #[inline]
    fn parse_lazy(&mut self, input: &mut Input) -> ParseResult<T, <Input as StreamOnce>::Error> {
        EmptyErr(<Input as StreamOnce>::Error::empty(input.position()).into())
    }
    fn add_error(&mut self, errors: &mut Tracked<<Input as StreamOnce>::Error>) {
        errors.error.add(StreamError::unexpected(self.0.clone()));
    }
}
/// Always fails with `message` as an unexpected error.
/// Never consumes any input.
///
/// Has `()` the output type
///
/// ```
/// # extern crate combine;
/// # use combine::*;
/// # use combine::error::StreamError;
/// # fn main() {
/// let result = unexpected("token")
///     .easy_parse("a");
/// assert!(result.is_err());
/// assert!(
///     result.err()
///         .unwrap()
///         .errors
///         .iter()
///         .any(|m| *m == StreamError::unexpected("token".into()))
/// );
/// # }
/// ```
#[inline]
pub fn unexpected<I, S>(message: S) -> Unexpected<I, ()>
where
    I: Stream,
    S: Into<Info<I::Item, I::Range>>,
{
    unexpected_any(message)
}

/// Always fails with `message` as an unexpected error.
/// Never consumes any input.
///
/// May have anything as the output type but must be used such that the output type can inferred.
/// The `unexpected` parser can be used if the output type does not matter
///
/// ```
/// # extern crate combine;
/// # use combine::*;
/// # use combine::parser::error::unexpected_any;
/// # use combine::error::StreamError;
/// # fn main() {
/// let result = token('b').or(unexpected_any("token"))
///     .easy_parse("a");
/// assert!(result.is_err());
/// assert!(
///     result.err()
///         .unwrap()
///         .errors
///         .iter()
///         .any(|m| *m == StreamError::unexpected("token".into()))
/// );
/// # }
/// ```
#[inline]
pub fn unexpected_any<I, S, T>(message: S) -> Unexpected<I, T>
where
    I: Stream,
    S: Into<Info<I::Item, I::Range>>,
{
    Unexpected(message.into(), PhantomData)
}

#[derive(Clone)]
pub struct Message<P, S>(P, S);
impl<Input, P, S> Parser<Input> for Message<P, S>
where
    Input: Stream,
    P: Parser<Input>,
    S: Clone + Into<Info<<Input as StreamOnce>::Item, <Input as StreamOnce>::Range>>,
{
    type Output = P::Output;
    type PartialState = P::PartialState;

    parse_mode!(Input);
    #[inline]
    fn parse_mode_impl<M>(
        &mut self,
        mode: M,
        input: &mut Input,
        state: &mut Self::PartialState,
    ) -> ParseResult<Self::Output, <Input as StreamOnce>::Error>
    where
        M: ParseMode,
    {
        match self.0.parse_mode(mode, input, state) {
            ConsumedOk(x) => ConsumedOk(x),
            EmptyOk(x) => EmptyOk(x),

            // The message should always be added even if some input was consumed before failing
            ConsumedErr(mut err) => {
                err.add_message(self.1.clone().into());
                ConsumedErr(err)
            }

            // The message will be added in `add_error`
            EmptyErr(err) => EmptyErr(err),
        }
    }

    fn add_error(&mut self, errors: &mut Tracked<<Input as StreamOnce>::Error>) {
        self.0.add_error(errors);
        errors.error.add_message(self.1.clone().into());
    }

    forward_parser!(Input, parser_count add_consumed_expected_error, 0);
}

/// Equivalent to [`p1.message(msg)`].
///
/// [`p1.message(msg)`]: ../parser/trait.Parser.html#method.message
#[inline]
pub fn message<Input, P, S>(p: P, msg: S) -> Message<P, S>
where
    P: Parser<Input>,
    Input: Stream,
    S: Clone + Into<Info<<Input as StreamOnce>::Item, <Input as StreamOnce>::Range>>,
{
    Message(p, msg)
}

#[derive(Clone)]
pub struct Expected<P, S>(P, S);
impl<Input, P, S> Parser<Input> for Expected<P, S>
where
    P: Parser<Input>,
    Input: Stream,
    S: Clone + Into<Info<<Input as StreamOnce>::Item, <Input as StreamOnce>::Range>>,
{
    type Output = P::Output;
    type PartialState = P::PartialState;

    parse_mode!(Input);
    #[inline]
    fn parse_mode_impl<M>(
        &mut self,
        mode: M,
        input: &mut Input,
        state: &mut Self::PartialState,
    ) -> ParseResult<Self::Output, <Input as StreamOnce>::Error>
    where
        M: ParseMode,
    {
        self.0.parse_mode(mode, input, state)
    }

    fn add_error(&mut self, errors: &mut Tracked<<Input as StreamOnce>::Error>) {
        ParseError::set_expected(
            errors,
            StreamError::expected(self.1.clone().into()),
            |errors| {
                self.0.add_error(errors);
            },
        )
    }

    forward_parser!(Input, parser_count add_consumed_expected_error, 0);
}

/// Equivalent to [`p.expected(info)`].
///
/// [`p.expected(info)`]: ../parser/trait.Parser.html#method.expected
#[inline]
pub fn expected<Input, P, S>(p: P, info: S) -> Expected<P, S>
where
    P: Parser<Input>,
    Input: Stream,
    S: Clone + Into<Info<<Input as StreamOnce>::Item, <Input as StreamOnce>::Range>>,
{
    Expected(p, info)
}

#[derive(Clone)]
pub struct Silent<P>(P);
impl<Input, P> Parser<Input> for Silent<P>
where
    P: Parser<Input>,
    Input: Stream,
{
    type Output = P::Output;
    type PartialState = P::PartialState;

    parse_mode!(Input);
    #[inline]
    fn parse_mode_impl<M>(
        &mut self,
        mode: M,
        input: &mut Input,
        state: &mut Self::PartialState,
    ) -> ParseResult<Self::Output, <Input as StreamOnce>::Error>
    where
        M: ParseMode,
    {
        self.0.parse_mode(mode, input, state).map_err(|mut err| {
            err.clear_expected();
            err
        })
    }

    fn add_error(&mut self, _errors: &mut Tracked<<Input as StreamOnce>::Error>) {}

    fn add_consumed_expected_error(&mut self, _errors: &mut Tracked<<Input as StreamOnce>::Error>) {
    }

    forward_parser!(Input, parser_count, 0);
}

/// Equivalent to [`p.silent()`].
///
/// [`p.silent()`]: ../trait.Parser.html#method.silent
#[inline]
pub fn silent<Input, P>(p: P) -> Silent<P>
where
    P: Parser<Input>,
    Input: Stream,
{
    Silent(p)
}
