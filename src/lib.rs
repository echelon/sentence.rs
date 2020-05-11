// Copyright (c) 2016, 2020 Brandon Thomas <bt@brand.io>

#![deny(dead_code)]
#![deny(missing_docs)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]

//! **Sentence**, a library for lexing English language sentences into tokens for TTS purposes.
//!
//! Usage:
//!
//! ```rust
//! use sentence::{SentenceTokenizer, Token, Punctuation};
//! let tokenizer = SentenceTokenizer::new();
//! let tokens = tokenizer.tokenize("Hello, world!");
//! assert_eq!(tokens, vec![
//!   Token::Word("Hello".into()),
//!   Token::Punctuation(Punctuation::Comma),
//!   Token::Word("world".into()),
//!   Token::Punctuation(Punctuation::Exclamation),
//! ]);
//! ```

#[macro_use] extern crate lazy_static;
use regex::Regex;

/// Sentence Tokenizer.
///
/// For now this is stateless. This library is in a very early state, but I intend to add
/// preferences and dictionary lookup callback support. ("Intend to come back" are famous last
/// words.)
pub struct SentenceTokenizer {
}

// TODO: Emdash,
// TODO: Ellipsis,
/// Punctuation marks
#[derive(Clone, Debug, PartialEq)]
pub enum Punctuation {
  /// Colon: ':'
  Colon,
  /// Comma: ','
  Comma,
  /// Dash: '-'
  Dash,
  /// Exclamation: '!'
  Exclamation,
  /// Period: '.'
  Period,
  /// Question: '?'
  Question,
  /// Semicolon: ';'
  Semicolon,
}

// TODO: Currency/prices, ordinals, percentages, math symbols, emoji, etc.
/// A parsed token
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
  /// A comma-formatted integer. Like Integer, but has comma separators.
  CommaFormattedInteger(String),
  /// A comma-formatted real number. Like RealNumber, but has comma separators.
  CommaFormattedRealNumber(String),
  /// Twitter-style hashtag, which matches '^#\w+$'.
  Hashtag(String),
  /// A hyphenated word matches '^([A-Za-z]+\-)+[A-Za-z]+$'.
  /// If the word isn't in your dictionary, perhaps splitting by hyphens will help.
  HyphenatedWord(String),
  /// A simple integer. Matches '\d+'
  Integer(String),
  /// A punctuation mark.
  Punctuation(Punctuation),
  /// A simple real number. Matches '\d+\.\d+'
  RealNumber(String),
  /// An http:// or https:// prefixed URL
  Url(String),
  /// Twitter-style username mention, which matches '^@\w+'
  UsernameMention(String),
  /// A word matches '^\w+$'.
  Word(String),
  /// A sequence that doesn't match any other pattern. Catch-all.
  Unknown(String),
}

impl SentenceTokenizer {
  /// Constructor.
  pub fn new() -> Self {
    // TODO: I'm 99% sure I'll pick up this work and make this configurable, but right now
    //  it has no state and making this a struct is overblown.
    Self {}
  }

  /// Turn a text sequence into a series of tokens.
  ///
  /// ```rust
  /// use sentence::{SentenceTokenizer, Token, Punctuation};
  /// let tokenizer = SentenceTokenizer::new();
  /// let tokens = tokenizer.tokenize("Hello, world!");
  /// assert_eq!(tokens, vec![
  ///   Token::Word("Hello".into()),
  ///   Token::Punctuation(Punctuation::Comma),
  ///   Token::Word("world".into()),
  ///   Token::Punctuation(Punctuation::Exclamation),
  /// ]);
  /// ```
  pub fn tokenize(&self, sequence: &str) -> Vec<Token> {
    let split = sequence.split(char::is_whitespace);
    let mut tokens = Vec::new();

    for s in split {
      let trim = s.trim();
      if trim.len() == 0 {
        continue;
      }
      tokens.push(Token::Unknown(s.to_string()));
    }

    // TODO: None of this is efficient.
    Self::separate_end_punctuation(&mut tokens);
    Self::parse_integers_and_reals(&mut tokens);
    Self::parse_words_etc(&mut tokens);

    tokens
  }

