//! This is an implementation of
//! [_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
//! [_surena_](https://github.com/RememberOfLife/surena) game engine and the
//! [_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

use mirabel::game::player_id;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Player {
    Forehand,
    Middlehand,
    Rearhand,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CardLocation {
    NotInGame,
    Player(Player),
    Skat,
    Unknown,
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
        // FIXME: Replace with std::mem::variant_count when stabilize.
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
                cards[suit * CardValue::count() + value] =
                    Self(Suit::all()[suit], CardValue::all()[value]);
                value += 1;
            }
            suit += 1;
        }
        cards
    }
}

#[derive(Clone, Debug)]
struct Skat {
    cards: [CardLocation; Card::count()],
    dealer: player_id,
    state: GameState,
}

#[derive(Clone, Debug)]
enum GameState {
    Dealing,
    Bidding,
    Declaring,
    Playing,
}
