//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

mod structures;

use std::{
    fmt::{self, Display, Write},
    vec,
};

use mirabel::{
    cstr,
    error::{Error, ErrorCode, Result},
    game::{
        move_code, player_id, semver, GameFeatures, GameMethods, Metadata, MoveCode, MoveData,
        PLAYER_RAND,
    },
    game_init::GameInit,
    plugin_get_game_methods, MoveDataSync,
};

use structures::{Card, CardStruct, Player};

use crate::structures::OptCard;

#[derive(Clone, Debug, Default)]
enum GameState {
    /// State while dealing cards.
    #[default]
    Dealing,
    /// State of the bidding phase.
    Bidding {
        // FIXME: This could fit into 8 bytes when a offset is used.
        bid: u16,
        state: BiddingState,
    },
    /// Single player is deciding whether to look at the Skat or not.
    SkatDecision,
    Declaring,
    Playing,
    // FIXME: Replace with fixed-size array.
    Finished(Vec<Player>),
}

impl GameState {
    const MINIMUM_BID: u16 = 18;
    const MAXIMUM_BID: u16 = 264;

    /// Does the game have a declarer at this stage.
    fn has_declarer(&self) -> bool {
        !matches!(
            self,
            GameState::Dealing | GameState::Bidding { bid: _, state: _ }
        )
    }
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
            GameState::Finished(players) => {
                if players.is_empty() {
                    write!(f, "draw")
                } else {
                    write!(
                        f,
                        "{} won",
                        players
                            .iter()
                            .fold("".to_string(), |a, b| format!("{a} and {b}"))
                    )
                }
            }
            GameState::SkatDecision => write!(f, "declarer deciding on picking the Skat"),
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
    /// Returns `true` when `self` represents a respond to a call.
    ///
    /// This also returns `true` in the [`Self::Forehand`] case.
    fn respond(&self) -> bool {
        match self {
            Self::MiddleCallsFore => false,
            Self::ForeRespondsMiddle => true,
            Self::RearCallsFore => false,
            Self::ForeRespondsRear => true,
            Self::RearCallsMiddle => false,
            Self::MiddleRespondsRear => true,
            Self::Forehand => true,
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

    /// Evaluate next state after [`Self::source`] `passed` or not.
    fn next(&self, passed: bool, any_bid: bool) -> BiddingResult {
        if passed {
            match self {
                Self::MiddleCallsFore => BiddingResult::Continue(Self::RearCallsFore),
                Self::ForeRespondsMiddle => BiddingResult::Continue(Self::RearCallsMiddle),
                Self::RearCallsFore if any_bid => BiddingResult::Finished(Player::Forehand),
                Self::RearCallsFore => BiddingResult::Continue(Self::Forehand),
                Self::ForeRespondsRear => BiddingResult::Finished(Player::Rearhand),
                Self::RearCallsMiddle => BiddingResult::Finished(Player::Middlehand),
                Self::MiddleRespondsRear => BiddingResult::Finished(Player::Rearhand),
                Self::Forehand => BiddingResult::Draw,
            }
        } else {
            match self {
                Self::MiddleCallsFore => BiddingResult::Continue(Self::ForeRespondsMiddle),
                Self::ForeRespondsMiddle => BiddingResult::Continue(Self::MiddleCallsFore),
                Self::RearCallsFore => BiddingResult::Continue(Self::ForeRespondsRear),
                Self::ForeRespondsRear => BiddingResult::Continue(Self::RearCallsFore),
                Self::RearCallsMiddle => BiddingResult::Continue(Self::MiddleRespondsRear),
                Self::MiddleRespondsRear => BiddingResult::Continue(Self::RearCallsMiddle),
                Self::Forehand => BiddingResult::Finished(Player::Forehand),
            }
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

enum BiddingResult {
    /// Bidding continues.
    Continue(BiddingState),
    /// Bidding finished with [`Player`] becoming declarer.
    Finished(Player),
    /// All passed.
    Draw,
}

#[derive(Clone, Debug)]
struct Skat {
    cards: CardStruct,
    /// The one player playing against the rest.
    declarer: Player,
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
        Self {
            cards: Default::default(),
            // This will be overridden in the bidding phase anyway.
            declarer: Player::Forehand,
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
            GameState::Bidding { bid: _, state } => state.source().into(),
            GameState::SkatDecision => self.declarer.into(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
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
            GameState::Bidding { bid, state } => {
                // 0 means passing.
                moves.push(0.into());
                if state.respond() {
                    // 1 means accepting.
                    moves.push(1.into());
                } else {
                    moves.extend(
                        (bid.saturating_add(1)..=GameState::MAXIMUM_BID)
                            .map(move_code::from)
                            .map(MoveCode::from),
                    );
                }
            }
            GameState::SkatDecision => moves.extend_from_slice(&[0.into(), 1.into()]),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
        }

        Ok(())
    }

    /// Convert a move string to a [`MoveCode`].
    ///
    /// Examples for dealing cards: `10S` for _10 of spades_ or `?` for a hidden
    /// action.
    fn get_move_data(&mut self, _player: player_id, string: &str) -> Result<Self::Move> {
        let string = string.trim();
        match self.state {
            GameState::Dealing => {
                let card: OptCard = string.parse()?;
                Ok(card.into())
            }
            GameState::Bidding { bid: _, state: _ } => {
                if string.eq_ignore_ascii_case("pass") {
                    Ok(0.into())
                } else if string.eq_ignore_ascii_case("accept")
                    || string.eq_ignore_ascii_case("yes")
                {
                    Ok(1.into())
                } else {
                    string.parse().map(move_code::into).map_err(|e| {
                        Error::new_dynamic(
                            ErrorCode::InvalidInput,
                            format!("failed to parse move as a valid number: {e}"),
                        )
                    })
                }
            }
            GameState::SkatDecision => {
                if string.eq_ignore_ascii_case("hand") {
                    Ok(0.into())
                } else if string.eq_ignore_ascii_case("pick") {
                    Ok(1.into())
                } else {
                    Err(Error::new_static(
                        ErrorCode::InvalidInput,
                        "invalid Skat decision\0",
                    ))
                }
            }
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
        }
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
                write!(str_buf, "{card}")
            }
            GameState::Bidding { bid: _, state: _ } => {
                #[allow(clippy::assertions_on_constants)]
                const _: () = assert!(1 < GameState::MAXIMUM_BID);

                if mov.md == 0 {
                    write!(str_buf, "pass")
                } else if mov.md == 1 {
                    write!(str_buf, "accept")
                } else {
                    write!(str_buf, "{}", mov.md)
                }
            }
            GameState::SkatDecision if mov.md == 0 => write!(str_buf, "Hand"),
            GameState::SkatDecision => write!(str_buf, "pick"),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
        }
        .expect("writing move failed");
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
            GameState::Bidding { bid, state } => {
                let any_bid = *bid >= GameState::MINIMUM_BID;
                let next = match mov.md {
                    0 => state.next(true, any_bid),
                    1 => state.next(false, any_bid),
                    m => {
                        *bid = m.try_into().expect("bid overflowed");
                        state.next(false, any_bid)
                    }
                };
                match next {
                    BiddingResult::Continue(s) => *state = s,
                    BiddingResult::Finished(p) => {
                        self.declarer = p;
                        self.state = GameState::SkatDecision
                    }
                    BiddingResult::Draw => self.state = GameState::Finished(Default::default()),
                }
            }
            GameState::SkatDecision if mov.md == 0 => todo!(),
            GameState::SkatDecision => todo!(),
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
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
        match self.state {
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
            GameState::Bidding { bid, state } => {
                if Player::try_from(player) != Ok(state.source()) {
                    return Err(Error::new_static(
                        ErrorCode::InvalidPlayer,
                        "player is currently not at turn while bidding\0",
                    ));
                }
                if state.respond() {
                    if mov.md > 1 {
                        return Err(Error::new_static(
                            ErrorCode::InvalidMove,
                            "invalid bidding response\0",
                        ));
                    }
                } else if mov.md != 0
                    && (mov.md <= bid.into() || mov.md > GameState::MAXIMUM_BID.into())
                {
                    return Err(Error::new_static(ErrorCode::InvalidMove, "invalid bid\0"));
                }
            }
            GameState::SkatDecision => {
                // Any move code is legal.
            }
            GameState::Declaring => todo!(),
            GameState::Playing => todo!(),
            GameState::Finished(_) => todo!(),
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
            _ => Ok(mov.md.into()),
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
        if self.state.has_declarer() {
            writeln!(f, "{} is declarer", self.declarer)?;
        }
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
