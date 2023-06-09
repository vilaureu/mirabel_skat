//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

mod structures;

use core::panic;
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

use structures::{Card, CardStruct, Declaration, DeclarationMove, Matadors, Player};

use crate::structures::OptCard;

#[derive(Clone, Debug, Default)]
enum GameState {
    /// State while dealing cards.
    #[default]
    Dealing,
    /// State of the bidding phase.
    Bidding {
        state: BiddingState,
    },
    /// Single player is deciding whether to look at the Skat or not.
    SkatDecision,
    /// Single player is picking up the Skat.
    ///
    /// This is performed by [`PLAYER_RAND`].
    Picking,
    /// Single player is putting back cards.
    Putting,
    Declaring,
    /// Single player is revealing their cards.
    ///
    /// Stores the index of the next card to reveal.
    Revealing(usize),
    /// The actual trick-taking game is going on.
    ///
    /// Stores the player whose turn it is.
    Playing(PlayingState),
    // FIXME: Replace with fixed-size array.
    Finished(Vec<Player>),
}

impl GameState {
    /// Does the game have a declarer at this stage.
    fn has_declarer(&self) -> bool {
        !matches!(
            self,
            GameState::Dealing | GameState::Bidding { state: _ } | GameState::Finished(_)
        )
    }

    fn has_declaration(&self) -> bool {
        self.has_declarer()
            && !matches!(
                self,
                GameState::SkatDecision
                    | GameState::Picking
                    | GameState::Putting
                    | GameState::Declaring
            )
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameState::Dealing => write!(f, "dealing"),
            GameState::Bidding { state } => {
                write!(f, "bidding: {state}")
            }
            GameState::SkatDecision => write!(f, "declarer deciding on picking the Skat"),
            GameState::Picking => write!(f, "declarer picking up the Skat"),
            GameState::Putting => write!(f, "declarer putting back cards"),
            GameState::Declaring => write!(f, "declarer is declaring"),
            GameState::Revealing(i) => write!(f, "declarer is revealing card {i} next"),
            GameState::Playing(state) => state.fmt(f),
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

#[derive(Debug, Clone)]
struct PlayingState {
    player: Player,
    declarer_points: Option<u8>,
    team_points: Option<u8>,
}

impl Display for PlayingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "it is {}'s turn", self.player)?;
        for (name, points) in [
            ("declarer", self.declarer_points),
            ("team", self.team_points),
        ] {
            writeln!(f)?;
            if let Some(points) = points {
                write!(f, "{name} has {points} points")?;
            } else {
                write!(f, "{name} has no tricks")?;
            }
        }
        Ok(())
    }
}

