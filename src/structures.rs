use std::{
    fmt::{self, Display},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use mirabel::{
    error::{Error, ErrorCode},
    game::{move_code, player_id, MoveCode, MOVE_NONE, PLAYER_NONE, PLAYER_RAND},
};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0},
    combinator::{cut, eof, map, value},
    error::{context, convert_error, VerboseError},
    sequence::{delimited, separated_pair, terminated},
    Finish,
};

type IResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Player {
    Forehand,
    Middlehand,
    Rearhand,
}

impl Player {
    pub(crate) const COUNT: usize = 3;

    const fn all() -> [Self; Self::COUNT] {
        [Self::Forehand, Self::Middlehand, Self::Rearhand]
    }
}

impl From<player_id> for Player {
    /// Convert a [`player_id`] to [`Self`].
    ///
    /// # Panics
    /// Panics if the id is out of range.
    fn from(value: player_id) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(0 == PLAYER_NONE);
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(PLAYER_RAND > 3);
        Self::all()[usize::from(value.checked_sub(1).unwrap())]
    }
}

impl From<Player> for player_id {
    fn from(value: Player) -> Self {
        value as u8 + 1
    }
}

impl Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Player::Forehand => "forehand",
                Player::Middlehand => "middlehand",
                Player::Rearhand => "rearhand",
            }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum CardValue {
    Num7,
    Num8,
    Num9,
    Jack,
    Queen,
    King,
    Num10,
    Ace,
}

impl CardValue {
    pub(crate) const COUNT: usize = 8;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Num7,
            Self::Num8,
            Self::Num9,
            Self::Jack,
            Self::Queen,
            Self::King,
            Self::Num10,
            Self::Ace,
        ]
    }

    /// Parses a card value.
    ///
    /// The input could be either `7`, `8`, `9`, `J`, `Q`, `K`, `10`, or `A`
    /// ignoring case.
    fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "card value",
            alt((
                value(Self::Num7, char('7')),
                value(Self::Num8, char('8')),
                value(Self::Num9, char('9')),
                value(Self::Jack, tag_no_case("J")),
                value(Self::Queen, tag_no_case("Q")),
                value(Self::King, tag_no_case("K")),
                value(Self::Num10, tag("10")),
                value(Self::Ace, tag_no_case("A")),
            )),
        )(input)
    }
}

impl Display for CardValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CardValue::Num7 => "7",
                CardValue::Num8 => "8",
                CardValue::Num9 => "9",
                CardValue::Jack => "J",
                CardValue::Queen => "Q",
                CardValue::King => "K",
                CardValue::Num10 => "10",
                CardValue::Ace => "A",
            }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Suit {
    Clubs,
    Spades,
    Hearts,
    Diamonds,
}

impl Suit {
    // FIXME: Replace with std::mem::variant_count when stabilized.
    pub(crate) const COUNT: usize = 4;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [Self::Clubs, Self::Spades, Self::Hearts, Self::Diamonds]
    }

    /// Parses a suit.
    ///
    /// The input could be either `C`, `S`, `H`, or `D` ignoring case.
    fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "suit",
            alt((
                value(Self::Clubs, tag_no_case("C")),
                value(Self::Spades, tag_no_case("S")),
                value(Self::Hearts, tag_no_case("H")),
                value(Self::Diamonds, tag_no_case("D")),
            )),
        )(input)
    }
}

impl Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Suit::Clubs => "C",
                Suit::Spades => "S",
                Suit::Hearts => "H",
                Suit::Diamonds => "D",
            }
        )
    }
}

// FIXME: Fit into a single byte.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Card(CardValue, Suit);

