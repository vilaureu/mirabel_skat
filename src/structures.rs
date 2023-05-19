use std::{
    cmp::Ordering,
    fmt::{self, Display},
    ops::{Deref, DerefMut, Index, IndexMut},
    str::FromStr,
};

use mirabel::{
    error::{Error, ErrorCode, Result},
    game::{move_code, player_id, MoveCode, MOVE_NONE, PLAYER_NONE, PLAYER_RAND},
};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, space0, space1},
    combinator::{cut, eof, map, opt, value},
    error::{context, convert_error, VerboseError},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
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

    /// Return the other two players.
    pub const fn others(&self) -> [Self; Self::COUNT - 1] {
        let all = Self::all();
        let mut others = [Self::Forehand; Self::COUNT - 1];
        let (mut a, mut o) = (0, 0);
        while a < Self::COUNT {
            if *self as usize != all[a] as usize {
                others[o] = all[a];
                o += 1;
            }
            a += 1;
        }
        others
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

/// The value of cards.
///
/// [`Ord`] follows the ordering of a Null game with [`Self::Ace`] being the
/// lowest.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum CardValue {
    Ace,
    King,
    Queen,
    Jack,
    Num10,
    Num9,
    Num8,
    Num7,
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

    /// Returns the ordinal number of this card value in a regular game.
    ///
    /// Precisely, _Ace_ will be mapped to _0_, _10_  to _1_, and so on until
    /// _7_ is mapped to _6_.
    ///
    /// # Panics
    /// Panics if invoked on [`Self::Jack`].
    const fn ordinal(&self) -> usize {
        match self {
            CardValue::Ace => 0,
            CardValue::Num10 => 1,
            CardValue::King => 2,
            CardValue::Queen => 3,
            CardValue::Num9 => 4,
            CardValue::Num8 => 5,
            CardValue::Num7 => 6,
            CardValue::Jack => panic!("jacks are no regular values in a normal game"),
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum Suit {
    Clubs,
    Spades,
    Hearts,
    Diamonds,
}

impl Suit {
    // FIXME: Replace with std::mem::variant_count when stabilized.
    pub(crate) const COUNT: usize = 4;
    const BITS: u32 = count_bits(Self::COUNT);

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
    const BITS: u32 = count_bits(Self::COUNT);

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

    /// Orders the cards with the jack of clubs being the lowest.
    fn cmp(&self, other: &Card) -> Ordering {
        let self_jack = matches!(self.0, CardValue::Jack);
        let other_jack = matches!(other.0, CardValue::Jack);
        if self_jack && other_jack {
            self.1.cmp(&other.1)
        } else if self_jack && !other_jack {
            Ordering::Less
        } else if !self_jack && other_jack {
            Ordering::Greater
        } else if matches!(self.1.cmp(&other.1), Ordering::Equal) {
            self.0.ordinal().cmp(&other.0.ordinal())
        } else {
            self.1.cmp(&other.1)
        }
    }

    /// Sort according to a Null game with the ace of clubs being the lowest.
    fn cmp_null(&self, other: &Card) -> Ordering {
        let ordering_suit = self.1.cmp(&other.1);
        if matches!(ordering_suit, Ordering::Equal) {
            self.0.cmp(&other.0)
        } else {
            ordering_suit
        }
    }

    fn trump_suit(&self, declaration: Declaration) -> TrumpSuit {
        match declaration {
            Declaration::Normal(_, _) if matches!(self.0, CardValue::Jack) => TrumpSuit::Trump,
            Declaration::Normal(NormalMode::Color(suit), _) if suit == self.1 => TrumpSuit::Trump,
            _ => TrumpSuit::Color(self.1),
        }
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

impl From<Card> for MoveCode {
    fn from(value: Card) -> Self {
        move_code::from(value).into()
    }
}

impl TryFrom<move_code> for Card {
    type Error = Error;

    fn try_from(value: move_code) -> Result<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|v| Card::all().get(v).cloned())
            .ok_or_else(|| {
                Error::new_static(ErrorCode::InvalidMove, "card value in move too high\0")
            })
    }
}

impl FromStr for Card {
    type Err = Error;

    /// Parses into a [`Self`] like [`Self::parse()`] but with trimming.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(terminated(delimited(space0, Card::parse, space0), eof)(s)
            .finish()
            .map_err(|e| {
                Error::new_dynamic(
                    ErrorCode::InvalidInput,
                    format!("failed to parse card:\n{}", convert_error(s, e)),
                )
            })?
            .1)
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

    /// Sorts with hidden cards have the highest value.
    ///
    /// See [`Card::cmp()`].
    fn cmp(&self, other: &Self, null: bool) -> Ordering {
        match (self, other) {
            (OptCard::Known(s), OptCard::Known(o)) => {
                if null {
                    s.cmp_null(o)
                } else {
                    s.cmp(o)
                }
            }
            (OptCard::Hidden, OptCard::Known(_)) => Ordering::Greater,
            (OptCard::Known(_), OptCard::Hidden) => Ordering::Less,
            (OptCard::Hidden, OptCard::Hidden) => Ordering::Equal,
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
    pub(crate) fn iter_known(&self) -> impl Iterator<Item = Card> + '_ {
        self.iter().cloned().flatten()
    }

    /// Sort in-place respecting whether this is a Null game or not.
    fn sort(&mut self, null: bool) {
        self.sort_by(|a, b| a.cmp(b, null));
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
    pub(crate) const SKAT_SIZE: usize = 2;
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

    /// Take the `card` away from `player`.
    ///
    /// If the `card` is [`OptCard::Hidden`], it redacts the `player`s cards.
    /// It then searches for the `card` in the `player`s hand and removes it.
    /// If it was not found, it removes a hidden card or, if there is no hidden
    /// card, it returns an error.
    pub(crate) fn take(&mut self, player: Player, card: OptCard) -> Result<()> {
        if matches!(card, OptCard::Hidden) {
            for card in self[player].iter_mut() {
                *card = OptCard::Hidden;
            }
        }
        let index = match self[player].iter().enumerate().find(|(_, c)| **c == card) {
            Some((i, _)) => i,
            None => {
                self[player]
                    .iter()
                    .enumerate()
                    .find(|(_, c)| matches!(c, OptCard::Hidden))
                    .ok_or(Error::new_static(
                        ErrorCode::InvalidMove,
                        "cannot take this card for this player\0",
                    ))?
                    .0
            }
        };
        self[player].swap_remove(index);
        Ok(())
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
            for card in self.hands[player].iter_mut() {
                *card = OptCard::Hidden;
            }
        }
        for card in self.skat.iter_mut() {
            *card = OptCard::Hidden;
        }
    }

    /// Sort cards in-place.
    ///
    /// `null` specified whether to sort for a Null game or for a normal game.
    pub(crate) fn sort(&mut self, null: bool) {
        for hand in self.hands.iter_mut() {
            hand.sort(null);
        }
        self.skat.sort(null);
    }

    /// Returns the [`Card`]s the [`Player`] is allowed to play.
    ///
    /// It considers the first card in the current trick if any.
    /// If any card of the player is unknown, this returns a list of their known
    /// cards and all unknown ones.
    pub(crate) fn allowed(&self, player: Player, declaration: Declaration) -> Vec<Card> {
        let hand = self[player];
        let mut allowed = Vec::with_capacity(hand.len());
        for card in hand.iter() {
            match card {
                OptCard::Hidden => return hand.iter_known().chain(self.iter_unknown()).collect(),
                OptCard::Known(c) => allowed.push(*c),
            }
        }

        let Some(first) = self.trick.get(0) else { return allowed; };
        let follow = first.trump_suit(declaration);
        let must_follow = allowed.iter().any(|c| c.trump_suit(declaration) == follow);
        if must_follow {
            allowed.retain(|c| c.trump_suit(declaration) == follow)
        }
        allowed
    }
}

impl Index<Player> for CardStruct {
    type Output = CardVec;

    fn index(&self, player: Player) -> &Self::Output {
        &self.hands[player as usize]
    }
}

impl IndexMut<Player> for CardStruct {
    fn index_mut(&mut self, player: Player) -> &mut Self::Output {
        &mut self.hands[player as usize]
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

#[derive(Default, Clone, Copy, Debug)]
pub(crate) enum Declaration {
    /// A normal game (i.e., not a _Null_ game)
    ///
    /// This set of states is encoded as:
    /// ```text
    /// HSB 0...01M...ML...L LSB
    ///          ║║║║║║╚╩╩╩╩ GameLevel
    ///          ║╚╩╩╩╩═════ NormalMode
    ///          ╚══════════ Set for Normal variant
    /// ```
    Normal(NormalMode, GameLevel),
    /// Default to a non-_Hand_ game.
    #[default]
    Null,
    NullHand,
    NullOuvert,
    NullOuvertHand,
}

impl Declaration {
    const BITS: u32 = max(NormalMode::BITS + GameLevel::BITS, 2) + 1;
    const NULL: move_code = 0;
    const NULL_HAND: move_code = 1;
    const NULL_OUVERT: move_code = 2;
    const NULL_OUVERT_HAND: move_code = 3;

    /// List all possible declarations.
    ///
    /// If `hand`, assume a _Hand_ game else assume otherwise.
    // FIXME: Replace with fixed-sized vector.
    pub(crate) fn all(hand: bool) -> Vec<Self> {
        let mut possibilities = if hand {
            vec![Self::NullHand, Self::NullOuvertHand]
        } else {
            vec![Self::Null, Self::NullOuvert]
        };
        for mode in NormalMode::all() {
            for &level in GameLevel::all(hand) {
                possibilities.push(Self::Normal(mode, level));
            }
        }
        possibilities
    }

    pub(crate) fn is_hand(&self) -> bool {
        match self {
            Declaration::Normal(_, l) => l.is_hand(),
            Declaration::Null => false,
            Declaration::NullHand => true,
            Declaration::NullOuvert => false,
            Declaration::NullOuvertHand => true,
        }
    }

    pub(crate) fn is_ouvert(&self) -> bool {
        matches!(
            self,
            Declaration::Normal(_, GameLevel::Ouvert)
                | Declaration::NullOuvert
                | Declaration::NullOuvertHand,
        )
    }

    /// Is this declaration allowed given the `bid` value and number of
    /// `matadors`.
    pub(crate) fn allowed(&self, bid: u16, matadors: &Matadors) -> bool {
        match *self {
            Declaration::Normal(mode, level) => {
                // Add 2 for possibly playing Schneider and Schwarz.
                bid <= (u16::from(matadors[mode]) + u16::from(level) + 2) * u16::from(mode)
            }
            Declaration::Null => bid <= 23,
            Declaration::NullHand => bid <= 35,
            Declaration::NullOuvert => bid <= 46,
            Declaration::NullOuvertHand => bid <= 59,
        }
    }

    pub(crate) fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "declaration",
            alt((
                value(
                    Self::NullOuvertHand,
                    tuple((
                        tag_no_case("null"),
                        space1,
                        tag_no_case("ouvert"),
                        space1,
                        tag_no_case("hand"),
                    )),
                ),
                value(
                    Self::NullOuvert,
                    separated_pair(tag_no_case("null"), space1, tag_no_case("ouvert")),
                ),
                value(
                    Self::NullHand,
                    separated_pair(tag_no_case("null"), space1, tag_no_case("hand")),
                ),
                value(Self::Null, tag_no_case("null")),
                map(
                    pair(
                        cut(NormalMode::parse),
                        opt(preceded(
                            space1,
                            context(
                                "level",
                                alt((
                                    value(GameLevel::Hand, tag_no_case("hand")),
                                    value(GameLevel::Schneider, tag_no_case("schneider")),
                                    value(GameLevel::Schwarz, tag_no_case("schwarz")),
                                    value(GameLevel::Ouvert, tag_no_case("ouvert")),
                                )),
                            ),
                        )),
                    ),
                    |(m, l)| Self::Normal(m, l.unwrap_or(GameLevel::Normal)),
                ),
            )),
        )(input)
    }

    pub(crate) fn is_null(&self) -> bool {
        !matches!(self, Self::Normal(_, _))
    }
}

impl From<Declaration> for move_code {
    fn from(value: Declaration) -> Self {
        match value {
            Declaration::Normal(mode, level) => {
                (1 << (Declaration::BITS - 1))
                    + (move_code::from(mode) << GameLevel::BITS)
                    + move_code::from(level)
            }
            Declaration::Null => Declaration::NULL,
            Declaration::NullHand => Declaration::NULL_HAND,
            Declaration::NullOuvert => Declaration::NULL_OUVERT,
            Declaration::NullOuvertHand => Declaration::NULL_OUVERT_HAND,
        }
    }
}

impl TryFrom<move_code> for Declaration {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            Self::NULL => Self::Null,
            Self::NULL_HAND => Self::NullHand,
            Self::NULL_OUVERT => Self::NullOuvert,
            Self::NULL_OUVERT_HAND => Self::NullOuvertHand,
            _ => {
                if value >> Declaration::BITS != 0 || value & (1 << (Declaration::BITS - 1)) == 0 {
                    return Err(Error::new_static(
                        ErrorCode::InvalidMove,
                        "invalid declaration move\0",
                    ));
                }
                let level_value = value & ((1 << GameLevel::BITS) - 1);
                let mode_value = (value >> GameLevel::BITS) & ((1 << NormalMode::BITS) - 1);
                Self::Normal(mode_value.try_into()?, level_value.try_into()?)
            }
        })
    }
}

