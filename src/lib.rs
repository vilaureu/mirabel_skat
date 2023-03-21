//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

mod structures;

use std::{
    fmt::{self, Display, Write},
    str::FromStr,
};

use mirabel::{
    cstr,
    error::{Error, ErrorCode, Result},
    game::{
        move_code, player_id, semver, GameFeatures, GameMethods, Metadata, MoveCode, MoveData,
        MOVE_NONE, PLAYER_NONE, PLAYER_RAND,
    },
    game_init::GameInit,
    plugin_get_game_methods, MoveDataSync,
};
use nom::{
    character::complete::space0,
    combinator::eof,
    error::convert_error,
    sequence::{delimited, terminated},
    Finish,
};
use structures::{Card, CardStruct, Player};

#[derive(Clone, Copy, Debug, Default)]
enum GameState {
    #[default]
    Dealing,
    Bidding,
    Declaring,
    Playing,
}

#[derive(Clone, Debug)]
struct Skat {
    cards: CardStruct,
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
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(0 == PLAYER_NONE);
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(PLAYER_RAND > 3);
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
        // FIXME: Reuse allocation or avoid dynamic allocations.
        *self = other.clone();
        Ok(())
    }

    fn player_count(&mut self) -> Result<u8> {
        Ok(Player::COUNT.try_into().unwrap())
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
            GameState::Dealing => PLAYER_RAND,
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        });
        Ok(())
    }

    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()> {
        match self.state {
            GameState::Dealing => {
                for card in self.cards.iter_unknown() {
                    moves.push(CardAction::Card(card).into())
                }
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }

        Ok(())
    }

    /// Convert a move string to a [`MoveCode`].
    ///
    /// Examples for dealing cards: `10S` for _10 of spades_ or `?` for a hidden
    /// action.
    fn get_move_data(&mut self, _player: player_id, string: &str) -> Result<Self::Move> {
        Ok(match self.state {
            GameState::Dealing => {
                let card: CardAction = string.parse()?;
                card.into()
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        })
    }

    fn get_move_str(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
        str_buf: &mut mirabel::ValidCString,
    ) -> Result<()> {
        match self.state {
            GameState::Dealing => {
                let card: CardAction = mov.md.try_into()?;
                write!(str_buf, "{card}").expect("writing card action move failed");
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }
        Ok(())
    }

    fn make_move(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
    ) -> Result<()> {
        match &mut self.state {
            GameState::Dealing => {
                assert_eq!(PLAYER_RAND, player);
                let card = mov.md.try_into()?;
                let dealt = self.cards.count();
                let target = deal_to(dealt);
                self.cards.give(
                    target,
                    match card {
                        CardAction::Hidden => {
                            // Add an unknown card (None) to the player.
                            None
                        }
                        CardAction::Card(card) => Some(card),
                    },
                );
                if usize::from(dealt) + 1 >= Card::COUNT {
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
            GameState::Dealing => {
                if player != PLAYER_RAND {
                    return Err(Error::new_static(
                        ErrorCode::InvalidPlayer,
                        "only PLAYER_RAND can deal cards\0",
                    ));
                }
                let card = mov.md.try_into()?;
                if let CardAction::Card(card) = card {
                    if self.cards.iter().any(|c| c == card) {
                        return Err(Error::new_static(
                            ErrorCode::InvalidMove,
                            "this card has already been dealt\0",
                        ));
                    }
                }
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }

        Ok(())
    }

    fn get_concrete_move_probabilities(
        &mut self,
        move_probabilities: &mut Vec<std::ffi::c_float>,
    ) -> Result<()> {
        // FIXME: Replace with a fixed-capacity array vector.
        let mut moves = vec![];
        self.get_concrete_moves(PLAYER_RAND, &mut moves)?;
        for _ in &moves {
            move_probabilities.push(1f32 / moves.len() as f32);
        }
        Ok(())
    }

    fn get_actions(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()> {
        todo!()
    }

    fn move_to_action(
        &mut self,
        player: player_id,
        mov: MoveDataSync<<Self::Move as MoveData>::Rust<'_>>,
        target_player: player_id,
    ) -> Result<Self::Move> {
        // Catch misuse of this function and behave as the identity in this
        // case.
        if player == target_player || target_player == PLAYER_RAND {
            return Ok(mov.md.into());
        }

        match self.state {
            GameState::Dealing => {
                assert_eq!(PLAYER_RAND, player);
                let target = deal_to(self.cards.count());
                if target
                    .filter(|&t| t == self.player_from_id(target_player))
                    .is_some()
                {
                    Ok(mov.md.into())
                } else {
                    Ok(CardAction::HIDDEN.into())
                }
            }
            GameState::Bidding => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }
    }

    fn get_random_move(&mut self, seed: u64) -> Result<Self::Move> {
        // FIXME: Replace with a fixed-capacity array vector.
        let mut moves = vec![];
        self.get_concrete_moves(PLAYER_RAND, &mut moves)?;
        Ok(moves[seed as usize % moves.len()])
    }

    fn redact_keep_state(&mut self, players: &[player_id]) -> Result<()> {
        let mut keep = [false; Player::COUNT];
        for &player in players {
            keep[self.player_from_id(player) as usize] = true;
        }
        self.cards.redact(keep);
        Ok(())
    }
}

impl Skat {
    /// Returns the [`Player`] corresponding to the [`player_id`] for this game.
    ///
    /// # Panics
    /// Panics if `player` is out of range.
    fn player_from_id(&self, player: player_id) -> Player {
        assert!(0 < player);
        assert!(usize::from(player) <= Player::COUNT);
        match (usize::from(player) - 1 + Player::COUNT - (usize::from(self.dealer) - 1))
            % Player::COUNT
        {
            0 => Player::Rearhand,
            1 => Player::Forehand,
            2 => Player::Middlehand,
            _ => unreachable!(),
        }
    }
}

/// Representation of a card move which could also be a hidden action.
///
/// # Encoding
/// [`Self`] is encoded as a [`move_code`] in the following way:
/// ```text
/// HSB X0...0XXXXX LSB
///     ║     ╚╩╩╩╩ Card index if not an action
///     ╚ 1 if action
/// ```
#[derive(Clone, Copy, Debug)]
enum CardAction {
    Hidden,
    Card(Card),
}

impl CardAction {
    const HIDDEN: move_code = (0b1 as move_code).reverse_bits();
}

impl From<CardAction> for move_code {
    fn from(value: CardAction) -> Self {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(move_code::MAX == MOVE_NONE);
        // The highest bit in a move_code must never be set for a card.
        assert!(move_code::try_from(Card::COUNT - 1).unwrap() < CardAction::HIDDEN);
        match value {
            CardAction::Hidden => CardAction::HIDDEN,
            CardAction::Card(card) => card.index() as move_code,
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
        Ok(if value == Self::HIDDEN {
            Self::Hidden
        } else {
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

impl FromStr for CardAction {
    type Err = Error;

    /// Parses into a card action like [`Card::parse_optional()`].
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (_, card) = terminated(delimited(space0, Card::parse_optional, space0), eof)(s)
            .finish()
            .map_err(|e| {
                Error::new_dynamic(
                    ErrorCode::InvalidInput,
                    format!("failed to parse card action:\n{}", convert_error(s, e)),
                )
            })?;
        Ok(match card {
            Some(c) => Self::Card(c),
            None => Self::Hidden,
        })
    }
}

impl Display for CardAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            CardAction::Hidden => Card::fmt_optional(f, None),
            CardAction::Card(c) => Card::fmt_optional(f, Some(c)),
        }
    }
}

/// Returns the player to which should be dealt next.
///
/// `dealt` is the number of already dealt cards.
/// The returned value is either a [`Player`] or [`None`] for the Skat.
///
/// # Panics
/// Panics if `dealt` is out of range.
fn deal_to(dealt: u8) -> Option<Player> {
    match dealt {
        0..=2 | 11..=14 | 23..=25 => Some(Player::Forehand),
        3..=5 | 15..=18 | 26..=28 => Some(Player::Middlehand),
        6..=8 | 19..=22 | 29..=31 => Some(Player::Rearhand),
        9..=10 => None,
        32.. => panic!("dealt too many cards"),
    }
}

fn generate_metadata() -> Metadata {
    Metadata {
        game_name: cstr("Skat\0"),
        variant_name: cstr("Standard\0"),
        impl_name: cstr("vilaureu\0"),
        version: semver {
            major: 0,
            minor: 1,
            patch: 0,
        },
        features: GameFeatures {
            random_moves: true,
            hidden_information: true,
            ..Default::default()
        },
    }
}

plugin_get_game_methods!(Skat{generate_metadata()});
