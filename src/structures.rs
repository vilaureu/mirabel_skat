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
}

// FIXME: Fit into a single byte.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Card(Suit, CardValue);

impl Card {
    pub(crate) const COUNT: usize = Suit::COUNT * CardValue::COUNT;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        let mut cards = [Self(Suit::Clubs, CardValue::Num7); Self::COUNT];
        let mut suit = 0;
        while suit < Suit::COUNT {
            let mut value = 0;
            while value < CardValue::COUNT {
                let card = Self(Suit::all()[suit], CardValue::all()[value]);
                cards[card.index()] = card;
                value += 1;
            }
            suit += 1;
        }
        cards
    }

    /// Returns the index of `self` into [`Self::all()`].
    pub(crate) const fn index(&self) -> usize {
        self.0 as usize * CardValue::COUNT + self.1 as usize
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