impl Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Declaration::Normal(m, l) => {
                write!(f, "{m}")?;
                match l {
                    GameLevel::Normal => Ok(()),
                    GameLevel::Hand => write!(f, " Hand"),
                    GameLevel::Schneider => write!(f, " Schneider"),
                    GameLevel::Schwarz => write!(f, " Schwarz"),
                    GameLevel::Ouvert => write!(f, " Ouvert"),
                }
            }
            Declaration::Null => write!(f, "Null"),
            Declaration::NullHand => write!(f, "Null Hand"),
            Declaration::NullOuvert => write!(f, "Null Ouvert"),
            Declaration::NullOuvertHand => write!(f, "Null Ouvert Hand"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NormalMode {
    Color(Suit),
    Grand,
}

impl NormalMode {
    const BITS: u32 = Suit::BITS + 1;

    const fn all() -> [Self; Suit::COUNT + 1] {
        let mut result = [Self::Grand; Suit::COUNT + 1];
        let mut i = 0;
        while i < Suit::COUNT {
            result[i] = Self::Color(Suit::all()[i]);
            i += 1;
        }
        result[Suit::COUNT] = Self::Grand;
        result
    }

    pub(crate) fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "mode",
            alt((
                value(Self::Grand, tag_no_case("grand")),
                value(Self::Color(Suit::Clubs), tag_no_case("clubs")),
                value(Self::Color(Suit::Spades), tag_no_case("spades")),
                value(Self::Color(Suit::Hearts), tag_no_case("hearts")),
                value(Self::Color(Suit::Diamonds), tag_no_case("diamonds")),
            )),
        )(input)
    }
}

