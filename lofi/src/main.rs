// Procedural lo-fi hip hop, rendered offline to a WAV.
// A port of the Web Audio lofi plugin from https://git.borodutch.com/backmeupplz/vancouver
// (public/plugins/lofi/lofi.js): same vibes, same track generator (a seed gives the same track name and
// structure), same synth graph, with the Web Audio nodes it uses written out by hand.
//
//   lofi <vibe> <seed> <out.wav>   prints {"name","vibe","label","bpm","duration"}

use std::f64::consts::PI;

const SR: f64 = 44100.0;
const START: f64 = 0.05; // the engine starts each track here, like the web version

// --- vibes ------------------------------------------------------------------

const PLACES: &[&str] = &["Kitsilano", "Gastown", "Yaletown", "Strathcona", "Kerrisdale", "Jericho", "Lonsdale", "Dunbar",
  "Fairview", "Burrard", "Cambie", "Davie", "Robson", "Hastings", "Granville", "Chinatown", "Marpole", "Shaughnessy",
  "Steveston", "Seawall", "Kits", "Lynn", "Grouse", "Cypress", "Musqueam", "Riley", "Hornby", "Main", "Commercial", "Deas"];
const MOODS: &[&str] = &["Rain", "Drizzle", "Fog", "Ferry", "Lanterns", "Puddles", "Seabus", "Tea", "Windows", "Sunday", "Dusk",
  "Streetcar", "Herons", "Ravens", "Umbrellas", "Cedars", "Steam", "Moss", "Neon", "Skytrain", "Morning", "Afterglow",
  "Driftwood", "Pages", "Cocoa", "Tides", "Gulls", "Mist", "Blossoms", "Bookshop", "Sleepers", "Harbour"];

// Chord qualities as semitones above the chord root; 5+ note chords are voiced rootless (the bass has the root).
fn quality(q: &str) -> &'static [i32] {
  match q {
    "M9" => &[0, 4, 7, 11, 14],
    "m9" => &[0, 3, 7, 10, 14],
    "d9" => &[0, 4, 7, 10, 14],
    "m7" => &[0, 3, 7, 10],
    "M7" => &[0, 4, 7, 11],
    "m11" => &[0, 3, 7, 10, 14, 17],
    "d13" => &[0, 4, 10, 14, 21],
    "69" => &[0, 4, 7, 9, 14],
    _ => unreachable!("unknown chord quality {q}"),
  }
}

