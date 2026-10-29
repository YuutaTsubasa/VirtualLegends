//! 牌氣：萬、筒、條各 1～9 四張，字牌（東南西北中發白）1～7 各兩張，所以字牌比較稀有。
//! 三張可以組成順子（同花色連號）或刻子（三張一樣）；字牌可以當萬用牌，但組出來的牌「降一級」。

use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Suit {
    Man,
    Pin,
    Sou,
    Honor,
}

impl Suit {
    /// 萬剋條、條剋筒、筒剋萬；字不剋也不被剋
    pub fn beats(self, other: Suit) -> bool {
        matches!(
            (self, other),
            (Suit::Man, Suit::Sou) | (Suit::Sou, Suit::Pin) | (Suit::Pin, Suit::Man)
        )
    }

    /// 攻擊方對防守方的傷害倍率（百分比）
    pub fn affinity(attacker: Suit, defender: Suit) -> i64 {
        if attacker.beats(defender) {
            130
        } else if defender.beats(attacker) {
            80
        } else {
            100
        }
    }

    /// 這個花色有幾種點數
    pub fn ranks(self) -> u8 {
        if self == Suit::Honor {
            7
        } else {
            9
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Tile {
    pub suit: Suit,
    pub rank: u8,
}

impl Tile {
    pub fn new(suit: Suit, rank: u8) -> Self {
        Tile { suit, rank }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeldKind {
    Sequence,
    Triplet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meld {
    pub kind: MeldKind,
    /// 用了字牌當萬用牌
    pub wild: bool,
}

impl Meld {
    /// 大招的威力倍率（百分比）：順子 100、刻子 130，用了萬用牌再乘 0.75
    pub fn power(self) -> i64 {
        let base = match self.kind {
            MeldKind::Sequence => 100,
            MeldKind::Triplet => 130,
        };
        if self.wild {
            base * 75 / 100
        } else {
            base
        }
    }
}

/// 三張牌能組成什麼。能組刻子時優先算刻子（比較強）。
pub fn meld_of(tiles: [Tile; 3]) -> Option<Meld> {
    let honors: Vec<Tile> = tiles
        .iter()
        .copied()
        .filter(|t| t.suit == Suit::Honor)
        .collect();
    let mut naturals: Vec<Tile> = tiles
        .iter()
        .copied()
        .filter(|t| t.suit != Suit::Honor)
        .collect();
    naturals.sort();
    if naturals.iter().any(|t| t.suit != naturals[0].suit) {
        return None;
    }
    match naturals.len() {
        // 三張字牌：一樣的是真刻子，不一樣的全當萬用牌
        0 => {
            let same = honors.iter().all(|t| *t == honors[0]);
            Some(Meld {
                kind: MeldKind::Triplet,
                wild: !same,
            })
        }
        1 => Some(Meld {
            kind: MeldKind::Triplet,
            wild: true,
        }),
        2 => {
            let (a, b) = (naturals[0].rank, naturals[1].rank);
            if a == b {
                Some(Meld {
                    kind: MeldKind::Triplet,
                    wild: true,
                })
            } else if b - a <= 2 {
                Some(Meld {
                    kind: MeldKind::Sequence,
                    wild: true,
                })
            } else {
                None
            }
        }
        _ => {
            let r: Vec<u8> = naturals.iter().map(|t| t.rank).collect();
            if r[0] == r[1] && r[1] == r[2] {
                Some(Meld {
                    kind: MeldKind::Triplet,
                    wild: false,
                })
            } else if r[0] + 1 == r[1] && r[1] + 1 == r[2] {
                Some(Meld {
                    kind: MeldKind::Sequence,
                    wild: false,
                })
            } else {
                None
            }
        }
    }
}

/// 合技：這回合花掉的牌裡，能不能湊出不重疊的一組順子＋一組刻子
pub fn has_combo(spent: &[Tile]) -> bool {
    let n = spent.len();
    if n < 6 {
        return false;
    }
    let triples: Vec<[usize; 3]> = (0..n)
        .flat_map(|i| (i + 1..n).flat_map(move |j| (j + 1..n).map(move |k| [i, j, k])))
        .collect();
    let kind = |t: &[usize; 3]| meld_of([spent[t[0]], spent[t[1]], spent[t[2]]]).map(|m| m.kind);
    triples
        .iter()
        .filter(|t| kind(t) == Some(MeldKind::Triplet))
        .any(|tri| {
            triples
                .iter()
                .filter(|s| s.iter().all(|i| !tri.contains(i)))
                .any(|s| kind(s) == Some(MeldKind::Sequence))
        })
}

/// 牌山：摸完就重新洗一副
#[derive(Clone, Debug)]
pub struct Wall {
    tiles: Vec<Tile>,
}

impl Wall {
    pub fn full_set() -> Vec<Tile> {
        let mut v = Vec::new();
        for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
            for rank in 1..=9 {
                v.extend(std::iter::repeat_n(Tile::new(suit, rank), 4));
            }
        }
        for rank in 1..=7 {
            v.extend(std::iter::repeat_n(Tile::new(Suit::Honor, rank), 2));
        }
        v
    }

    pub fn new(rng: &mut Rng) -> Self {
        let mut w = Wall { tiles: Vec::new() };
        w.refill(rng);
        w
    }

    fn refill(&mut self, rng: &mut Rng) {
        self.tiles = Self::full_set();
        rng.shuffle(&mut self.tiles);
    }

    pub fn draw(&mut self, rng: &mut Rng) -> Tile {
        if self.tiles.is_empty() {
            self.refill(rng);
        }
        self.tiles.pop().expect("牌山剛洗好不會是空的")
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}
