//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

use mirabel::{
    error::{Error, ErrorCode, Result},
    game::{
        move_code, player_id, GameMethods, MoveCode, MoveData, MOVE_NONE, PLAYER_NONE, PLAYER_RAND,
    },
    game_init::GameInit,
    MoveDataSync,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Player {
    Forehand,
    Middlehand,
    Rearhand,
}

impl Player {
    fn count() -> u8 {
        todo!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CardLocation {
    NotInGame,
    Player(Player),
    Skat,
    Unknown,
}

impl Default for CardLocation {
    fn default() -> Self {
        Self::NotInGame
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Suit {
    Clubs,
    Spades,
    Hearts,
    Diamonds,
}

impl Suit {
    const fn count() -> usize {
        // FIXME: Replace with std::mem::variant_count when stabilized.
        4
    }

    const fn all() -> [Self; Self::count()] {
        [Self::Clubs, Self::Spades, Self::Hearts, Self::Diamonds]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CardValue {
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
    const fn count() -> usize {
        8
    }

    const fn all() -> [Self; Self::count()] {
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Card(Suit, CardValue);

impl Card {
    const fn count() -> usize {
        Suit::count() * CardValue::count()
    }

    const fn all() -> [Self; Self::count()] {
        let mut cards = [Self(Suit::Clubs, CardValue::Num7); Self::count()];
        let mut suit = 0;
        while suit < Suit::count() {
            let mut value = 0;
            while value < CardValue::count() {
                let card = Self(Suit::all()[suit], CardValue::all()[value]);
                cards[card.index()] = card;
                value += 1;
            }
            suit += 1;
        }
        cards
    }

    /// Returns the index of `self` into [`Self::all()`].
    const fn index(&self) -> usize {
        self.0 as usize * CardValue::count() + self.1 as usize
    }
}

#[derive(Clone, Copy, Debug)]
enum GameState {
    Dealing {
        /// # Invariants
        /// This must be a in the range from `0` to excluding [`Card::count()`].
        dealt: u8,
    },
    Bidding,
    Declaring,
    Playing,
}

impl Default for GameState {
    fn default() -> Self {
        Self::Dealing { dealt: 0 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Skat {
    cards: [CardLocation; Card::count()],
    /// # Invariants
    /// This must be a valid player.
    dealer: player_id,
    state: GameState,
}

impl PartialEq for Skat {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Skat {}

impl Default for Skat {
    fn default() -> Self {
        assert_eq!(0, PLAYER_NONE);
        assert!(PLAYER_RAND > 3);
        Self {
            cards: Default::default(),
            dealer: 1,
            state: Default::default(),
        }
    }
}

impl GameMethods for Skat {
    type Move = MoveCode;

    fn create(init_info: &GameInit) -> Result<Self> {
        Ok(match init_info {
            GameInit::Default => Self::default(),
            GameInit::Standard {
                opts,
                legacy,
                state,
            } => todo!(),
            GameInit::Serialized(_) => todo!(),
        })
    }

    fn copy_from(&mut self, other: &mut Self) -> Result<()> {
        *self = *other;
        Ok(())
    }

    fn player_count(&mut self) -> Result<u8> {
        Ok(Player::count())
    }

    fn import_state(&mut self, string: Option<&str>) -> Result<()> {
        todo!()
    }

    fn export_state(
        &mut self,
        player: player_id,
        str_buf: &mut mirabel::ValidCString,
    ) -> Result<()> {
        todo!()
    }

    fn players_to_move(&mut self, players: &mut Vec<player_id>) -> Result<()> {
        players.push(match self.state {
            GameState::Dealing { dealt: _ } => PLAYER_RAND,
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        });
        Ok(())
    }

    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()> {
        match self.state {
            GameState::Dealing { dealt } => {
                for (index, &location) in self.cards.iter().enumerate() {
                    assert_eq!(PLAYER_RAND, player);
                    if location == CardLocation::NotInGame {
                        moves.push(CardAction::new(index).into());
                    }
                }
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }

        Ok(())
    }

    fn get_move_data(&mut self, player: player_id, string: &str) -> Result<Self::Move> {
        todo!()
    }

    fn get_move_str(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
        str_buf: &mut mirabel::ValidCString,
    ) -> Result<()> {
        todo!()
    }

    fn make_move(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
    ) -> Result<()> {
        match &mut self.state {
            GameState::Dealing { dealt } => {
                assert_eq!(PLAYER_RAND, player);
                let card: CardAction = mov.md.try_into()?;
                match card {
                    CardAction::Hidden => {
                        // Because any card currently not in the game could have
                        // been dealt, all these card locations become unknown.
                        for location in self
                            .cards
                            .iter_mut()
                            .filter(|&&mut c| c == CardLocation::NotInGame)
                        {
                            *location = CardLocation::Unknown
                        }
                    }
                    CardAction::Card(card) => {
                        self.cards[card.index()] = deal_to(*dealt);
                    }
                }
                *dealt += 1;
                if usize::from(*dealt) >= Card::count() {
                    self.state = GameState::Bidding;
                }
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }

        Ok(())
    }

    fn get_results(&mut self, players: &mut Vec<player_id>) -> Result<()> {
        todo!()
    }

    fn is_legal_move(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
    ) -> Result<()> {
        match &mut self.state {
            GameState::Dealing { dealt } => {
                if player != PLAYER_RAND {
                    return Err(Error::new_static(
                        ErrorCode::InvalidPlayer,
                        "only PLAYER_RAND can deal cards\0",
                    ));
                }
                todo!()
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }

        Ok(())
    }
}

/// Representation of a card move which could also be a hidden action.
///
/// # Encoding
/// [`Self`] is encoded as a [`move_code`] in the following way:
/// ```text
/// HSB 1X0...0XXXXX LSB
///      ║    ╚╩╩╩╩ Card index if not an action
///      ╚ 1 if action
/// ```
#[derive(Clone, Copy, Debug)]
enum CardAction {
    Hidden,
    Card(Card),
}

impl CardAction {
    const MASK: move_code = (0b1 as move_code).reverse_bits();
    const HIDDEN: move_code = (0b11 as move_code).reverse_bits();

    /// # Panics
    /// Panics if `index` is out of range.
    fn new(index: usize) -> Self {
        Self::Card(Card::all()[index])
    }
}

impl From<CardAction> for move_code {
    fn from(value: CardAction) -> Self {
        assert_eq!(0, MOVE_NONE);
        // The highest two bits in a move_code must never be set for a card.
        assert_eq!(
            0,
            move_code::try_from(Card::count() - 1).unwrap() & CardAction::HIDDEN
        );
        match value {
            CardAction::Hidden => CardAction::HIDDEN,
            CardAction::Card(card) => CardAction::MASK | card.index() as move_code,
        }
    }
}

impl From<CardAction> for MoveCode {
    fn from(value: CardAction) -> Self {
        move_code::from(value).into()
    }
}

impl TryFrom<move_code> for CardAction {
    type Error = Error;

    fn try_from(value: move_code) -> std::result::Result<Self, Self::Error> {
        if value & (0b1 as move_code).reverse_bits() == 0 {
            return Err(Error::new_static(
                ErrorCode::InvalidMove,
                "card actions must have MSB set\0",
            ));
        }
        Ok(if value == Self::HIDDEN {
            Self::Hidden
        } else {
            let value = value & !Self::MASK;
            Self::Card(
                usize::try_from(value)
                    .ok()
                    .and_then(|v| Card::all().get(v).cloned())
                    .ok_or_else(|| {
                        Error::new_static(ErrorCode::InvalidMove, "card value in move too high\0")
                    })?,
            )
        })
    }
}

/// Returns the [`CardLocation`] to which should be dealt next.
///
/// `dealt` is the number of already dealt cards.
/// The returned location is either [`CardLocation::Player`] or
/// [`CardLocation::Skat`].
///
/// # Panics
/// Panics if `dealt` is out of range.
fn deal_to(dealt: u8) -> CardLocation {
    match dealt {
        0..=2 | 11..=14 | 23..=25 => CardLocation::Player(Player::Forehand),
        3..=5 | 15..=18 | 26..=28 => CardLocation::Player(Player::Middlehand),
        6..=8 | 19..=22 | 29..=31 => CardLocation::Player(Player::Rearhand),
        9..=10 => CardLocation::Skat,
        32.. => panic!("dealt too many cards"),
    }
}