impl Card {
    pub(crate) const COUNT: usize = Suit::COUNT * CardValue::COUNT;
    /// The number of bits needed to encode a [`Self`].
    const BITS: u32 = (Self::COUNT - 1).ilog2() + 1;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        let mut cards = [Self(CardValue::Num7, Suit::Clubs); Self::COUNT];
        let mut suit = 0;
        while suit < Suit::COUNT {
            let mut value = 0;
            while value < CardValue::COUNT {
                let card = Self(CardValue::all()[value], Suit::all()[suit]);
                cards[card.index()] = card;
                value += 1;
            }
            suit += 1;
        }
        cards
    }

    /// Returns the index of `self` into [`Self::all()`].
    pub(crate) const fn index(&self) -> usize {
        self.0 as usize * Suit::COUNT + self.1 as usize
    }

    /// Parses a card value followed by its suit.
    pub(crate) fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "card",
            map(
                separated_pair(CardValue::parse, space0, cut(Suit::parse)),
                |(v, s)| Self(v, s),
            ),
        )(input)
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

impl From<Card> for move_code {
    /// Just use the lower [`Self::BITS`] bits for representing this card.
    fn from(value: Card) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(move_code::MAX == MOVE_NONE);
        assert!(move_code::try_from(Card::COUNT).is_ok());

        value.index() as move_code
    }
}

impl TryFrom<move_code> for Card {
    type Error = Error;

    fn try_from(value: move_code) -> Result<Self, Self::Error> {
        usize::try_from(value)
            .ok()
            .and_then(|v| Card::all().get(v).cloned())
            .ok_or_else(|| {
                Error::new_static(ErrorCode::InvalidMove, "card value in move too high\0")
            })
    }
}

/// This represents a card which can have a known value or a hidden one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptCard {
    Hidden,
    Known(Card),
}

impl OptCard {
    /// The number of bits needed to encode a [`Self`].
    const BITS: u32 = Card::BITS + 1;
    pub const HIDDEN: move_code = 1 << Card::BITS;

    /// Parses a string to a card interpreting `?` as [`Self::Hidden`].
    pub(crate) fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "optional card",
            alt((
                value(Self::Hidden, char('?')),
                map(Card::parse, Self::Known),
            )),
        )(input)
    }

    fn ok(self) -> Option<Card> {
        match self {
            OptCard::Hidden => None,
            OptCard::Known(card) => Some(card),
        }
    }
}

impl Display for OptCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hidden => write!(f, "?"),
            Self::Known(c) => c.fmt(f),
        }
    }
}

impl From<Card> for OptCard {
    fn from(value: Card) -> Self {
        Self::Known(value)
    }
}

impl IntoIterator for OptCard {
    type Item = Card;
    type IntoIter = std::option::IntoIter<Card>;

    fn into_iter(self) -> Self::IntoIter {
        self.ok().into_iter()
    }
}

impl From<OptCard> for move_code {
    /// Transform [`OptCard`] into a [`move_code`].
    ///
    /// # Encoding
    /// [`OptCard`] is encoded as a [`move_code`] in the following way:
    /// ```text
    /// HSB 0...HCCCCC LSB
    ///         ║╚╩╩╩╩ Card index if not an action
    ///         ╚ 1 if hidden
    /// ```
    fn from(value: OptCard) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(OptCard::BITS <= move_code::BITS);

        match value {
            OptCard::Hidden => OptCard::HIDDEN,
            OptCard::Known(card) => card.into(),
        }
    }
}

impl From<OptCard> for MoveCode {
    fn from(value: OptCard) -> Self {
        move_code::from(value).into()
    }
}

impl TryFrom<move_code> for OptCard {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        Ok(if value == Self::HIDDEN {
            Self::Hidden
        } else {
            Self::Known(value.try_into()?)
        })
    }
}

impl FromStr for OptCard {
    type Err = Error;

    /// Parses into a [`Self`] like [`Self::parse()`] but with trimming.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(
            terminated(delimited(space0, OptCard::parse, space0), eof)(s)
                .finish()
                .map_err(|e| {
                    Error::new_dynamic(
                        ErrorCode::InvalidInput,
                        format!("failed to parse optional card:\n{}", convert_error(s, e)),
                    )
                })?
                .1,
        )
    }
}