impl From<NormalMode> for u16 {
    fn from(value: NormalMode) -> Self {
        match value {
            NormalMode::Color(Suit::Diamonds) => 9,
            NormalMode::Color(Suit::Hearts) => 10,
            NormalMode::Color(Suit::Spades) => 11,
            NormalMode::Color(Suit::Clubs) => 12,
            NormalMode::Grand => 24,
        }
    }
}

impl From<NormalMode> for move_code {
    fn from(value: NormalMode) -> Self {
        match value {
            NormalMode::Color(suit) => suit as move_code,
            NormalMode::Grand => 1 << Suit::BITS,
        }
    }
}

impl TryFrom<move_code> for NormalMode {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        usize::try_from(value)
            .ok()
            .and_then(|index| Self::all().get(index).cloned())
            .ok_or(Error::new_static(
                ErrorCode::InvalidMove,
                "invalid normal game mode\0",
            ))
    }
}

impl Display for NormalMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalMode::Color(Suit::Clubs) => write!(f, "clubs"),
            NormalMode::Color(Suit::Spades) => write!(f, "spades"),
            NormalMode::Color(Suit::Hearts) => write!(f, "hearts"),
            NormalMode::Color(Suit::Diamonds) => write!(f, "diamonds"),
            NormalMode::Grand => write!(f, "grand"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GameLevel {
    Normal,
    Hand,
    Schneider,
    Schwarz,
    Ouvert,
}

impl GameLevel {
    // FIXME: Replace with std::mem::variant_count when stabilized.
    const COUNT: usize = 5;
    const BITS: u32 = count_bits(Self::COUNT);

    const fn all(hand: bool) -> &'static [Self] {
        if hand {
            &[Self::Hand, Self::Schneider, Self::Schwarz, Self::Ouvert]
        } else {
            &[Self::Normal]
        }
    }

    fn is_hand(&self) -> bool {
        !matches!(self, GameLevel::Normal)
    }
}

