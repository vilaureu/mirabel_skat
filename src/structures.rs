use std::fmt::{self, Display};

use mirabel::sys::{player_id, PLAYER_NONE, PLAYER_RAND};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0},
    combinator::{cut, map, value},
    error::{context, VerboseError},
    sequence::separated_pair,
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
        match value {
            1 => Self::Forehand,
            2 => Self::Middlehand,
            3 => Self::Rearhand,
            0 | 4.. => panic!("unexpected player id"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    /// Parses a string to a card interpreting `?` as [`None`].
    pub(crate) fn parse_optional(input: &str) -> IResult<&str, Option<Self>> {
        context(
            "optional card",
            alt((value(None, char('?')), map(Self::parse, Some))),
        )(input)
    }

    /// Inverse of [`Self::parse_optional`].
    pub(crate) fn fmt_optional(
        f: &mut fmt::Formatter,
        card: Option<Self>,
    ) -> Result<(), fmt::Error> {
        match card {
            None => write!(f, "?"),
            Some(c) => c.fmt(f),
        }
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

pub(crate) type CardVec = Vec<Option<Card>>;

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
            .flat_map(|h| h.iter().cloned())
            .chain(self.skat.iter().cloned())
            .flatten()
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
    /// The card can be a [`Card`] or [`None`] for an unknown card.
    pub(crate) fn give(&mut self, target: Option<Player>, card: Option<Card>) {
        match target {
            Some(player) => self.hands[player as usize].push(card),
            None => self.skat.push(card),
        }
    }

    /// Count the number of managed cards.
    ///
    /// This is useful in the dealing phase to find the number of dealt cards.
    pub(crate) fn count(&self) -> u8 {
        let count: usize = self.hands.iter().map(Vec::len).sum::<usize>()
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