/// A vector of [`OptCard`]s with helper functionality.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub(crate) struct CardVec(Vec<OptCard>);

impl CardVec {
    fn iter_known(&self) -> impl Iterator<Item = Card> + '_ {
        self.iter().cloned().flatten()
    }
}

impl Deref for CardVec {
    type Target = Vec<OptCard>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CardVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for CardVec {
    /// Write a space separated list of cards.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, card) in self.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{card}")?;
        }
        Ok(())
    }
}

// FIXME: Replace vectors with some array vectors.
#[derive(Default, Clone, Debug)]
pub(crate) struct CardStruct {
    /// # Invariants
    /// At most [`Self::HAND_SIZE`]`+`[`Self::SKAT_SIZE`] cards per hand.
    pub(crate) hands: [CardVec; Player::COUNT],
    /// # Invariants
    /// At most [`Self::SKAT_SIZE`] cards per hand.
    pub(crate) skat: CardVec,
    /// # Invariants
    /// At most [`Self::TRICK_SIZE`]`-1` cards per hand.
    pub(crate) trick: Vec<Card>,
    pub(crate) last_trick: Option<[Card; Self::TRICK_SIZE]>,
}

impl CardStruct {
    const HAND_SIZE: usize = 10;
    const SKAT_SIZE: usize = 2;
    const TRICK_SIZE: usize = 3;

    pub(crate) fn iter(&self) -> impl Iterator<Item = Card> + '_ {
        self.hands
            .iter()
            .flat_map(|h| h.iter_known())
            .chain(self.skat.iter_known())
            .chain(self.trick.iter().cloned())
            .chain(self.last_trick.iter().flat_map(|t| t.iter().cloned()))
    }

    pub(crate) fn iter_unknown(&self) -> impl Iterator<Item = Card> + '_ {
        let mut unknown = [true; Card::COUNT];
        for card in self.iter() {
            unknown[card.index()] = false;
        }

        Card::all()
            .into_iter()
            .zip(unknown.into_iter())
            .filter_map(|(c, u)| u.then_some(c))
    }

    /// Give the `target` a `card`.
    ///
    /// The target can be a [`Player`] or [`None`] for the Skat.
    pub(crate) fn give(&mut self, target: Option<Player>, card: OptCard) {
        match target {
            Some(player) => self.hands[player as usize].push(card),
            None => self.skat.push(card),
        }
    }

    /// Count the number of managed cards.
    ///
    /// This is useful in the dealing phase to find the number of dealt cards.
    pub(crate) fn count(&self) -> u8 {
        let count: usize = self.hands.iter().map(|v| v.len()).sum::<usize>()
            + self.skat.len()
            + self.trick.len()
            + self.last_trick.map(|t| t.len()).unwrap_or_default();
        count.try_into().expect("too many cards in card structure")
    }

    /// Redact hidden information like hands and the Skat.
    ///
    /// This keeps the state of players for which `keep[player_index]` is
    /// `true`.
    pub(crate) fn redact(&mut self, keep: [bool; Player::COUNT]) {
        for (player, _) in keep.into_iter().enumerate().filter(|&(_, k)| !k) {
            self.hands[player] = Default::default();
        }
        self.skat = Default::default();
    }
}

impl Display for CardStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for player in Player::all() {
            write!(f, "{player}: {}", self.hands[player as usize])?;
            writeln!(f)?;
        }

        write!(f, "Skat: {}", self.skat)?;

        if !self.trick.is_empty() {
            writeln!(f)?;
            write!(f, "current trick:")?;
            for card in &self.trick {
                write!(f, " {card}")?;
            }
        }

        if let Some(trick) = self.last_trick {
            writeln!(f)?;
            write!(f, "last trick:")?;
            for card in trick {
                write!(f, " {card}")?;
            }
        }

        Ok(())
    }
}
