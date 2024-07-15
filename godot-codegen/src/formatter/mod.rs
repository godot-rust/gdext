/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![cfg(not(feature = "codegen-rustfmt"))]

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// Perform a best-effort single-pass formatting pass over a stream of tokens
/// and returning the formatted string.
///
/// "Best effort" here means:
/// - line breaks are added in certain points
///   - attributes
///   - braces `{}`
///   - semicolons `;`
///   - colons if they follow braces `..},`
/// - whitespace around punctuation is tried to be made as natural as possible
///   - `::` always stick to whatever comes after them `use ::crate::module::{..}`
///   - `$` is always prefix
///   - `!` tries to join with whatever comes after it, `!boolean`, `x != y`
///   - `&` tries to join with whatever comes after it, `&self`, `&'static`, `a && b`
///   - joint punctuation tries to stay joint
///   - if an identifier is followed by `.` or `()` then it's "sticky", `f()`, `s.x`
///
/// Current "flaws" (still compiles properly, just a bit ugly):
/// - Because of stickiness, bitwise AND doesn't really look nice, like `a &mask`.
///   Unless we check for all possible keywords (which would be costly), we
///   can't solve this.
/// - Also because of stickiness, macros are a bit weird, `name !`. If we made
///   all `!` stick to previous identifiers then we'd get `x!= y`.
///   Maybe that's a good trade-off if your code contains more not-equals checks
///   than macro invocations.
/// - anonymous function literals are a bit weird because the `|` don't have
///   joint spacing when they come out of a `TokenStream` and identifiers don't
///   have any spacing information attached, so it looks like `| arg |`.
///   Also, to know whether it's a chained bitwise or, a closure would need more
///   information. `move | arg | x`
/// - Because we don't keep track of what's a type and what's an expression,
///   generic parameters look a bit spaced out. `T < A, B >`
/// - `match` arms can be `<pat> => <expr>,` and there _should_ be a line break
///   after the `,` but because this only looks at one TokenKind at a time
///   and only keeps a single state, it's too much effort to keep track of which
///   `,` in a [`TokenTree`] should trigger a newline (it would involve a stack
///   of infos whether that group is in a match or not, detecting pattern etc.)
/// - Because `,` can be used for function args (no-newline) and separators
///   in `struct` and `enum` definitons, those definitions are more awkward
///   and crammed into one line as a trade-off to make function calls and args
///   smoother.
///   Maybe it's worth to make ALL args on their own line, then function calls
///   get big, but `struct`s and `enum`s would be more natural?
pub(crate) fn format_tokens(tokens: TokenStream) -> String {
    let mut out = String::new();

    format(FormatState::Start, 0, tokens.into_iter(), &mut out);

    out
}

fn indent(n: usize) -> &'static str {
    // This looks strange, but it means we don't need to actually allocate anything.
    // The downside is there's a limit to how deep we can nest.
    // The code that's generated doesn't seem like it's any deeper than this.

    //           |   |   |   |   |   |   |   |   |   |   |   |   |   |   |
    let idents = "                                                        ";

    let end = n * 4;
    &idents[0..end]

    // If at some point this approach doesn't work anymore, a `Cow<'static, str>`
    // could be returned.

    // if let Some(s) = idents.get(0..end) {
    //     Cow::Borrowed(s)
    // } else {
    //     Cow::Owned("    ".repeat(n))
    // }
}

//
// Overall structure
//
// The idea is that each for most cases, the current token decides whether to
// *prepend* whitespace or not.
//

/// State that is kept between processing `TokenTree`s, used to decide
/// how to insert whitespace.

#[derive(Copy, Clone, Eq, PartialEq)]
enum FormatState {
    /// Starting state, meaning no whitespace is needed
    Start,
    /// State for when no special whitespace treatment is needed (so, just a
    /// space)
    NothingSpecial,
    /// The previous token was a joined operator, so no space needed
    PrevJoinedOperator,
    /// The previous token was a double colon `::`, so no space needed
    PrevDoubleColon,

    /// The previous token was an identifier, meaning special rules for
    /// function calls, `[]` and `.` indexing
    PrevIdentifier,
    /// The previous token was a closing brace, meaning a newline if there's
    /// no semicolon following.
    PrevClosingBrace,
    /// The previous token was a `#`, if it's followed by `[]` or `!` it's for
    /// attributes, which should not have whitespace after the hash.
    PrevHash,
}

fn format(
    mut state: FormatState,
    level: usize,
    tts: proc_macro2::token_stream::IntoIter,
    s: &mut String,
) {
    for tt in tts {
        format_one(&mut state, level, tt, s);
    }
}