impl From<GameLevel> for u16 {
    fn from(value: GameLevel) -> Self {
        match value {
            GameLevel::Normal => 1,
            GameLevel::Hand => 2,
            GameLevel::Schneider => 3,
            GameLevel::Schwarz => 4,
            GameLevel::Ouvert => 5,
        }
    }
}

impl From<GameLevel> for move_code {
    fn from(value: GameLevel) -> Self {
        value as move_code
    }
}

impl TryFrom<move_code> for GameLevel {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            0 => GameLevel::Normal,
            1 => GameLevel::Hand,
            2 => GameLevel::Schneider,
            3 => GameLevel::Schwarz,
            4 => GameLevel::Ouvert,
            5.. => {
                return Err(Error::new_static(
                    ErrorCode::InvalidMove,
                    "invalid game level\0",
                ))
            }
        })
    }
}

/// Count of the (missing) matadors per suit.
pub(crate) struct Matadors([u8; Suit::COUNT]);
impl Matadors {
    pub(crate) fn from_cards(cards: impl Iterator<Item = Card>) -> Self {
        let mut jacks = [false; Suit::COUNT];
        let mut colors = [[false; CardValue::COUNT - 1]; Suit::COUNT];

        for Card(value, suit) in cards {
            let idx = suit as usize;
            if matches!(value, CardValue::Jack) {
                jacks[idx] = true;
            } else {
                colors[idx][value.ordinal()] = true;
            }
        }

        let with = jacks[0];
        let mut matadors = [0; Suit::COUNT];
        for (i, m) in matadors.iter_mut().enumerate() {
            for &has in jacks.iter().chain(colors[i].iter()) {
                if has == with {
                    *m += 1;
                } else {
                    break;
                }
            }
        }
        Self(matadors)
    }
}

