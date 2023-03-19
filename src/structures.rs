use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0},
    combinator::{map, value, cut},
    error::{context, VerboseError},
    sequence::{separated_pair, tuple},
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
}

pub(crate) type CardVec = Vec<Option<Card>>;

// FIXME: Replace vectors with some array vectors.
#[derive(Default, Clone, Debug)]
pub(crate) struct CardStruct {
    pub(crate) hands: [CardVec; Player::COUNT],
    pub(crate) skat: CardVec,
    pub(crate) trick: Vec<Card>,
    pub(crate) last_trick: Option<[Card; 3]>,
}

impl CardStruct {
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
}