fn format_one(state: &mut FormatState, level: usize, tt: TokenTree, s: &mut String) {
    match tt {
        TokenTree::Punct(punct) => {
            let c = punct.as_char();

            match state {
                FormatState::Start => {}
                FormatState::NothingSpecial => {
                    if c == ';' || c == ',' {
                        // no leading space
                    } else {
                        s.push(' ');
                    }
                }
                FormatState::PrevJoinedOperator => {
                    // joined, don't do anything
                }
                FormatState::PrevDoubleColon => {}
                FormatState::PrevIdentifier => {
                    if ['.', ';', ',', ':'].contains(&c) {
                        // no whitespace
                    } else {
                        s.push(' ');
                    }
                }
                FormatState::PrevClosingBrace => {
                    if c == ';' || c == ',' {
                        // do nothing
                    } else {
                        s.push('\n');
                        s.push_str(indent(level));
                    }
                }
                FormatState::PrevHash => {
                    // do nothing
                }
            }

            s.push(c);

            match (c, *state) {
                (';', _) => {
                    s.push('\n');
                    s.push_str(indent(level));
                    *state = FormatState::Start;
                }
                ('#', _) => {
                    *state = FormatState::PrevHash;
                }
                // only used in macros, always prefix
                ('$', _) => {
                    *state = FormatState::PrevJoinedOperator;
                }
                ('&', FormatState::PrevJoinedOperator) => {
                    *state = FormatState::NothingSpecial;
                }
                ('&', _) => {
                    *state = FormatState::PrevJoinedOperator;
                }
                ('!', FormatState::PrevHash) => {
                    *state = FormatState::PrevHash;
                }
                ('!', FormatState::PrevJoinedOperator) => {
                    *state = FormatState::NothingSpecial;
                }
                ('!', _) => {
                    *state = FormatState::PrevJoinedOperator;
                }
                (':', FormatState::PrevJoinedOperator) => {
                    *state = FormatState::PrevDoubleColon;
                }
                ('.', FormatState::PrevIdentifier) => {
                    *state = FormatState::PrevJoinedOperator;
                }
                (_, FormatState::Start)
                | (_, FormatState::NothingSpecial)
                | (_, FormatState::PrevJoinedOperator)
                | (_, FormatState::PrevDoubleColon)
                | (_, FormatState::PrevIdentifier) => {
                    if punct.spacing() == Spacing::Joint {
                        *state = FormatState::PrevJoinedOperator;
                    } else {
                        *state = FormatState::NothingSpecial;
                    }
                }

                (c, FormatState::PrevClosingBrace) if c == ';' || c == ',' => {
                    // technically ; is already covered, but it's explicit.

                    s.push('\n');
                    s.push_str(indent(level));

                    *state = FormatState::Start;
                }
                (_, FormatState::PrevClosingBrace) => {
                    *state = FormatState::NothingSpecial;
                }
                (_, FormatState::PrevHash) => {
                    // the hash is a little bit special, it's really only
                    // used for attributes #[] and #![]
                    *state = FormatState::PrevJoinedOperator
                }
            }
        }
        TokenTree::Ident(ident) => {
            match state {
                FormatState::NothingSpecial => {
                    s.push(' ');
                }
                FormatState::PrevClosingBrace => {
                    s.push('\n');
                    s.push_str(indent(level));
                }
                FormatState::PrevDoubleColon => {}
                FormatState::PrevIdentifier => {
                    // <ident> <ident>, for example `let test`
                    s.push(' ');
                }
                FormatState::PrevJoinedOperator => {
                    // things like `&mut`, no space
                }
                FormatState::Start => {}
                FormatState::PrevHash => {
                    // shouldn't really happen, no space
                }
            }
            s.push_str(&ident.to_string());
            *state = FormatState::PrevIdentifier;
        }
        TokenTree::Literal(lit) => {
            match state {
                FormatState::Start => {}
                FormatState::NothingSpecial => {
                    s.push(' ');
                }
                FormatState::PrevJoinedOperator => {
                    // shouldn't happen, let's not put a space anyway
                }
                FormatState::PrevDoubleColon => {
                    // shouldn't really happen, but not putting a space
                    // should be fine.
                }
                FormatState::PrevIdentifier => {
                    // also shouldn't really happen, let's put a space for
                    // safety
                    s.push(' ');
                }
                FormatState::PrevClosingBrace => {
                    s.push('\n');
                    s.push_str(indent(level));
                }
                FormatState::PrevHash => {
                    // shouldn't really happen, no space
                }
            }

            s.push_str(&lit.to_string());
            *state = FormatState::NothingSpecial;
        }
        TokenTree::Group(group) => {
            match state {
                FormatState::Start => {}
                FormatState::NothingSpecial => s.push(' '),
                FormatState::PrevJoinedOperator => {
                    // shouldn't happen, no space.
                }
                FormatState::PrevDoubleColon => {
                    // `use module::{a, b, c}`, no space
                }
                FormatState::PrevIdentifier => {
                    // Parens should have no space `f()`
                    // Brackets should have no space `arr[]`
                    // braces should have a space `Self {}`
                    if group.delimiter() == Delimiter::Brace || group.delimiter() == Delimiter::None
                    {
                        s.push(' ');
                    }
                }
                FormatState::PrevClosingBrace => {
                    s.push('\n');
                    s.push_str(indent(level));
                }
                FormatState::PrevHash => {
                    // might be an attribute, no space
                }
            }

            match group.delimiter() {
                Delimiter::Brace => {
                    s.push_str("{\n");
                    s.push_str(indent(level + 1));

                    format(FormatState::Start, level + 1, group.stream().into_iter(), s);

                    s.push('\n');
                    s.push_str(indent(level));
                    s.push('}');

                    *state = FormatState::PrevClosingBrace;
                }
                Delimiter::Bracket => {
                    s.push('[');
                    format(FormatState::Start, level, group.stream().into_iter(), s);
                    s.push(']');
                    if *state == FormatState::PrevHash {
                        s.push('\n');
                        s.push_str(indent(level));
                        *state = FormatState::Start;
                    } else {
                        *state = FormatState::NothingSpecial;
                    }
                }
                Delimiter::Parenthesis => {
                    s.push('(');
                    format(FormatState::Start, level, group.stream().into_iter(), s);
                    s.push(')');
                    *state = FormatState::NothingSpecial;
                }
                Delimiter::None => {
                    format(FormatState::Start, level, group.stream().into_iter(), s);
                    *state = FormatState::NothingSpecial;
                }
            }
        }
    }
}