  // Within a token sequence, replace tokens like Unknown("word.") with two tokens,
  // one of which is the appropriate punctuation mark, and the other Unknown("word").
  fn separate_end_punctuation(tokens: &mut Vec<Token>) {
    lazy_static! {
      static ref PUNCTUATION: Regex = Regex::new(r"([\.\?\-:!,;]+)$").unwrap();
    }

    let mut i = 0;

    while i < tokens.len() {
      // TODO: We might benefit from a custom iterator.
      let token = if let Some(Token::Unknown(token)) = tokens.get(i) {
        token
      } else {
        i += 1;
        continue
      };

      let (before, punctuation, after)
          = if let Some(mat) = PUNCTUATION.find(token)
      {
        let punctuation = match mat.as_str() {
          "!" => Punctuation::Exclamation,
          "," => Punctuation::Comma,
          "-" => Punctuation::Dash,
          "." => Punctuation::Period,
          ":" => Punctuation::Colon,
          ";" => Punctuation::Semicolon,
          "?" => Punctuation::Question,
          _ => {
            i += 1;
            continue
          },
        };

        let before = token.get(0..mat.start())
            .filter(|s| s.len() > 0)
            .map(|s| s.to_string());

        let after = token.get(mat.end()..token.len())
            .filter(|s| s.len() > 0)
            .map(|s| s.to_string());

        (before, punctuation, after)
      } else {
        i += 1;
        continue
      };

      // String before the punctuation match
      let mut insert = false;
      if let Some(before) = before {
        if let Some(elem) = tokens.get_mut(i) {
          *elem = Token::Unknown(before);
        }
        i += 1;
        insert = true;
      }

      // Punctuation
      if insert {
        tokens.insert(i, Token::Punctuation(punctuation));
      } else {
        if let Some(elem) = tokens.get_mut(i) {
          *elem = Token::Punctuation(punctuation);
        }
      }

      i += 1;

      // String after the punctuation match
      if let Some(after) = after {
        tokens.insert(i, Token::Unknown(after));
        i += 1;
      }
    }
  }

  // Materialize Unknown("\d+") and Unknown("\d+\.\d+") sequences into integer and real tokens.
  fn parse_integers_and_reals(tokens: &mut Vec<Token>) {
    lazy_static! {
      static ref REALS : Regex = Regex::new(r"^\d+\.\d+$").unwrap();
      static ref INTEGERS : Regex = Regex::new(r"^\d+$").unwrap();
      static ref COMMA_FORMATTED_REALS : Regex = Regex::new(r"^(\d+,)+\d+\.\d+$").unwrap();
      static ref COMMA_FORMATTED_INTEGERS : Regex = Regex::new(r"^(\d+,)+\d+$").unwrap();
    }

    for token in tokens.iter_mut() {
      match token {
        Token::Unknown(value) => {
          if REALS.is_match(value) {
            *token = Token::RealNumber(value.clone()); // TODO: Move instead.
          }
          else if INTEGERS.is_match(value) {
            *token = Token::Integer(value.clone()); // TODO: Move instead.
          }
          else if COMMA_FORMATTED_REALS.is_match(value) {
            *token = Token::CommaFormattedRealNumber(value.clone()); // TODO: Move instead.
          }
          else if COMMA_FORMATTED_INTEGERS.is_match(value) {
            *token = Token::CommaFormattedInteger(value.clone()); // TODO: Move instead.
          }
        },
        _ => continue,
      }
    }
  }