type Chord = (i32, &'static str); // degree in semitones above the key, quality
type Prog = &'static [Chord];
type Steps = &'static [i32];
type Comp = &'static [(i32, i32)]; // [step, length]

#[derive(Clone, Copy, PartialEq)]
enum Kit { Tape, Brush }
#[derive(Clone, Copy, PartialEq)]
enum Keys { Ep, Pad }
#[derive(Clone, Copy, PartialEq)]
enum Bass { Walk, Notes(&'static [(i32, i32, i32)]) } // [step, semitones above root, length]

#[derive(Clone, Copy)]
struct Mix { tone: [f64; 2], keys_lp: f64, trem: f64, drums: f64, drums_lp: f64, verb: f64, crackle: f64, hiss: f64, rain: f64, bass_lp: f64 }

// hats: hi-hat step division per section [A, B] (1 = 16ths, 2 = 8ths, 4 = quarters); mix.tone: master low-pass [A, B].
#[derive(Clone, Copy)]
struct Vibe {
  label: &'static str, moods: &'static [&'static str], bpm: [i32; 2], key: i32, scale: [i32; 5], progs: &'static [Prog],
  chord_bars: &'static [f64], comps: &'static [Comp], basses: &'static [Bass], bass_lo: i32, kicks: &'static [Steps],
  ghosts: &'static [Steps], swing: [f64; 2], lick_p: [f64; 2], hats: [i32; 2], drums: f64, kit: Kit, keys: Keys,
  lift: i32, bark: f64, mix: Mix,
}

const PENT: [i32; 5] = [0, 2, 4, 7, 9];
const MINPENT: [i32; 5] = [0, 3, 5, 7, 10];
const FORMS: &[&[(&str, usize)]] = &[
  &[("intro", 4), ("A", 16), ("B", 16), ("A", 16), ("outro", 4)],
  &[("intro", 4), ("A", 8), ("B", 16), ("A", 16), ("B", 8), ("outro", 4)],
  &[("intro", 2), ("A", 16), ("B", 8), ("A", 8), ("B", 16), ("outro", 4)],
];
const CHILL_MIX: Mix = Mix { tone: [6500.0, 8500.0], keys_lp: 2400.0, trem: 0.18, drums: 0.42, drums_lp: 9000.0, verb: 0.7, crackle: 0.22, hiss: 0.02, rain: 0.0, bass_lp: 420.0 };
const CHILL: Vibe = Vibe {
  label: "Chill", moods: MOODS, bpm: [70, 90], key: 48, scale: PENT,
  progs: &[
    &[(2, "m9"), (7, "d9"), (0, "M9"), (9, "m9")],
    &[(0, "M9"), (9, "m9"), (2, "m9"), (7, "d13")],
    &[(5, "M9"), (4, "m7"), (2, "m9"), (0, "M9")],
    &[(9, "m9"), (2, "m9"), (7, "d9"), (0, "M9")],
    &[(5, "M9"), (4, "m9"), (9, "m9"), (7, "d13")],
    &[(4, "m7"), (9, "m9"), (2, "m11"), (7, "d13")],
    &[(2, "m11"), (2, "m11"), (0, "M9"), (0, "M7")],
    &[(0, "M9"), (5, "M9"), (4, "m7"), (9, "m9")],
  ],
  chord_bars: &[1.0, 2.0],
  comps: &[&[(0, 16)], &[(0, 10), (10, 6)], &[(0, 6), (6, 10)], &[(0, 3), (3, 13)], &[(0, 16)]],
  basses: &[
    Bass::Notes(&[(0, 0, 7), (10, 0, 4), (14, 7, 2)]),
    Bass::Notes(&[(0, 0, 6), (6, 0, 2), (10, 7, 4), (14, 12, 2)]),
    Bass::Notes(&[(0, 0, 14)]),
    Bass::Notes(&[(0, 0, 4), (7, 0, 3), (10, 0, 6)]),
    Bass::Notes(&[(0, 0, 10), (10, 7, 6)]),
  ],
  bass_lo: 36,
  kicks: &[&[0, 10], &[0, 7, 10], &[0, 3, 10], &[0, 10, 13], &[0, 6, 10], &[0, 8, 10], &[0, 3, 8, 10]],
  ghosts: &[&[7, 15], &[15], &[3, 11], &[], &[14]],
  swing: [0.18, 0.36], lick_p: [0.35, 0.8], hats: [2, 1], drums: 1.0, kit: Kit::Tape, keys: Keys::Ep, lift: 0, bark: 1.0,
  mix: CHILL_MIX,
};
const RAINY: Vibe = Vibe {
  label: "Rainy day", bpm: [64, 76], key: 45, scale: MINPENT, chord_bars: &[2.0],
  moods: &["Rain", "Drizzle", "Puddles", "Umbrellas", "Windows", "Mist", "Downpour", "Gutters", "Raincoats", "Drops", "Grey Skies", "Wet Cedars"],
  progs: &[ // minor key: i iv bVII bIII and friends, m9/m11 heavy
    &[(0, "m9"), (5, "m9"), (10, "d9"), (3, "M9")],
    &[(0, "m11"), (8, "M9"), (3, "M9"), (10, "d13")],
    &[(0, "m9"), (0, "m11"), (5, "m9"), (7, "m7")],
    &[(5, "m9"), (10, "d9"), (0, "m11"), (0, "m9")],
    &[(8, "M9"), (7, "m7"), (5, "m11"), (0, "m11")],
  ],
  comps: &[&[(0, 16)], &[(0, 10), (10, 6)], &[(0, 6), (6, 10)]], lick_p: [0.3, 0.6], hats: [2, 2], bark: 0.6,
  mix: Mix { tone: [3800.0, 4600.0], keys_lp: 1500.0, trem: 0.16, drums: 0.34, drums_lp: 5000.0, verb: 0.9, crackle: 0.34, hiss: 0.03, rain: 0.7, bass_lp: 380.0 },
  ..CHILL
};
const SUNNY: Vibe = Vibe {
  label: "Sunny afternoon", bpm: [84, 96], key: 50, chord_bars: &[1.0],
  moods: &["Sunshine", "Blossoms", "Seawall", "Gulls", "Picnic", "Lemonade", "Kites", "Beach", "Afternoon", "Patio", "Bikes", "Sails"],
  progs: &[
    &[(0, "M9"), (5, "M9"), (0, "69"), (7, "d9")],
    &[(0, "69"), (9, "m9"), (5, "M9"), (7, "d13")],
    &[(5, "M9"), (7, "d9"), (0, "M9"), (0, "69")],
    &[(0, "M9"), (2, "m9"), (5, "69"), (7, "d9")],
    &[(5, "M9"), (0, "69"), (5, "M9"), (7, "d13")],
  ],
  comps: &[&[(0, 10), (10, 6)], &[(0, 6), (6, 10)], &[(0, 3), (3, 13)], &[(0, 3), (6, 4), (10, 6)]],
  kicks: &[&[0, 7, 10], &[0, 3, 10], &[0, 6, 10], &[0, 3, 8, 10]], ghosts: &[&[7, 15], &[3, 11], &[14]],
  swing: [0.12, 0.26], lick_p: [0.5, 0.9], hats: [1, 1], lift: 5, bark: 1.8,
  mix: Mix { tone: [9000.0, 11000.0], keys_lp: 3800.0, trem: 0.12, drums: 0.48, drums_lp: 12000.0, verb: 0.55, crackle: 0.16, hiss: 0.018, bass_lp: 480.0, ..CHILL_MIX },
  ..CHILL
};
const LATE: Vibe = Vibe {
  label: "Late night", bpm: [62, 72], key: 46, chord_bars: &[2.0],
  moods: &["Neon", "Lanterns", "Sleepers", "Midnight", "Skytrain", "Streetlights", "Night Bus", "Moonlight", "Insomnia", "2 AM", "Taxis", "Afterhours"],
  progs: &[
    &[(0, "m11"), (0, "m11"), (8, "M9"), (8, "M9")],
    &[(9, "m9"), (5, "M9"), (9, "m9"), (4, "m7")],
    &[(2, "m11"), (2, "m11"), (0, "M9"), (0, "M7")],
    &[(0, "m9"), (10, "d9"), (8, "M9"), (7, "m7")],
  ],
  comps: &[&[(0, 16)], &[(0, 10)]], basses: &[Bass::Notes(&[(0, 0, 14)]), Bass::Notes(&[(0, 0, 10), (10, 7, 6)])], bass_lo: 32,
  kicks: &[&[0, 10], &[0, 8]], ghosts: &[&[]], lick_p: [0.15, 0.35], hats: [4, 2], lift: -2, bark: 0.5,
  mix: Mix { tone: [4200.0, 5000.0], keys_lp: 1700.0, trem: 0.2, drums: 0.2, drums_lp: 2400.0, verb: 1.4, crackle: 0.2, bass_lp: 300.0, ..CHILL_MIX },
  ..CHILL
};
const JAZZ: Vibe = Vibe {
  label: "Jazz café", bpm: [78, 92], key: 48, chord_bars: &[0.5, 1.0],
  moods: &["Bookshop", "Espresso", "Cocoa", "Streetcar", "Vinyl", "Bistro", "Brushes", "Café", "Records", "Corner Booth", "Latte", "Saxophone"],
  progs: &[ // ii-V turnarounds, a tritone sub, a long cycle
    &[(2, "m9"), (7, "d13"), (0, "M9"), (9, "d9")],
    &[(0, "M9"), (9, "d9"), (2, "m9"), (7, "d13")],
    &[(4, "m7"), (9, "d9"), (2, "m9"), (7, "d13")],
    &[(2, "m9"), (1, "d9"), (0, "M9"), (0, "69")],
    &[(5, "M9"), (10, "d9"), (4, "m7"), (9, "d9"), (2, "m9"), (7, "d13"), (0, "M9"), (0, "69")],
  ],
  comps: &[&[(0, 6), (8, 6)], &[(0, 3), (6, 2), (8, 3), (14, 2)], &[(2, 4), (10, 4)]], basses: &[Bass::Walk],
  kicks: &[&[0, 10], &[0, 8]], ghosts: &[&[7, 15], &[3, 11]], swing: [0.3, 0.42], lick_p: [0.45, 0.8], hats: [2, 2], kit: Kit::Brush,
  mix: Mix { tone: [7000.0, 8000.0], keys_lp: 3000.0, trem: 0.06, drums: 0.42, drums_lp: 10000.0, verb: 0.8, crackle: 0.24, bass_lp: 560.0, ..CHILL_MIX },
  ..CHILL
};
const STUDY: Vibe = Vibe {
  label: "Study", bpm: [60, 72], key: 48, chord_bars: &[2.0],
  moods: &["Pages", "Tea", "Moss", "Cedars", "Tides", "Library", "Notes", "Focus", "Driftwood", "Margins", "Fog", "Stillness"],
  progs: &[&[(2, "m11"), (0, "M9")], &[(0, "M9"), (5, "M9")], &[(9, "m9"), (5, "M9")], &[(4, "m7"), (9, "m9"), (5, "M9"), (0, "M9")]],
  basses: &[Bass::Notes(&[(0, 0, 14)])], bass_lo: 34, lick_p: [0.1, 0.2], drums: 0.0, keys: Keys::Pad,
  mix: Mix { tone: [4800.0, 5600.0], keys_lp: 1600.0, trem: 0.1, drums: 0.0, verb: 1.3, crackle: 0.14, hiss: 0.015, bass_lp: 300.0, ..CHILL_MIX },
  ..CHILL
};
const SNOW: Vibe = Vibe { // late-night calm, but major and warmer
  label: "Snowfall", bpm: [64, 74], chord_bars: &[2.0],
  moods: &["Snow", "Flurries", "Frost", "Cocoa", "Firelight", "Mittens", "Snowfall", "Quiet Streets", "Wool", "Icicles"],
  comps: &[&[(0, 16)], &[(0, 10), (10, 6)]], basses: &[Bass::Notes(&[(0, 0, 14)]), Bass::Notes(&[(0, 0, 10), (10, 7, 6)])], bass_lo: 33,
  kicks: &[&[0, 10], &[0, 7, 10]], ghosts: &[&[], &[15]], lick_p: [0.25, 0.5], hats: [2, 2],
  mix: Mix { tone: [5200.0, 6000.0], keys_lp: 2100.0, trem: 0.14, drums: 0.26, drums_lp: 3500.0, verb: 1.2, crackle: 0.2, bass_lp: 340.0, ..CHILL_MIX },
  ..CHILL
};

fn vibe(name: &str) -> (&'static str, Vibe) {
  match name {
    "rainy" => ("rainy", RAINY),
    "sunny" => ("sunny", SUNNY),
    "late" => ("late", LATE),
    "jazz" => ("jazz", JAZZ),
    "study" => ("study", STUDY),
    "snow" => ("snow", SNOW),
    _ => ("chill", CHILL),
  }
}

// --- track ------------------------------------------------------------------

struct Rng(u32);
impl Rng { // mulberry32, bit-for-bit the JS one
  fn next(&mut self) -> f64 {
    self.0 = self.0.wrapping_add(0x6d2b79f5);
    let a = self.0;
    let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
    t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
    (t ^ (t >> 14)) as f64 / 4294967296.0
  }
  fn below(&mut self, n: usize) -> usize { (self.next() * n as f64) as usize }
  fn pick<T: Copy>(&mut self, a: &[T]) -> T { a[self.below(a.len())] }
}

#[derive(Clone, Copy, PartialEq)]
enum Part { Intro, A, B, Outro }

struct Section { part: Part, prog: Prog, kick: Steps, drums: f64, hats: i32, bass: bool, lick: Vec<(i32, i32, i32)>, lick_p: f64 }

struct Track {
  v: Vibe, vibe: &'static str, name: String, bpm: i32, key: i32, bars: Vec<(usize, usize, usize)>, // (section, i, n)
  sections: Vec<Section>, r: Rng, sd: f64, swing: f64, strum: f64, chord_bars: f64, comp: Comp, bass: Bass, ghost: Steps,
}

fn make_track(seed: u32, vibe_name: &str) -> Track {
  let (vibe_name, v) = vibe(vibe_name);
  let mut r = Rng(seed);
  let name = format!("{} {}", r.pick(PLACES), r.pick(v.moods));
  let bpm = v.bpm[0] + (r.next() * (v.bpm[1] - v.bpm[0] + 1) as f64) as i32;
  let key = v.key + (r.next() * 8.0) as i32;
  // A and B differ when the vibe has 2+ choices (the JS `other`)
  let two = |r: &mut Rng, n: usize| { let a = r.below(n); let b = r.below(n); (a, if a == b { (a + 1) % n } else { b }) };
  let (pa, pb) = two(&mut r, v.progs.len());
  let (ka, kb) = two(&mut r, v.kicks.len());
  let lick = |r: &mut Rng| { // a 2-bar pentatonic noodle, [step 0..31, midi, length]
    let mut notes: Vec<(i32, i32, i32)> = vec![];
    let mut s = r.below(4) as i32 * 2;
    let mut d = 3 + r.below(4) as i32;
    let n = 4 + r.below(4);
    while notes.len() < n && s < 30 {
      notes.push((s, key + 12 + v.scale[(d % 5) as usize] + 12 * (d / 5), 2));
      let gap = r.pick(&[2, 2, 3, 4, 1, 6]);
      let l = notes.len();
      if l > 1 { notes[l - 2].2 = 2.max(s - notes[l - 2].0) }
      s += gap;
      d = 11.min(2.max(d + r.pick(&[-2, -1, -1, 1, 1, 2])));
    }
    notes.last_mut().unwrap().2 = 8;
    notes
  };
  let lick_a = lick(&mut r);
  let lick_b = lick(&mut r);
  let sec = |part, prog, kick, drums, hats, bass, lick, lick_p| Section { part, prog, kick, drums, hats, bass, lick, lick_p };
  let sections = vec![
    sec(Part::Intro, v.progs[pa], &[][..], 0.0, 1, false, vec![], 0.0),
    sec(Part::A, v.progs[pa], v.kicks[ka], v.drums, v.hats[0], true, lick_a, v.lick_p[0]),
    sec(Part::B, v.progs[pb], v.kicks[kb], v.drums, v.hats[1], true, lick_b, v.lick_p[1]),
    sec(Part::Outro, v.progs[pa], v.kicks[ka], v.drums * 0.5, v.hats[0], true, vec![], 0.0),
  ];
  let mut bars = vec![];
  for &(s, n) in r.pick(FORMS) {
    let k = ["intro", "A", "B", "outro"].iter().position(|&x| x == s).unwrap();
    for i in 0..n { bars.push((k, i, n)) }
  }
  let swing = v.swing[0] + r.next() * (v.swing[1] - v.swing[0]);
  let strum = 0.006 + r.next() * 0.014;
  let chord_bars = r.pick(v.chord_bars);
  let comp = r.pick(v.comps);
  let bass = r.pick(v.basses);
  let ghost = r.pick(v.ghosts);
  Track { v, vibe: vibe_name, name, bpm, key, bars, sections, r, sd: 60.0 / bpm as f64 / 4.0, swing, strum, chord_bars, comp, bass, ghost }
}

// --- Web Audio building blocks ------------------------------------------------

// AudioParam automation, read at increasing times. Between events it steps sample to sample (an add, a multiply
// or an approach to the target) instead of calling exp/powf on every sample.
#[derive(Clone, Copy)]
enum Ev { Set(f64), Lin(f64), Exp(f64), Target(f64, f64) }
#[derive(Clone, Copy)]
enum Seg { Hold(f64), Target(f64, f64, f64, f64) } // target, time constant, start time, start value
#[derive(Clone, Copy)]
enum Step { Const, Add(f64), Mul(f64), Toward(f64, f64) }

struct Param { ev: Vec<(f64, Ev)>, i: usize, base: Seg, pt: f64, pv: f64, v: f64, last: f64, step: Step }
impl Param {
  fn new(v: f64) -> Self { Param { ev: vec![], i: 0, base: Seg::Hold(v), pt: 0.0, pv: v, v, last: -1.0, step: Step::Const } }
  fn at(&mut self, t: f64, e: Ev) -> &mut Self {
    let i = self.ev.partition_point(|x| x.0 <= t);
    self.ev.insert(i, (t, e));
    self
  }
  fn curve(&self, t: f64) -> f64 {
    if let Some(&(t1, e)) = self.ev.get(self.i) {
      let (pt, pv) = (self.pt, self.pv);
      match e { // a ramp runs from the previous event to its own time
        Ev::Lin(x) if t1 > pt => return pv + (x - pv) * (t - pt) / (t1 - pt),
        Ev::Exp(x) if t1 > pt && pv * x > 0.0 => return pv * (x / pv).powf((t - pt) / (t1 - pt)),
        _ => {}
      }
    }
    match self.base { Seg::Hold(v) => v, Seg::Target(x, tc, t0, v0) => x + (v0 - x) * (-(t - t0) / tc).exp() }
  }
  fn slope(&self) -> Step {
    if let Some(&(t1, e)) = self.ev.get(self.i) {
      let (pt, pv) = (self.pt, self.pv);
      match e {
        Ev::Lin(x) if t1 > pt => return Step::Add((x - pv) / ((t1 - pt) * SR)),
        Ev::Exp(x) if t1 > pt && pv * x > 0.0 => return Step::Mul((x / pv).powf(1.0 / ((t1 - pt) * SR))),
        _ => {}
      }
    }
    match self.base { Seg::Hold(_) => Step::Const, Seg::Target(x, tc, ..) => Step::Toward(x, (-1.0 / (tc * SR)).exp()) }
  }
  fn value(&mut self, t: f64) -> f64 {
    let mut exact = (t - self.last - 1.0 / SR).abs() > 1e-9; // not the next sample
    while self.i < self.ev.len() && self.ev[self.i].0 <= t {
      exact = true;
      let (te, e) = self.ev[self.i];
      let cur = self.curve(te);
      self.i += 1;
      self.pv = match e {
        Ev::Set(x) | Ev::Lin(x) | Ev::Exp(x) => { self.base = Seg::Hold(x); x }
        Ev::Target(x, tc) => { self.base = Seg::Target(x, tc, te, cur); cur }
      };
      self.pt = te;
    }
    self.last = t;
    if exact {
      self.v = self.curve(t);
      self.step = self.slope();
    } else {
      self.v = match self.step { Step::Const => self.v, Step::Add(d) => self.v + d, Step::Mul(m) => self.v * m, Step::Toward(x, k) => x + (self.v - x) * k };
    }
    self.v
  }
}

#[derive(Clone, Copy)]
enum Ft { Lp, Hp, Bp }
// BiquadFilterNode, with the Web Audio coefficient formulas (Q in dB for low/high-pass).
struct Bq { ft: Ft, qd: f64, f: f64, c: [f64; 5], s: [f64; 4] }
impl Bq {
  fn new(ft: Ft, f: f64, q: f64) -> Self {
    let qd = match ft { Ft::Bp => 2.0 * q, _ => 2.0 * 10f64.powf(q / 20.0) };
    let mut b = Bq { ft, qd, f: -1.0, c: [0.0; 5], s: [0.0; 4] };
    b.set(f);
    b
  }
  fn set(&mut self, f: f64) {
    if f == self.f { return }
    self.f = f;
    let w = 2.0 * PI * f.clamp(1.0, SR / 2.0 - 1.0) / SR;
    let (sn, cs) = w.sin_cos();
    let al = sn / self.qd;
    let (b, a) = match self.ft {
      Ft::Lp => ([(1.0 - cs) / 2.0, 1.0 - cs, (1.0 - cs) / 2.0], [1.0 + al, -2.0 * cs, 1.0 - al]),
      Ft::Hp => ([(1.0 + cs) / 2.0, -(1.0 + cs), (1.0 + cs) / 2.0], [1.0 + al, -2.0 * cs, 1.0 - al]),
      Ft::Bp => ([al, 0.0, -al], [1.0 + al, -2.0 * cs, 1.0 - al]),
    };
    self.c = [b[0] / a[0], b[1] / a[0], b[2] / a[0], a[1] / a[0], a[2] / a[0]];
  }
  fn run(&mut self, x: f64) -> f64 {
    let [b0, b1, b2, a1, a2] = self.c;
    let [x1, x2, y1, y2] = self.s;
    let y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
    self.s = [x, x1, y, y1];
    y
  }
}

// floor is a libm call on baseline x86-64; a truncating cast isn't. Phases can go negative under strong FM.
fn frac(p: f64) -> f64 { let f = p - (p as i64) as f64; if f < 0.0 { f + 1.0 } else { f } }
fn tri(p: f64) -> f64 { let p = frac(p); if p < 0.25 { 4.0 * p } else if p < 0.75 { 2.0 - 4.0 * p } else { 4.0 * p - 4.0 } }
static SINE: std::sync::LazyLock<Vec<f64>> = std::sync::LazyLock::new(|| (0..=4096).map(|i| (2.0 * PI * i as f64 / 4096.0).sin()).collect());
fn sine(p: f64) -> f64 { // table lookup, error ~1e-6
  let x = frac(p) * 4096.0;
  let (i, f) = (x as usize, frac(x));
  SINE[i] + (SINE[i + 1] - SINE[i]) * f
}
fn mtof(m: f64) -> f64 { 440.0 * 2f64.powf((m - 69.0) / 12.0) }
fn cents(c: f64) -> f64 { 2f64.powf(c / 1200.0) }

// Fractional delay line with linear interpolation (DelayNode).
struct Delay { buf: Vec<f64>, w: usize }
impl Delay {
  fn new(max: f64) -> Self { Delay { buf: vec![0.0; (max * SR) as usize + 2], w: 0 } }
  fn read(&self, d: f64) -> f64 { // d samples back from the next write
    let n = self.buf.len();
    let p = self.w as f64 + n as f64 - d.max(1.0);
    let (i, f) = (p as usize, frac(p));
    self.buf[i % n] * (1.0 - f) + self.buf[(i + 1) % n] * f
  }
  fn write(&mut self, x: f64) { self.buf[self.w] = x; self.w = (self.w + 1) % self.buf.len() }
}

fn fft(a: &mut [(f64, f64)], inverse: bool) { // iterative radix-2
  let n = a.len();
  let mut j = 0;
  for i in 1..n {
    let mut bit = n >> 1;
    while j & bit != 0 { j ^= bit; bit >>= 1 }
    j |= bit;
    if i < j { a.swap(i, j) }
  }
  let sign = if inverse { 1.0 } else { -1.0 };
  let tw: Vec<(f64, f64)> = (0..n / 2).map(|k| { let (s, c) = (2.0 * PI * k as f64 / n as f64).sin_cos(); (c, sign * s) }).collect();
  let mut len = 2;
  while len <= n {
    for i in (0..n).step_by(len) {
      for k in 0..len / 2 {
        let (wr, wi) = tw[k * (n / len)];
        let (ur, ui) = a[i + k];
        let (xr, xi) = a[i + k + len / 2];
        let (vr, vi) = (xr * wr - xi * wi, xr * wi + xi * wr);
        a[i + k] = (ur + vr, ui + vi);
        a[i + k + len / 2] = (ur - vr, ui - vi);
      }
    }
    len <<= 1;
  }
  if inverse { for x in a.iter_mut() { x.0 /= n as f64; x.1 /= n as f64 } }
}

// Overlap-add FFT convolution of a mono signal with a stereo impulse response. The left IR goes in the real
// part and the right in the imaginary part, so one inverse FFT gives both channels. Blocks run in parallel.
fn convolve(x: &[f32], ir: [&[f64]; 2]) -> [Vec<f32>; 2] {
  let l = ir[0].len();
  let m = (4 * l).next_power_of_two();
  let b = m - l + 1;
  let mut h: Vec<(f64, f64)> = (0..m).map(|i| if i < l { (ir[0][i], ir[1][i]) } else { (0.0, 0.0) }).collect();
  fft(&mut h, false);
  let blocks: Vec<usize> = (0..x.len()).step_by(b).collect();
  let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(blocks.len().max(1));
  let parts: Vec<Vec<(usize, Vec<(f64, f64)>)>> = std::thread::scope(|sc| {
    let handles: Vec<_> = blocks.chunks(blocks.len().div_ceil(threads).max(1)).map(|mine| {
      let h = &h;
      sc.spawn(move || {
        let mut out = vec![];
        for &start in mine {
          let mut y: Vec<(f64, f64)> = x[start..(start + b).min(x.len())].iter().map(|&v| (v as f64, 0.0)).collect();
          if y.iter().all(|v| v.0 == 0.0) { continue }
          y.resize(m, (0.0, 0.0));
          fft(&mut y, false);
          for (a, &(br, bi)) in y.iter_mut().zip(h) { *a = (a.0 * br - a.1 * bi, a.0 * bi + a.1 * br) }
          fft(&mut y, true);
          out.push((start, y));
        }
        out
      })
    }).collect();
    handles.into_iter().map(|h| h.join().unwrap()).collect()
  });
  let mut out = [vec![0f32; x.len()], vec![0f32; x.len()]];
  for (start, y) in parts.into_iter().flatten() {
    for (k, v) in y.iter().enumerate().take(x.len().saturating_sub(start)) {
      out[0][start + k] += v.0 as f32;
      out[1][start + k] += v.1 as f32;
    }
  }
  out
}

// --- the song ----------------------------------------------------------------

struct Song {
  n: usize, noise: Rng,
  keys: Vec<f32>, bass: Vec<f32>, lead: Vec<f32>, drums: Vec<f32>, snares: Vec<f32>,
  music: Param, mix: Param, tone: Param,
}

impl Song {
  fn span(&self, t: f64, end: f64) -> std::ops::Range<usize> {
    ((t * SR).ceil().max(0.0) as usize).min(self.n)..((end * SR).ceil() as usize).min(self.n)
  }

  // electric piano: detuned sine + triangle, a 1:1 FM bark that decays
  fn ep(&mut self, t: f64, m: f64, v: f64, d: f64, bark: f64, lead: bool) {
    let f = mtof(m);
    let mut g = Param::new(0.0);
    g.at(t, Ev::Set(0.0)).at(t + 0.006, Ev::Lin(v)).at(t + 0.006, Ev::Target(v * 0.35, 0.7)).at(t + d, Ev::Target(0.0, 0.12));
    let mut mg = Param::new(0.0);
    mg.at(t, Ev::Set(f * 1.4 * bark)).at(t, Ev::Target(f * 0.3 * bark, 0.12));
    let (mut pa, mut pb, mut pm, fb) = (0.0, 0.0, 0.0, f * cents(7.0));
    let range = self.span(t, t + d + 0.8);
    let out = if lead { &mut self.lead } else { &mut self.keys };
    for i in range {
      let tt = i as f64 / SR;
      let fm = sine(pm) * mg.value(tt);
      out[i] += (g.value(tt) * (sine(pa) + 0.6 * tri(pb))) as f32;
      pm += f / SR;
      pa += (f + fm) / SR;
      pb += fb / SR;
    }
  }

  // slow swell of detuned triangles, long release
  fn pad(&mut self, t: f64, m: f64, v: f64, d: f64) {
    let f = mtof(m);
    let mut g = Param::new(0.0);
    g.at(t, Ev::Set(0.0)).at(t + 1.2, Ev::Lin(v)).at(t + d, Ev::Target(0.0, 0.7));
    let (fa, fb) = (f * cents(-6.0), f * cents(6.0));
    for i in self.span(t, t + d + 3.0) {
      let k = (i as f64 / SR - t).max(0.0);
      self.keys[i] += (g.value(i as f64 / SR) * (tri(fa * k) + tri(fb * k))) as f32;
    }
  }

  fn bass_note(&mut self, t: f64, m: f64, v: f64, d: f64) {
    let f = mtof(m);
    let mut g = Param::new(0.0);
    g.at(t, Ev::Set(0.0)).at(t + 0.012, Ev::Lin(v)).at(t + 0.02, Ev::Target(v * 0.7, 0.3)).at(t + d, Ev::Target(0.0, 0.06));
    for i in self.span(t, t + d + 0.5) {
      let k = (i as f64 / SR - t).max(0.0);
      self.bass[i] += (g.value(i as f64 / SR) * (sine(f * k) + 0.3 * tri(f * k))) as f32;
    }
  }

  fn kick(&mut self, t: f64, v: f64) {
    let mut g = Param::new(0.0);
    g.at(t, Ev::Set(0.0)).at(t + 0.004, Ev::Lin(v)).at(t + 0.35, Ev::Exp(0.001));
    let mut f = Param::new(130.0);
    f.at(t, Ev::Set(130.0)).at(t + 0.12, Ev::Exp(46.0));
    let mut p = 0.0;
    for i in self.span(t, t + 0.5) {
      let tt = i as f64 / SR;
      self.drums[i] += (g.value(tt) * sine(p)) as f32;
      p += f.value(tt) / SR;
    }
    self.music.at(t, Ev::Target(0.5, 0.008)).at(t + 0.06, Ev::Target(1.0, 0.14)); // sidechain-ish duck
  }

  // filtered noise burst
  fn hit(&mut self, t: f64, v: f64, ft: Ft, f: f64, q: f64, dec: f64, snare: bool, att: f64) {
    let mut g = Param::new(0.0);
    g.at(t, Ev::Set(0.0)).at(t + att, Ev::Lin(v)).at(t + att + dec, Ev::Exp(0.001));
    let mut bq = Bq::new(ft, f, q);
    for i in self.span(t, t + att + dec + 0.02) {
      let x = bq.run(self.noise.next() * 2.0 - 1.0) * g.value(i as f64 / SR);
      if snare { self.snares[i] += x as f32 } else { self.drums[i] += x as f32 }
    }
  }

  fn tonal(&mut self, t: f64, v: f64, f0: f64, f1: f64, dec: f64, snare: bool) {
    let mut g = Param::new(v);
    g.at(t, Ev::Set(v)).at(t + dec, Ev::Exp(0.001));
    let mut f = Param::new(f0);
    f.at(t, Ev::Set(f0)).at(t + dec, Ev::Exp(f1));
    let mut p = 0.0;
    for i in self.span(t, t + dec + 0.02) {
      let tt = i as f64 / SR;
      let x = g.value(tt) * tri(p);
      p += f.value(tt) / SR;
      if snare { self.snares[i] += x as f32 } else { self.drums[i] += x as f32 }
    }
  }

  fn snare(&mut self, kit: Kit, t: f64, v: f64) {
    match kit {
      Kit::Tape => { self.hit(t, v * 0.7, Ft::Bp, 1900.0, 0.7, 0.18, true, 0.002); self.tonal(t, v * 0.5, 190.0, 160.0, 0.09, true) }
      Kit::Brush => self.hit(t, v * 0.55, Ft::Bp, 2600.0, 0.5, 0.24, true, 0.025), // slow-attack swishes, not sticks
    }
  }
  fn rim(&mut self, kit: Kit, t: f64, v: f64) {
    match kit {
      Kit::Tape => { self.hit(t, v, Ft::Bp, 3200.0, 3.0, 0.03, false, 0.002); self.tonal(t, v * 0.6, 900.0, 850.0, 0.03, false) }
      Kit::Brush => self.hit(t, v * 0.8, Ft::Bp, 2200.0, 0.6, 0.08, false, 0.012),
    }
  }
  fn hat(&mut self, kit: Kit, t: f64, v: f64, open: bool) {
    match kit {
      Kit::Tape => self.hit(t, v, Ft::Hp, 6500.0, 0.5, if open { 0.3 } else { 0.045 }, false, 0.002),
      Kit::Brush => self.hit(t, v * 0.7, Ft::Bp, 5200.0, 0.7, 0.1, false, 0.018),
    }
  }
}

fn voice(t: &Track, (root, q): Chord) -> Vec<i32> {
  let iv = quality(q);
  let mut ms: Vec<i32> = iv[if iv.len() > 4 { 1 } else { 0 }..].iter().map(|&i| {
    let mut m = t.key + root + i;
    while m < 52 + t.v.lift { m += 12 }
    while m > 71 + t.v.lift { m -= 12 }
    m
  }).collect();
  ms.sort();
  ms
}

// Schedules every 16th note of the track, like the web engine's step(), then mixes the buses.
fn render(t: &mut Track) -> [Vec<f32>; 2] {
  let len = START + t.bars.len() as f64 * 16.0 * t.sd - 0.01;
  let n = (len * SR).ceil() as usize;
  let (v, sd, mx) = (t.v, t.sd, t.v.mix);
  let mut s = Song {
    n, noise: Rng(t.r.0 ^ 0x9e3779b9),
    keys: vec![0.0; n], bass: vec![0.0; n], lead: vec![0.0; n], drums: vec![0.0; n], snares: vec![0.0; n],
    music: Param::new(1.0), mix: Param::new(1.0), tone: Param::new(5200.0),
  };
  let mut lick_on = false;
  for bar in 0..t.bars.len() {
    let (si, bi, bn) = t.bars[bar];
    let sec = &t.sections[si];
    let (part, prog) = (sec.part, sec.prog);
    for st in 0..16i32 {
      let time = START + (bar * 16 + st as usize) as f64 * sd;
      let ts = time + if st % 2 == 1 { t.swing * sd } else { 0.0 };
      let hum = |r: &mut Rng| ts + (r.next() - 0.5) * 0.008;
      if st == 0 {
        if bi == 0 { // section start: filter sweeps, the outro fade
          let dur = bn as f64 * 16.0 * sd;
          match part {
            Part::Intro => { s.mix.at(time, Ev::Set(1.0)); s.tone.at(time, Ev::Set(500.0)).at(time, Ev::Target(mx.tone[0], dur / 3.0)); }
            Part::Outro => { s.tone.at(time, Ev::Target(600.0, dur / 3.0)); s.mix.at(time + dur * 0.4, Ev::Target(0.0, dur / 6.0)); }
            Part::B => { s.tone.at(time, Ev::Target(mx.tone[1], 0.4)); }
            Part::A => { s.tone.at(time, Ev::Target(mx.tone[0], 0.4)); }
          }
        }
        if bi % 4 == 2 { lick_on = !sec.lick.is_empty() && t.r.next() < sec.lick_p }
        if bi % 4 < 2 { lick_on = false }
      }
      let span = (16.0 * t.chord_bars) as usize;
      let at = bi * 16 + st as usize; // position within the section, in steps
      let chord_i = |p: usize| (p / span) % prog.len();
      let chord = prog[chord_i(at)];
      if v.keys == Keys::Pad {
        if at % span == 0 { for m in voice(t, chord) { s.pad(time, m as f64, 0.045, span as f64 * sd - 0.05) } }
      } else {
        for &(cs, cd) in t.comp {
          if cs != st { continue }
          for (k, m) in voice(t, chord).into_iter().enumerate() {
            let at = ts + k as f64 * t.strum + t.r.next() * 0.006;
            let vel = 0.08 + t.r.next() * 0.03;
            s.ep(at, m as f64, vel, cd as f64 * sd - 0.03, v.bark, false);
          }
        }
      }
      if sec.bass {
        let lo = v.bass_lo;
        let root_of = |c: Chord| { let m = (t.key + c.0) % 12 + lo; if m > lo + 9 { m - 12 } else { m } };
        let root = root_of(chord);
        match t.bass {
          Bass::Walk => if st % 4 == 0 { // quarter-note walk: root, chord tones, then a step into the next chord
            let nx = chord_i(at + 4);
            let q = quality(chord.1);
            let m = if nx != chord_i(at) { root_of(prog[nx]) + if t.r.next() < 0.6 { -1 } else { 1 } }
              else if at % span == 0 { root } else { root + [q[1], 7, q[3]][t.r.below(3)] };
            let h = hum(&mut t.r);
            s.bass_note(h, m as f64, 0.12, 4.0 * sd * 0.85);
          },
          Bass::Notes(notes) => for &(bs, iv, bd) in notes {
            if bs == st { let h = hum(&mut t.r); s.bass_note(h, (root + iv) as f64, 0.13, bd as f64 * sd - 0.02) }
          },
        }
      }
      let brk = bi == bn - 1 && part != Part::Outro; // last bar of a section: drop the kick, half the hats
      if sec.drums > 0.0 && (bi as f64) < bn as f64 * sec.drums {
        if sec.kick.contains(&st) && !(brk && st > 0) { let h = hum(&mut t.r); s.kick(h, if v.kit == Kit::Brush { 0.42 } else { 0.55 }) }
        if st == 4 || st == 12 { let h = hum(&mut t.r); let vel = 0.5 + t.r.next() * 0.1; s.snare(v.kit, h, vel) }
        else if t.ghost.contains(&st) && t.r.next() < 0.6 { let h = hum(&mut t.r); s.rim(v.kit, h, 0.12) }
        if st % sec.hats == 0 && !(brk && st >= 8) {
          let h = hum(&mut t.r);
          let vel = (if st % 4 == 0 { 0.3 } else if st % 2 == 0 { 0.2 } else { 0.09 }) * (0.8 + t.r.next() * 0.4);
          s.hat(v.kit, h, vel, sec.hats == 1 && st == 14 && bi % 2 == 1);
        }
      }
      if lick_on && bi % 4 >= 2 {
        let ls = (bi % 4 - 2) as i32 * 16 + st;
        for &(nn, m, d) in &sec.lick { if nn == ls { let h = hum(&mut t.r); s.ep(h, m as f64, 0.09, d as f64 * sd, v.bark, true) } }
      }
    }
  }
  mixdown(s, mx, sd)
}

// The bus graph:
//   keys -> lowpass -> tremolo -> music;  keys lowpass -> 0.3 -> verb
//   bass -> lowpass -> music;  lead -> lowpass -> music, lead -> 0.4 -> verb, lead -> echo (feedback) -> 0.3 -> music
//   snares -> drums, snares -> 0.25 -> verb;  drums -> gain -> lowpass -> mix;  verb -> return -> music
//   music (ducked by the kick) -> mix -> tape wow -> tone lowpass -> soft clip <- vinyl crackle, hiss, rain
fn mixdown(mut s: Song, mx: Mix, sd: f64) -> [Vec<f32>; 2] {
  let n = s.n;
  let set = |init: f64, v: f64| { let mut p = Param::new(init); p.at(START, Ev::Set(v)); p };
  let mut keys_lp = (Bq::new(Ft::Lp, 2400.0, 0.6), set(2400.0, mx.keys_lp));
  // pass 1: keys low-pass in place, the reverb send replaces the snares bus once it's mixed into the drums
  for i in 0..n {
    let tt = i as f64 / SR;
    keys_lp.0.set(keys_lp.1.value(tt));
    s.keys[i] = keys_lp.0.run(s.keys[i] as f64) as f32;
    s.drums[i] += s.snares[i];
    s.snares[i] = 0.3 * s.keys[i] + 0.4 * s.lead[i] + 0.25 * s.snares[i];
  }
  // a short generated room, normalised like ConvolverNode does
  let mut rr = Rng(s.noise.0 ^ 0x51ed270b);
  let il = (1.4 * SR).ceil() as usize;
  let irs: Vec<Vec<f64>> = (0..2).map(|_| {
    let mut y = 0.0;
    (0..il).map(|i| { y += 0.3 * ((rr.next() * 2.0 - 1.0) - y); y * (1.0 - i as f64 / il as f64).powi(3) }).collect()
  }).collect();
  let power = (irs.iter().flatten().map(|v| v * v).sum::<f64>() / (2 * il) as f64).sqrt().max(0.000125);
  let scale = 1.0 / power * 0.00125 * 0.5;
  let wet = convolve(&s.snares, [&irs[0], &irs[1]]);

  let mut trem = set(0.18, mx.trem);
  let mut bass_lp = (Bq::new(Ft::Lp, 420.0, 0.7), set(420.0, mx.bass_lp));
  let mut lead_lp = Bq::new(Ft::Lp, 3200.0, 0.5);
  let (mut echo, mut echo_lp, echo_d) = (Delay::new(2.0), Bq::new(Ft::Lp, 1800.0, 0.4), sd * 3.0 * SR);
  let (mut drums_g, mut drums_lp) = (set(0.42, mx.drums), (Bq::new(Ft::Lp, 9000.0, 0.3), set(9000.0, mx.drums_lp)));
  let mut verb = set(0.7, mx.verb);
  let mut wow = [Delay::new(0.05), Delay::new(0.05)];
  let mut tone = [Bq::new(Ft::Lp, 5200.0, 0.4), Bq::new(Ft::Lp, 5200.0, 0.4)];
  let (mut crackle_g, mut hiss_g, mut rain_g) = (set(0.22, mx.crackle), set(0.02, mx.hiss), set(0.0, mx.rain));
  let (mut crackle_hp, mut hiss_bp, mut rain_hp) = (Bq::new(Ft::Hp, 500.0, 0.7), Bq::new(Ft::Bp, 5000.0, 0.3), Bq::new(Ft::Hp, 250.0, 0.5));
  let (mut crk, mut drops, mut wash) = ([0f64; 64], [0f64; 128], 0.0);
  let shape = |x: f64| 0.95 * (1.5 * x.clamp(-1.0, 1.0)).tanh() / 1.5f64.tanh();
  let lfo = |f: f64, t: f64| sine(f * t);
  let r = &mut s.noise;
  let mut out = [vec![0f32; n], vec![0f32; n]];
  for i in 0..n {
    let tt = i as f64 / SR;
    let keys = s.keys[i] as f64 * (0.82 + lfo(3.1, tt) * trem.value(tt));
    bass_lp.0.set(bass_lp.1.value(tt));
    let eo = echo.read(echo_d);
    echo.write(s.lead[i] as f64 + 0.35 * echo_lp.run(eo));
    let dry = keys + bass_lp.0.run(s.bass[i] as f64) + lead_lp.run(s.lead[i] as f64) + 0.3 * eo;
    drums_lp.0.set(drums_lp.1.value(tt));
    let drums = drums_lp.0.run(s.drums[i] as f64 * drums_g.value(tt));
    let (music, mix, rv) = (s.music.value(tt), s.mix.value(tt), verb.value(tt));
    let wd = (0.012 + 0.0016 * lfo(0.27, tt) + 0.00007 * lfo(5.3, tt)) * SR; // tape wow and flutter
    let tf = s.tone.value(tt);
    // vinyl loops, from the engine start: sparse clicks, hiss, and rain (a soft wash plus tiny droplets)
    let mut vinyl = 0.0;
    if tt >= START {
      if r.next() <= 14.0 / SR {
        let a = (0.15 + 0.85 * r.next().powi(3)) * if r.next() < 0.5 { -1.0 } else { 1.0 };
        for k in 0..40 { crk[(i + k) % 64] += a * (-(k as f64) / 6.0).exp() * if k % 2 == 1 { -0.6 } else { 1.0 } }
      }
      wash += 0.08 * ((r.next() * 2.0 - 1.0) - wash);
      if r.next() <= 90.0 / SR {
        let (a, f) = (0.02 + 0.1 * r.next().powi(2), 0.25 + r.next() * 0.5);
        for k in 0..120 { drops[(i + k) % 128] += a * (-(k as f64) / 25.0).exp() * (k as f64 * f).sin() }
      }
      vinyl = crackle_hp.run(crk[i % 64]) * crackle_g.value(tt) + hiss_bp.run(r.next() * 2.0 - 1.0) * hiss_g.value(tt)
        + rain_hp.run(wash * 0.35 + drops[i % 128]) * rain_g.value(tt);
      crk[i % 64] = 0.0;
      drops[i % 128] = 0.0;
    }
    for c in 0..2 {
      let m = ((dry + rv * scale * wet[c][i] as f64) * music + drums) * mix;
      let w = wow[c].read(wd);
      wow[c].write(m);
      if c == 0 { tone[0].set(tf) } else { (tone[1].f, tone[1].c) = (tone[0].f, tone[0].c) }
      out[c][i] = shape(tone[c].run(w) + vinyl) as f32;
    }
  }
  out
}

fn wav(path: &str, ch: &[Vec<f32>; 2], title: &str, artist: &str) -> std::io::Result<()> {
  let chunk = |id: &str, body: &[u8]| { // RIFF chunks are padded to even lengths
    let mut c = id.as_bytes().to_vec();
    c.extend((body.len() as u32).to_le_bytes());
    c.extend(body);
    if body.len() % 2 == 1 { c.push(0) }
    c
  };
  let z = |s: &str| [s.as_bytes(), &[0]].concat();
  let info = [b"INFO".to_vec(), chunk("INAM", &z(title)), chunk("IART", &z(artist))].concat();
  let mut fmt = vec![];
  for v in [1u16, 2] { fmt.extend(v.to_le_bytes()) }
  for v in [SR as u32, SR as u32 * 4] { fmt.extend(v.to_le_bytes()) }
  for v in [4u16, 16] { fmt.extend(v.to_le_bytes()) }
  let mut data = Vec::with_capacity(ch[0].len() * 4);
  for i in 0..ch[0].len() { for c in ch { data.extend(((c[i].clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()) } }
  let body = [b"WAVE".to_vec(), chunk("fmt ", &fmt), chunk("LIST", &info), chunk("data", &data)].concat();
  std::fs::write(path, chunk("RIFF", &body))
}

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.len() != 3 {
    eprintln!("usage: lofi <chill|rainy|sunny|late|jazz|study|snow> <seed> <out.wav>");
    std::process::exit(2);
  }
  let seed: u32 = args[1].parse().unwrap_or_else(|_| { eprintln!("seed must be a number up to 4294967295"); std::process::exit(2) });
  let mut t = make_track(seed, &args[0]);
  let out = render(&mut t);
  let artist = format!("Lo-fi · {}", t.v.label);
  if let Err(e) = wav(&args[2], &out, &t.name, &artist) {
    eprintln!("can't write {}: {e}", args[2]);
    std::process::exit(1);
  }
  let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
  println!(r#"{{"name":"{}","vibe":"{}","label":"{}","bpm":{},"duration":{:.3}}}"#,
    esc(&t.name), t.vibe, esc(t.v.label), t.bpm, out[0].len() as f64 / SR);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tracks_match_the_web_version() { // values from lofi.js makeTrack()
    for (seed, v, name, bpm, key, bars, swing, cb) in [
      (42, "chill", "Steveston Umbrellas", 87, 53, 56, 0.331684, 2.0),
      (42, "jazz", "Steveston Bistro", 90, 53, 56, 0.401123, 1.0),
      (7, "study", "Kitsilano Pages", 72, 53, 54, 0.353878, 2.0),
      (123456789, "rainy", "Dunbar Wet Cedars", 74, 46, 56, 0.219299, 2.0),
    ] {
      let t = make_track(seed, v);
      assert_eq!((t.name.as_str(), t.bpm, t.key, t.bars.len(), t.chord_bars), (name, bpm, key, bars, cb));
      assert!((t.swing - swing).abs() < 1e-6);
    }
  }

  #[test]
  fn fft_convolution_matches_direct() {
    let x: Vec<f32> = (0..3000).map(|i| ((i * 7919) % 13) as f32 - 6.0).collect();
    let ir: Vec<f64> = (0..300).map(|i| ((i * 31) % 7) as f64 - 3.0).collect();
    let ir2: Vec<f64> = ir.iter().map(|v| -2.0 * v).collect();
    let y = convolve(&x, [&ir, &ir2]);
    for i in [0, 1, 299, 1000, 2999] {
      let d: f64 = (0..=i.min(299)).map(|k| x[i - k] as f64 * ir[k]).sum();
      assert!((y[0][i] as f64 - d).abs() < 1e-3, "{i}: {} vs {d}", y[0][i]);
      assert!((y[1][i] as f64 + 2.0 * d).abs() < 2e-3, "{i}: {} vs {}", y[1][i], -2.0 * d);
    }
  }

  #[test]
  fn oscillators_wrap_negative_phase() {
    for p in [-2.75, -0.25, 0.25, 1.25] {
      assert!((sine(p) - (2.0 * PI * p).sin()).abs() < 1e-5, "{p}");
      assert!((tri(p) - tri(p + 3.0)).abs() < 1e-9, "{p}");
    }
  }

  #[test]
  fn param_automation() {
    let mut p = Param::new(0.0);
    p.at(1.0, Ev::Set(0.0)).at(2.0, Ev::Lin(1.0)).at(3.0, Ev::Target(0.0, 0.5));
    assert_eq!(p.value(0.5), 0.0);
    assert!((p.value(1.5) - 0.5).abs() < 1e-9);
    assert!((p.value(2.5) - 1.0).abs() < 1e-9);
    assert!((p.value(3.5) - (-1f64).exp()).abs() < 1e-9);
    // stepping sample by sample lands on the same curve
    let mut q = Param::new(0.0);
    q.at(0.0, Ev::Set(1.0)).at(0.1, Ev::Exp(0.001)).at(0.2, Ev::Lin(0.5)).at(0.3, Ev::Target(0.0, 0.05));
    let mut exact = Param::new(0.0);
    exact.ev = q.ev.clone();
    for i in 0..(0.5 * SR) as usize {
      let t = i as f64 / SR;
      let v = q.value(t);
      exact.last = -1.0;
      assert!((v - exact.value(t)).abs() < 1e-6, "{t}");
    }
  }
}
