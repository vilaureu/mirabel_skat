//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

mod structures;

use std::fmt::{self, Display, Write};

use mirabel::{
    cstr,
    error::{Error, ErrorCode, Result},
    game::{
        player_id, semver, GameFeatures, GameMethods, Metadata, MoveCode, MoveData, PLAYER_RAND,
    },
    game_init::GameInit,
    plugin_get_game_methods, MoveDataSync,
};

use structures::{Card, CardStruct, Player};

use crate::structures::OptCard;

#[derive(Clone, Copy, Debug, Default)]
enum GameState {
    #[default]
    Dealing,
    Bidding {
        // FIXME: This could fit into 8 bytes when a offset is used.
        bid: u16,
        state: BiddingState,
    },
    Declaring,
    Playing,
}

impl GameState {
    const MINIMUM_BID: u16 = 18;
}

impl Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameState::Dealing => write!(f, "dealing"),
            GameState::Bidding { bid, state } => {
                if *bid < Self::MINIMUM_BID {
                    writeln!(f, "bidding just started")?;
                } else {
                    writeln!(f, "bidding at {bid}")?;
                }
                write!(f, "{state}")
            }
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BiddingState {
    #[default]
    MiddleCallsFore,
    ForeRespondsMiddle,
    RearCallsFore,
    ForeRespondsRear,
    RearCallsMiddle,
    MiddleRespondsRear,
    /// Forehand is free to decide whether to play or not.
    ///
    /// This happens when middlehand and rearhand directly pass.
    Forehand,
}

impl BiddingState {
    /// Returns true when `self` represents a respond to a call.
    fn respond(&self) -> bool {
        match self {
            Self::MiddleCallsFore => false,
            Self::ForeRespondsMiddle => true,
            Self::RearCallsFore => false,
            Self::ForeRespondsRear => true,
            Self::RearCallsMiddle => false,
            Self::MiddleRespondsRear => true,
            Self::Forehand => false,
        }
    }

    /// Who is currently making a statement.
    fn source(&self) -> Player {
        match self {
            Self::MiddleCallsFore => Player::Middlehand,
            Self::ForeRespondsMiddle => Player::Forehand,
            Self::RearCallsFore => Player::Rearhand,
            Self::ForeRespondsRear => Player::Forehand,
            Self::RearCallsMiddle => Player::Rearhand,
            Self::MiddleRespondsRear => Player::Middlehand,
            Self::Forehand => Player::Forehand,
        }
    }

    /// Who is currently the audience for the statement.
    ///
    /// # Panics
    /// Panics for [`Self::Forehand`].
    fn target(&self) -> Player {
        match self {
            Self::MiddleCallsFore => Player::Forehand,
            Self::ForeRespondsMiddle => Player::Middlehand,
            Self::RearCallsFore => Player::Forehand,
            Self::ForeRespondsRear => Player::Rearhand,
            Self::RearCallsMiddle => Player::Middlehand,
            Self::MiddleRespondsRear => Player::Rearhand,
            Self::Forehand => panic!("the forehand is the only one left bidding"),
        }
    }
}

impl Display for BiddingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self, Self::Forehand) {
            write!(f, "only the forehand is left bidding")
        } else {
            write!(
                f,
                "{} {} {}",
                self.source(),
                if self.respond() {
                    "should respond to"
                } else {
                    "should make a call to"
                },
                self.target()
            )
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Skat {
    cards: CardStruct,
    state: GameState,
}

impl PartialEq for Skat {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Skat {}

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
            GameState::Bidding { bid, state } => {
                // TODO
                Player::Middlehand.into()
            }
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
        });
        Ok(())
    }

    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()> {
        match self.state {
            GameState::Dealing => {
                for card in self.cards.iter_unknown() {
                    moves.push(OptCard::from(card).into())
                }
            }
            GameState::Bidding { bid, state } => todo!(),
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
                let card: OptCard = string.parse()?;
                card.into()
            }
            GameState::Bidding { bid, state } => todo!(),
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
                let card: OptCard = mov.md.try_into()?;
                write!(str_buf, "{card}").expect("writing card action move failed");
            }
            GameState::Bidding { bid, state } => todo!(),
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
                self.cards.give(target, card);
                if usize::from(dealt) + 1 >= Card::COUNT {
                    self.state = GameState::Bidding {
                        bid: GameState::MINIMUM_BID - 1,
                        state: Default::default(),
                    };
                }
            }
            GameState::Bidding { bid, state } => todo!(),
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
                if let OptCard::Known(card) = card {
                    if self.cards.iter().any(|c| c == card) {
                        return Err(Error::new_static(
                            ErrorCode::InvalidMove,
                            "this card has already been dealt\0",
                        ));
                    }
                }
            }
            GameState::Bidding { bid, state } => todo!(),
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
                    .filter(|&t| t == Player::from(target_player))
                    .is_some()
                {
                    Ok(mov.md.into())
                } else {
                    Ok(OptCard::Hidden.into())
                }
            }
            GameState::Bidding { bid, state } => todo!(),
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
            keep[Player::from(player) as usize] = true;
        }
        self.cards.redact(keep);
        Ok(())
    }

    fn print(&mut self, _player: player_id, str_buf: &mut mirabel::ValidCString) -> Result<()> {
        write!(str_buf, "{}", self).expect("failed to write to print buffer");
        Ok(())
    }
}

impl Display for Skat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.cards)?;
        writeln!(f, "{}", self.state)
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
            print: true,
            ..Default::default()
        },
    }
}

plugin_get_game_methods!(Skat{generate_metadata()});