  // Materialize Unknown("\w+") sequences into word tokens.
  fn parse_words_etc(tokens: &mut Vec<Token>) {
    lazy_static! {
      static ref WORD : Regex = Regex::new(r"^\w+$").unwrap();
      static ref HYPHENATED_WORD : Regex = Regex::new(r"^([A-Za-z]+\-)+[A-Za-z]+$").unwrap();
      static ref URL : Regex = Regex::new(r"^http(s)?://(\w+\.)+(\w+)/?([\w/#\?&=\.])*$").unwrap();
      static ref USERNAME : Regex = Regex::new(r"^@\w+$").unwrap();
      static ref HASHTAG : Regex = Regex::new(r"^#\w+$").unwrap();
    }

    for token in tokens.iter_mut() {
      match token {
        Token::Unknown(value) => {
          if URL.is_match(value) {
            *token = Token::Url(value.clone()); // TODO: Move instead.
          }
          else if HASHTAG.is_match(value) {
            *token = Token::Hashtag(value.clone()); // TODO: Move instead.
          }
          else if USERNAME.is_match(value) {
            *token = Token::UsernameMention(value.clone()); // TODO: Move instead.
          }
          else if WORD.is_match(value) {
            *token = Token::Word(value.clone()); // TODO: Move instead.
          }
          else if HYPHENATED_WORD.is_match(value) {
            *token = Token::HyphenatedWord(value.clone()); // TODO: Move instead.
          }
        },
        _ => continue,
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::Punctuation;
  use crate::SentenceTokenizer;
  use crate::Token;

  #[test]
  fn simple_sentence() {
    let sentence = "this is an example";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("this".into()),
      Token::Word("is".into()),
      Token::Word("an".into()),
      Token::Word("example".into()),
    ]);
  }

  #[test]
  fn simple_sentence_with_punctuation() {
    let sentence = "This, right here, is a sentence.";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("This".into()),
      Token::Punctuation(Punctuation::Comma),
      Token::Word("right".into()),
      Token::Word("here".into()),
      Token::Punctuation(Punctuation::Comma),
      Token::Word("is".into()),
      Token::Word("a".into()),
      Token::Word("sentence".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
  }

  #[test]
  fn hyphenated_words() {
    let sentence = "Please double-check the drive-thru";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("Please".into()),
      Token::HyphenatedWord("double-check".into()),
      Token::Word("the".into()),
      Token::HyphenatedWord("drive-thru".into()),
    ]);
    let sentence = "Please double-check the drive-thru - pretty-please.";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("Please".into()),
      Token::HyphenatedWord("double-check".into()),
      Token::Word("the".into()),
      Token::HyphenatedWord("drive-thru".into()),
      Token::Punctuation(Punctuation::Dash),
      Token::HyphenatedWord("pretty-please".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
  }

  #[test]
  fn sentence_with_integers() {
    let sentence = "9 out of 10 agree";
    assert_eq!(tokenize(sentence), vec![
      Token::Integer("9".into()),
      Token::Word("out".into()),
      Token::Word("of".into()),
      Token::Integer("10".into()),
      Token::Word("agree".into()),
    ]);
  }

  #[test]
  fn sentence_with_integers_and_punctuation() {
    let sentence = "1, 2, 3, 100.";
    assert_eq!(tokenize(sentence), vec![
      Token::Integer("1".into()),
      Token::Punctuation(Punctuation::Comma),
      Token::Integer("2".into()),
      Token::Punctuation(Punctuation::Comma),
      Token::Integer("3".into()),
      Token::Punctuation(Punctuation::Comma),
      Token::Integer("100".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
  }

  #[test]
  fn sentence_with_real_numbers_and_punctuation() {
    let sentence = "The total comes to 25.15.";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("The".into()),
      Token::Word("total".into()),
      Token::Word("comes".into()),
      Token::Word("to".into()),
      Token::RealNumber("25.15".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
  }

  #[test]
  fn number_with_commas() {
    let sentence = "1,000,000 people have 1,234.56 points";
    assert_eq!(tokenize(sentence), vec![
      Token::CommaFormattedInteger("1,000,000".into()),
      Token::Word("people".into()),
      Token::Word("have".into()),
      Token::CommaFormattedRealNumber("1,234.56".into()),
      Token::Word("points".into()),
    ]);
  }

  #[test]
  fn punctuation_colon() {
    // No space.
    let sentence = "one: two";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("one".into()),
      Token::Punctuation(Punctuation::Colon),
      Token::Word("two".into()),
    ]);
    // With a space.
    let sentence = "one : two";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("one".into()),
      Token::Punctuation(Punctuation::Colon),
      Token::Word("two".into()),
    ]);
    // At the end
    let sentence = "this:";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("this".into()),
      Token::Punctuation(Punctuation::Colon),
    ]);
  }

  #[test]
  fn punctuation_question() {
    // No space.
    let sentence = "what? no";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("what".into()),
      Token::Punctuation(Punctuation::Question),
      Token::Word("no".into()),
    ]);
    // With a space.
    let sentence = "what ? no";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("what".into()),
      Token::Punctuation(Punctuation::Question),
      Token::Word("no".into()),
    ]);
    // At the end.
    let sentence = "what?";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("what".into()),
      Token::Punctuation(Punctuation::Question),
    ]);
    // Just question.
    let sentence = "?";
    assert_eq!(tokenize(sentence), vec![
      Token::Punctuation(Punctuation::Question),
    ]);
  }

  #[test]
  fn punctuation_exclamation() {
    // No space.
    let sentence = "yes! that";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("yes".into()),
      Token::Punctuation(Punctuation::Exclamation),
      Token::Word("that".into()),
    ]);
    // With a space.
    let sentence = "yes ! that";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("yes".into()),
      Token::Punctuation(Punctuation::Exclamation),
      Token::Word("that".into()),
    ]);
    // At the end.
    let sentence = "yes!";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("yes".into()),
      Token::Punctuation(Punctuation::Exclamation),
    ]);
  }

  #[test]
  fn punctuation_semicolon() {
    // No space.
    let sentence = "one; two";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("one".into()),
      Token::Punctuation(Punctuation::Semicolon),
      Token::Word("two".into()),
    ]);
    // With a space.
    let sentence = "one ; two";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("one".into()),
      Token::Punctuation(Punctuation::Semicolon),
      Token::Word("two".into()),
    ]);
    // At the end.
    let sentence = "one;";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("one".into()),
      Token::Punctuation(Punctuation::Semicolon),
    ]);
  }

  #[test]
  fn punctuation_dash() {
    // Single space.
    let sentence = "but- no";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("but".into()),
      Token::Punctuation(Punctuation::Dash),
      Token::Word("no".into()),
    ]);
    // With a space.
    let sentence = "but - no";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("but".into()),
      Token::Punctuation(Punctuation::Dash),
      Token::Word("no".into()),
    ]);
    // At the end.
    let sentence = "but-";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("but".into()),
      Token::Punctuation(Punctuation::Dash),
    ]);

    // TODO: Intra-word is not supported.
    //  I want to work on dictionary lookup before adding it.
  }

  #[test]
  fn urls() {
    let sentence = "Go to https://google.com";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("Go".into()),
      Token::Word("to".into()),
      Token::Url("https://google.com".into()),
    ]);
    let sentence = "Go to https://www.google.com.";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("Go".into()),
      Token::Word("to".into()),
      Token::Url("https://www.google.com".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
    let sentence = "My website is http://127.0.0.1";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("My".into()),
      Token::Word("website".into()),
      Token::Word("is".into()),
      Token::Url("http://127.0.0.1".into()),
    ]);
    let sentence = "My website is http://127.0.0.1/my/page.html?foo=bar&bin=baz#hah";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("My".into()),
      Token::Word("website".into()),
      Token::Word("is".into()),
      Token::Url("http://127.0.0.1/my/page.html?foo=bar&bin=baz#hah".into()),
    ]);
  }

  #[test]
  fn hashtags() {
    let sentence = "#hashtag";
    assert_eq!(tokenize(sentence), vec![
      Token::Hashtag("#hashtag".into()),
    ]);
    let sentence = "This is #rust #awesomeness!";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("This".into()),
      Token::Word("is".into()),
      Token::Hashtag("#rust".into()),
      Token::Hashtag("#awesomeness".into()),
      Token::Punctuation(Punctuation::Exclamation),
    ]);
  }

  #[test]
  fn usernames() {
    let sentence = "@echelon";
    assert_eq!(tokenize(sentence), vec![
      Token::UsernameMention("@echelon".into()),
    ]);
    let sentence = "The author is @echelon.";
    assert_eq!(tokenize(sentence), vec![
      Token::Word("The".into()),
      Token::Word("author".into()),
      Token::Word("is".into()),
      Token::UsernameMention("@echelon".into()),
      Token::Punctuation(Punctuation::Period),
    ]);
  }

  #[test]
  fn empty_strings() {
    // Empty.
    let sentence = "";
    assert_eq!(tokenize(sentence), vec![]);
    // Just space.
    let sentence = "  ";
    assert_eq!(tokenize(sentence), vec![]);
    // Just tabs.
    let sentence = "\t\t";
    assert_eq!(tokenize(sentence), vec![]);
    // Just newlines.
    let sentence = "\n\n\n";
    assert_eq!(tokenize(sentence), vec![]);
    // Mix.
    let sentence = "\n \t \n";
    assert_eq!(tokenize(sentence), vec![]);
  }

  #[test]
  fn not_yet_supported_but_ensure_no_infinite_loop() {
    let _ = tokenize(".");
    let _ = tokenize("...");
    let _ = tokenize(". . .");
    let _ = tokenize("yes!!!!!");
    let _ = tokenize("yes!!!!1??");
    let _ = tokenize("iOHuijahdfkjq2nero88u928nkjwfn  qio23u980HjkH@!J#Kj1j 1j4o2o");
    let _ = tokenize("dashes--emdash");
    let _ = tokenize("This does not work!?");
    let _ = tokenize("haven't, how're, she'll, isn't, it'll, it'd, donald's");
    let _ = tokenize("might've, they'd, weren't, o'neill's, o'grady's");
    let _ = tokenize("'nuff, 'em, o'clock, will-o'-the-wisp");
    let _ = tokenize("I'm sorry you can't do it.");
    let _ = tokenize("That is \"good\" enough");
  }

  fn tokenize(sentence: &str) -> Vec<Token> {
    let tokenizer = SentenceTokenizer {};
    tokenizer.tokenize(sentence)
  }
}