impl Default for PlayingState {
    fn default() -> Self {
        Self {
            player: Player::Forehand,
            declarer_points: Default::default(),
            team_points: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
struct Skat {
    cards: CardStruct,
    // FIXME: This could fit into 8 bytes when a offset is used.
    bid: u16,
    /// The one player playing against the rest.
    declarer: Player,
    declaration: Declaration,
    // mode: GameMode,
    state: GameState,
}

impl Skat {
    const MINIMUM_BID: u16 = 18;
    const MAXIMUM_BID: u16 = 264;
    /// Declarer is winner with at least this many points.
    const POINTS_WINNING: u8 = 61;
    /// Party is _Schneider_ when having these many or less points.
    const POINTS_SCHNEIDER: u8 = 30;

    /// Calculate the (missing) matadors for the declarer.
    ///
    /// The Skat is only considered if the declaration is not a _Hand_ game.
    /// Returns [`Node`] if any used cards are [`OptCard::Hidden`].
    fn calculate_matadors(&self) -> Option<Matadors> {
        // FIXME: Avoid allocation.
        let mut cards = (*self.cards[self.declarer]).clone();
        if !self.declaration.is_hand() {
            cards.extend_from_slice(&self.cards.skat);
        }
        if cards.iter().any(|c| matches!(c, OptCard::Hidden)) {
            return None;
        }
        Some(Matadors::from_cards(cards.into_iter().map(|c| match c {
            OptCard::Hidden => unreachable!(),
            OptCard::Known(c) => c,
        })))
    }

    /// Return the declaration if [`GameState::has_declaration()`] is `true`.
    fn declaration(&self) -> Option<Declaration> {
        if self.state.has_declaration() {
            Some(self.declaration)
        } else {
            None
        }
    }

    /// Calculates the points for the declarer's score when the game is over.
    ///
    /// # Panics
    /// Panics if not in [`GameState::Playing`].
    fn calculate_points(&self) -> i16 {
        let GameState::Playing(ref state) = self.state else {panic!("can only determine winner is state playing")};

        let Declaration::Normal(mode, _) = self.declaration else {
            // No need to check overbidding as it is impossible for Null games.
            let value: i16 = u16::from(self.declaration).try_into().unwrap();
            if state.declarer_points.is_some() {
                return -2 * value;
            } else {
                return value;
            }
        };

        let won = state.declarer_points.unwrap_or_default() >= Self::POINTS_WINNING;
        let looser_points = if won {
            state.team_points
        } else {
            state.declarer_points
        };
        let schneider = looser_points.unwrap_or_default() <= Self::POINTS_SCHNEIDER;
        let schneider_announced = self.declaration.is_schneider();
        let schwarz = looser_points.is_none();
        let schwarz_announced = self.declaration.is_schwarz();

        let matadors = Matadors::from_cards(
            self.cards.played[self.declarer as usize]
                .iter()
                .cloned()
                .chain(self.cards.skat.iter_known()),
        )[mode];

        let multiplier: i16 = 1i16
            + i16::from(self.declaration.is_hand())
            + i16::from(schneider || schneider_announced)
            + i16::from(schneider_announced)
            + i16::from(schwarz || schwarz_announced)
            + i16::from(schwarz_announced)
            + i16::from(self.declaration.is_ouvert())
            + i16::from(matadors);
        let value = i16::try_from(u16::from(self.declaration)).unwrap() * multiplier;
        let bid = self.bid.try_into().unwrap();
        if won
            && (!schneider_announced || schneider)
            && (!schwarz_announced || schwarz)
            && value >= bid
        {
            value
        } else {
            -2 * value.max(bid)
        }
    }
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
            bid: Self::MINIMUM_BID - 1,
            // This will be overridden in the bidding phase anyway.
            declarer: Player::Forehand,
            declaration: Default::default(),
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
            GameState::Dealing | GameState::Picking | GameState::Revealing(_) => PLAYER_RAND,
            GameState::Bidding { state } => state.source().into(),
            GameState::SkatDecision | GameState::Putting | GameState::Declaring => {
                self.declarer.into()
            }
            GameState::Playing(ref state) => state.player.into(),
            GameState::Finished(_) => return Ok(()),
        });
        Ok(())
    }

    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()> {
        match self.state {
            GameState::Dealing => moves.extend(
                self.cards
                    .iter_unknown()
                    .map(|card| MoveCode::from(OptCard::from(card))),
            ),
            GameState::Bidding { state } => {
                // 0 means passing.
                moves.push(0.into());
                if state.respond() {
                    // 1 means accepting.
                    moves.push(1.into());
                } else {
                    moves.extend(
                        (self.bid.saturating_add(1)..=Self::MAXIMUM_BID)
                            .map(move_code::from)
                            .map(MoveCode::from),
                    );
                }
            }
            GameState::SkatDecision => moves.extend_from_slice(&[0.into(), 1.into()]),
            GameState::Picking => match self.cards.skat.last() {
                Some(OptCard::Known(card)) => moves.push(OptCard::from(*card).into()),
                Some(OptCard::Hidden) => moves.extend(
                    self.cards
                        .iter_unknown()
                        .map(|card| MoveCode::from(OptCard::from(card))),
                ),
                None => {
                    return Err(Error::new_static(
                        ErrorCode::InvalidState,
                        "no card in the Skat to pick up\0",
                    ))
                }
            },
            GameState::Putting => {
                let hand = &self.cards[self.declarer];
                moves.extend(
                    hand.iter_known()
                        .map(|card| MoveCode::from(OptCard::from(card))),
                );
                if hand.iter().any(|card| matches!(card, OptCard::Hidden)) {
                    moves.extend(
                        self.cards
                            .iter_unknown()
                            .map(|card| MoveCode::from(OptCard::from(card))),
                    )
                }
            }
            GameState::Declaring => {
                let matadors = self.calculate_matadors();
                moves.extend(
                    Declaration::all(self.declaration.is_hand())
                        .into_iter()
                        .filter(|d| {
                            matadors
                                .as_ref()
                                .filter(|m| d.allowed(self.bid, m))
                                .is_some()
                        })
                        .map(|d| MoveCode::from(DeclarationMove::Declare(d))),
                );
                if moves.is_empty() {
                    moves.push(DeclarationMove::Overbidden.into());
                }
            }
            GameState::Revealing(i) => {
                let card = self.cards[self.declarer]
                    .get(i)
                    .ok_or_else(|| reveal_error(i))?;
                match *card {
                    OptCard::Known(c) => moves.push(c.into()),
                    OptCard::Hidden => {
                        moves.extend(self.cards.iter_unknown().map(Into::<MoveCode>::into))
                    }
                }
            }
            GameState::Playing(ref state) => moves.extend(
                self.cards
                    .allowed(state.player, self.declaration)
                    .into_iter()
                    .map(Into::<MoveCode>::into),
            ),
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
            GameState::Dealing | GameState::Picking | GameState::Putting => {
                let card: OptCard = string.parse()?;
                Ok(card.into())
            }
            GameState::Bidding { state: _ } => {
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
            GameState::Declaring => {
                let declaration: DeclarationMove = string.parse()?;
                Ok(declaration.into())
            }
            GameState::Revealing(_) | GameState::Playing(_) => {
                let card: Card = string.parse()?;
                Ok(card.into())
            }
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
            GameState::Dealing | GameState::Picking | GameState::Putting => {
                let card: OptCard = mov.md.try_into()?;
                write!(str_buf, "{card}")
            }
            GameState::Bidding { state: _ } => {
                #[allow(clippy::assertions_on_constants)]
                const _: () = assert!(1 < Skat::MAXIMUM_BID);

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
            GameState::Declaring => {
                let declaration: DeclarationMove = mov.md.try_into()?;
                write!(str_buf, "{declaration}")
            }
            GameState::Revealing(_) | GameState::Playing(_) => {
                let card: Card = mov.md.try_into()?;
                write!(str_buf, "{card}")
            }
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
                        state: Default::default(),
                    };
                }
            }
            GameState::Bidding { state } => {
                let any_bid = self.bid >= Self::MINIMUM_BID;
                let next = match mov.md {
                    0 => state.next(true, any_bid),
                    1 => state.next(false, any_bid),
                    m => {
                        self.bid = m.try_into().expect("bid overflowed");
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
            GameState::SkatDecision if mov.md == 0 => {
                // Change the game to a _Hand_ game to encode that the declarer
                // is playing _Hand_.
                self.declaration = Declaration::NullHand;
                self.state = GameState::Declaring;
            }
            GameState::SkatDecision => self.state = GameState::Picking,
            GameState::Picking => {
                assert_eq!(PLAYER_RAND, player);
                let card = mov.md.try_into()?;
                self.cards.skat.pop();
                self.cards.give(Some(self.declarer), card);
                if self.cards.skat.is_empty() {
                    self.state = GameState::Putting;
                }
            }
            GameState::Putting => {
                let card = mov.md.try_into()?;
                self.cards.take(self.declarer, card)?;
                self.cards.give(None, card);
                if self.cards.skat.len() >= CardStruct::SKAT_SIZE {
                    self.state = GameState::Declaring;
                }
            }
            GameState::Declaring => {
                let declaration: DeclarationMove = mov.md.try_into()?;
                match declaration {
                    DeclarationMove::Declare(declaration) => {
                        self.declaration = declaration;
                        self.state = if declaration.is_ouvert() {
                            // This assumes that the declarer has at least one
                            // card.
                            GameState::Revealing(0)
                        } else {
                            GameState::Playing(Default::default())
                        };
                    }
                    DeclarationMove::Overbidden => {
                        self.state = GameState::Finished(self.declarer.others().to_vec())
                    }
                }
            }
            GameState::Revealing(i) => {
                let card: Card = mov.md.try_into()?;
                let hand = &mut self.cards[self.declarer];
                *hand.get_mut(*i).ok_or_else(|| reveal_error(*i))? = OptCard::Known(card);
                *i += 1;
                if *i >= hand.len() {
                    self.state = GameState::Playing(Default::default())
                }
            }
            GameState::Playing(state) => 'p: {
                let card: Card = mov.md.try_into()?;
                self.cards.take(state.player, OptCard::Known(card))?;
                let trick = &mut self.cards.trick;
                trick.push(card);
                state.player = state.player.next();
                if trick.len() < Player::COUNT {
                    break 'p;
                }

                let w = self.cards.winner(self.declaration);
                let mut winner = state.player;
                for _ in 0..w {
                    winner = winner.next();
                }
                let points: u8 = self.cards.trick.iter().cloned().sum();
                if winner == self.declarer {
                    *state.declarer_points.get_or_insert(0) += points;
                } else {
                    *state.team_points.get_or_insert(0) += points;
                }
                self.cards.put_trick(state.player);
                state.player = winner;

                // TODO: Calculate overall winner.
                if (self.declaration.is_null() && state.declarer_points.is_some())
                    || (self.declaration.is_schwarz() && state.team_points.is_some())
                    || self.cards.hands.iter().all(|h| h.is_empty())
                {
                    // TODO: Send Skat to players.
                    let points = self.calculate_points();
                }
            }
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
            GameState::Bidding { state } => {
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
                    && (mov.md <= self.bid.into() || mov.md > Self::MAXIMUM_BID.into())
                {
                    return Err(Error::new_static(ErrorCode::InvalidMove, "invalid bid\0"));
                }
            }
            GameState::SkatDecision => {
                // Any move code is legal.
            }
            GameState::Picking => {
                if player != PLAYER_RAND {
                    return Err(Error::new_static(
                        ErrorCode::InvalidPlayer,
                        "PLAYER_RAND must pick up Skat cards\0",
                    ));
                }
                let Some(skat_card) = self.cards.skat.last() else {
                    return Err(Error::new_static(
                        ErrorCode::InvalidState,
                        "no card in the Skat to pick up\0",
                    ));
                };
                if let OptCard::Known(card) = mov.md.try_into()? {
                    match skat_card {
                        OptCard::Known(skat_card) => {
                            if card != *skat_card {
                                return Err(Error::new_static(
                                    ErrorCode::InvalidMove,
                                    "not the correct card to pick up\0",
                                ));
                            }
                        }
                        OptCard::Hidden => {
                            if self.cards.iter().any(|c| c == card) {
                                return Err(Error::new_static(
                                    ErrorCode::InvalidMove,
                                    "this card is already at another place\0",
                                ));
                            }
                        }
                    }
                }
            }
            GameState::Putting => {
                let hand = &self.cards[self.declarer];
                if hand.is_empty() {
                    return Err(Error::new_static(
                        ErrorCode::InvalidState,
                        "declarer's hand is empty\0",
                    ));
                }

                if let OptCard::Known(card) = mov.md.try_into()? {
                    if !hand.iter_known().any(|c| c == card) {
                        if hand.iter().any(|c| matches!(c, OptCard::Hidden)) {
                            if self.cards.iter().any(|c| c == card) {
                                return Err(Error::new_static(
                                    ErrorCode::InvalidMove,
                                    "this card is already at another place\0",
                                ));
                            }
                        } else {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                "this card is not in the declarer's hand\0",
                            ));
                        }
                    }
                }
            }
            GameState::Declaring => 'b: {
                let declaration: DeclarationMove = mov.md.try_into()?;
                let Some(matadors) = self.calculate_matadors() else {break 'b;};

                match declaration {
                    DeclarationMove::Declare(declaration) => {
                        if declaration.is_hand() != self.declaration.is_hand() {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                if declaration.is_hand() {
                                    "cannot declare Hand after picking up Skat\0"
                                } else {
                                    "you must declare a Hand game\0"
                                },
                            ));
                        }
                        if !declaration.allowed(self.bid, &matadors) {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                "declaration would lead to overbidding\0",
                            ));
                        }
                    }
                    DeclarationMove::Overbidden => {
                        if Declaration::all(self.declaration.is_hand())
                            .iter()
                            .any(|d| d.allowed(self.bid, &matadors))
                        {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                "not actually overbidden\0",
                            ));
                        }
                    }
                }
            }
            GameState::Revealing(i) => {
                let card: Card = mov.md.try_into()?;
                let target = self.cards[self.declarer]
                    .get(i)
                    .ok_or_else(|| reveal_error(i))?;
                match target {
                    OptCard::Hidden => {
                        if self.cards.iter().any(|c| c == card) {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                "this card is already at another place\0",
                            ));
                        }
                    }
                    OptCard::Known(t) => {
                        if &card != t {
                            return Err(Error::new_static(
                                ErrorCode::InvalidMove,
                                "not the correct card for this index\0",
                            ));
                        }
                    }
                }
            }
            GameState::Playing(ref state) => {
                let card: Card = mov.md.try_into()?;
                if !self
                    .cards
                    .allowed(state.player, self.declaration)
                    .contains(&card)
                {
                    return Err(Error::new_static(
                        ErrorCode::InvalidMove,
                        "not allowed to play this card\0",
                    ));
                }
            }
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

        let target_player = Player::from(target_player);
        match self.state {
            GameState::Dealing => {
                assert_eq!(PLAYER_RAND, player);
                let target = deal_to(self.cards.count());
                if target.filter(|&t| t == target_player).is_some() {
                    Ok(mov.md.into())
                } else {
                    Ok(OptCard::Hidden.into())
                }
            }
            GameState::Picking => {
                assert_eq!(PLAYER_RAND, player);
                if self.declarer == target_player {
                    Ok(mov.md.into())
                } else {
                    Ok(OptCard::Hidden.into())
                }
            }
            GameState::Putting => Ok(OptCard::Hidden.into()),
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
        let mut cards = self.cards.clone();
        cards.sort(self.declaration().filter(|d| d.is_null()).is_some());
        writeln!(f, "{}", cards)?;
        if self.bid >= Self::MINIMUM_BID {
            writeln!(f, "highest bid: {}", self.bid)?;
        }
        if self.state.has_declarer() {
            writeln!(f, "{} is declarer", self.declarer)?;
        }
        if self.state.has_declaration() {
            writeln!(f, "playing {}", self.declaration)?;
        } else if self.declaration.is_hand() {
            writeln!(f, "going to be a Hand game")?;
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

/// Returns an error that the card i cannot be revealed as it does not exist.
fn reveal_error(i: usize) -> Error {
    Error::new_dynamic(
        ErrorCode::InvalidState,
        format!("cannot reveal card {i} as it does not exist"),
    )
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