impl Index<NormalMode> for Matadors {
    type Output = u8;

    fn index(&self, index: NormalMode) -> &Self::Output {
        match index {
            NormalMode::Color(suit) => &self.0[suit as usize],
            NormalMode::Grand => self.0.iter().min().unwrap().min(&(Suit::COUNT as u8)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DeclarationMove {
    Declare(Declaration),
    Overbidden,
}

impl DeclarationMove {
    const OVERBIDDEN: move_code = 1 << Declaration::BITS;

    /// Parse a declaration move from string.
    ///
    /// # Examples
    /// These moves can be parsed: `cLubs`, `null  Ouvert hand`,
    /// `grand sChWaRz`, `overbidden`.
    /// However, these do not parse: `null hand ouvert`, `grand offen`.
    pub(crate) fn parse(input: &str) -> IResult<&str, Self> {
        context(
            "declaration move",
            alt((
                value(Self::Overbidden, tag_no_case("overbidden")),
                map(Declaration::parse, Self::Declare),
            )),
        )(input)
    }
}

impl From<DeclarationMove> for move_code {
    fn from(value: DeclarationMove) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(Declaration::BITS < move_code::BITS);

        match value {
            DeclarationMove::Declare(d) => d.into(),
            DeclarationMove::Overbidden => DeclarationMove::OVERBIDDEN,
        }
    }
}

impl From<DeclarationMove> for MoveCode {
    fn from(value: DeclarationMove) -> Self {
        move_code::from(value).into()
    }
}

impl TryFrom<move_code> for DeclarationMove {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            Self::OVERBIDDEN => Self::Overbidden,
            _ => Self::Declare(value.try_into()?),
        })
    }
}

impl FromStr for DeclarationMove {
    type Err = Error;

    /// Parses into a [`Self`] like [`Self::parse()`] but with trimming.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(
            terminated(delimited(space0, DeclarationMove::parse, space0), eof)(s)
                .finish()
                .map_err(|e| {
                    Error::new_dynamic(
                        ErrorCode::InvalidInput,
                        format!("failed to parse declaration:\n{}", convert_error(s, e)),
                    )
                })?
                .1,
        )
    }
}

impl Display for DeclarationMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeclarationMove::Declare(declaration) => declaration.fmt(f),
            DeclarationMove::Overbidden => write!(f, "overbidden"),
        }
    }
}

/// Suit of a card including trump cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrumpSuit {
    Color(Suit),
    Trump,
}

/// Returns the number of bits required to represent `count` states.
///
/// # Panics
/// Panics in debug mode if count is zero.
const fn count_bits(count: usize) -> u32 {
    (count - 1).ilog2() + 1
}

const fn max(a: u32, b: u32) -> u32 {
    if a < b {
        b
    } else {
        a
    }
}
