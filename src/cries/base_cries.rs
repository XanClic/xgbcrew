use crate::Instruction;

macro_rules! sound_insns {
    { $($insn:ident $($val:expr),*)* } => { &[$(
        sound_insn! { $insn $($val),* },
    )*] }
}

macro_rules! sound_insn {
    { duty_cycle_pattern $($val:expr),* } => (Instruction::DutyCyclePattern($($val),*));
    { pitch_sweep $($val:expr),* } => (Instruction::PitchSweep($($val),*));
    { square_note $($val:expr),* } => (Instruction::SquareNote($($val),*));
    { noise_note $($val:expr),* } => (Instruction::NoiseNote($($val),*));
    { pitch_offset $($val:expr),* } => (Instruction::PitchOffset($($val),*));
    { duty_cycle $($val:expr),* } => (Instruction::DutyCycle($($val),*));
}

pub struct MonCryBase(
    pub &'static [Instruction],
    pub &'static [Instruction],
    pub &'static [Instruction],
    pub &'static [Instruction],
);

const BULBASAUR_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 3, 3, 0, 1
    square_note 4, 15, 7, 1984
    square_note 12, 14, 6, 1986
    square_note 6, 11, 5, 1664
    square_note 4, 12, 4, 1648
    square_note 4, 11, 5, 1632
    square_note 8, 12, 1, 1600
};
const BULBASAUR_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 3, 0, 3, 0
    square_note 3, 12, 7, 1921
    square_note 12, 11, 6, 1920
    square_note 6, 10, 5, 1601
    square_note 4, 12, 4, 1586
    square_note 6, 11, 5, 1569
    square_note 8, 10, 1, 153
};
const BULBASAUR_CH8: &[Instruction] = sound_insns! {
    noise_note 3, 14, 4, 60
    noise_note 12, 13, 6, 44
    noise_note 4, 14, 4, 60
    noise_note 8, 11, 7, 92
    noise_note 15, 12, 2, 93
};

pub(super) const BULBASAUR_BASE: MonCryBase = MonCryBase(
    BULBASAUR_CH5,
    BULBASAUR_CH6,
    &[],
    BULBASAUR_CH8,
);

const MIMIKYU_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 1, 2, 1
    pitch_sweep 3, 5
    square_note 5, 10, -1, 1910
    pitch_sweep 15, 0
    square_note 15, 15, 1, 1970
    square_note 5, 0, 0, 0
    duty_cycle_pattern 2, 2, 1, 1
    pitch_sweep 4, 7
    square_note 15, 10, -1, 1930
    pitch_sweep 15, 0
    square_note 15, 15, 1, 1950
    square_note 30, 0, 0, 0
    duty_cycle 1
    pitch_sweep 3, 7
    square_note 20, 10, 7, 1930
    pitch_sweep 3, -7
    square_note 30, 8, 2, 1950
};

pub(super) const MIMIKYU_BASE: MonCryBase = MonCryBase(
    MIMIKYU_CH5,
    &[],
    &[],
    &[],
);

const LITWICK_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 1, 1, 1, 1
    pitch_sweep 15, 7
    square_note 12, 8, -1, 1844
    pitch_sweep 15, -7
    square_note 12, 15, 1, 1844
    pitch_sweep 15, 7
    square_note 10, 3, -1, 1770
    pitch_sweep 15, 0
    square_note 10, 5, 0, 1770
    square_note 10, 3, 0, 1760
    square_note 10, 2, 0, 1770
    square_note 5, 1, 0, 1780
    square_note 10, 1, 0, 1770
    square_note 5, 1, 1, 1770
};
const LITWICK_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 1, 1, 1, 1

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 1, -1, 1820
    square_note 1, 2, 2, 1820

    square_note 2, 0, -1, 1820
    square_note 1, 1, 2, 1820

    square_note 2, 0, -1, 1820
    square_note 1, 1, 2, 1820

    square_note 2, 0, -1, 1820
    square_note 1, 1, 2, 1820

    square_note 2, 0, -1, 1820
    square_note 1, 1, 2, 1820

    square_note 2, 0, -1, 1820
    square_note 1, 1, 2, 1820
};
const LITWICK_CH8: &[Instruction] = sound_insns! {
    noise_note 14, 15, 2, 101
    noise_note 13, 10, 2, 85
    noise_note 14, 8, 2, 86
    noise_note 12, 10, 1, 102
};

pub(super) const LITWICK_BASE: MonCryBase = MonCryBase(
    LITWICK_CH5,
    LITWICK_CH6,
    &[],
    LITWICK_CH8,
);

const LAMPENT_CH5: &[Instruction] = sound_insns! {
    duty_cycle 1
    pitch_sweep 15, 7
    square_note 10, 7, 0, 1750
    pitch_sweep 15, 7
    square_note 10, 7, -1, 1820
    pitch_sweep 15, 0
    square_note 10, 10, -1, 1920
    duty_cycle_pattern 1, 0, 2, 0
    square_note 30, 13, -1, 1920

    square_note 2, 9, 8, 1913
    square_note 4, 7, 8, 1856
    square_note 4, 5, 8, 1852
    square_note 8, 3, 8, 1852
    pitch_sweep 15, -7
    duty_cycle_pattern 0, 2, 0, 2
    square_note 3, 4, 7, 1700
    square_note 3, 6, 7, 1600
    square_note 3, 8, 7, 1500
    square_note 3, 10, 7, 1450
    pitch_sweep 15, 7
    square_note 3, 14, 7, 1450
    square_note 3, 15, -7, 1400
    square_note 3, 10, -7, 1300
};
const LAMPENT_CH8: &[Instruction] = sound_insns! {
    noise_note 15, 2, 0, 0
    noise_note 10, 5, -1, 0
    noise_note 15, 10, 0, 1
    noise_note 10, 10, 7, 1
    noise_note 13, 0, 0, 0
    noise_note 5, 0, -1, 18
    noise_note 5, 5, 0, 18
    noise_note 15, 5, 7, 18
};

pub(super) const LAMPENT_BASE: MonCryBase = MonCryBase(
    LAMPENT_CH5,
    LITWICK_CH6,
    &[],
    LAMPENT_CH8,
);

const CHANDELURE_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 0, 1, 0
    pitch_sweep 11, 7
    square_note 15, 0, -7, 500
    pitch_sweep 11, 7
    square_note 15, 3, -7, 700
    pitch_sweep 11, 5
    square_note 10, 6, 3, 900
    pitch_sweep 15, 0
    duty_cycle 2
    square_note 15, 2, -7, 300
    square_note 15, 5, 7, 200
    square_note 25, 3, 4, 100
    square_note 25, 1, 1, 50
};
const CHANDELURE_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    square_note 15, 4, -7, 2002
    square_note 15, 8, 0, 2002
    square_note 15, 8, 7, 2001
    square_note 25, 4, 7, 2000
    square_note 25, 2, 1, 2000
};
const CHANDELURE_CH8: &[Instruction] = sound_insns! {
    noise_note 20, 4, -7, 16
    noise_note 20, 8, 7, 48
    noise_note 20, 4, -7, 49
    noise_note 20, 8, 7, 48
    noise_note 20, 5, 1, 52
};

pub(super) const CHANDELURE_BASE: MonCryBase = MonCryBase(
    &CHANDELURE_CH5,
    &CHANDELURE_CH6,
    &[],
    &CHANDELURE_CH8,
);

const SINISTEA_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 0
    pitch_sweep 15, 7
    square_note 20, 0, -1, 1950
    pitch_sweep 15, 0
    square_note 36, 3, 7, 2000

    square_note 30, 4, 4, 500
};
const SINISTEA_CH8: &[Instruction] = sound_insns! {
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 18
    noise_note 1, 0, -1, 18
    noise_note 1, 15, 1, 19
    noise_note 1, 0, -1, 19

    noise_note 1, 10, 1, 20
    noise_note 1, 0, -1, 20
    noise_note 1, 5, 1, 21
    noise_note 3, 0, -1, 21

    noise_note 1, 8, 3, 22
    noise_note 1, 0, -3, 22
    noise_note 1, 8, 3, 22
    noise_note 1, 0, -4, 22
    noise_note 1, 6, 4, 22
    noise_note 1, 0, -4, 22
    noise_note 1, 6, 4, 22
    noise_note 1, 0, -5, 22
    noise_note 1, 5, 5, 22
    noise_note 1, 0, -5, 22
    noise_note 1, 5, 5, 22
    noise_note 1, 0, -5, 22
    noise_note 1, 4, 5, 22
    noise_note 1, 0, -6, 22
    noise_note 1, 4, 6, 22
    noise_note 1, 0, -6, 22
    noise_note 1, 3, 6, 22
    noise_note 1, 0, -6, 22
    noise_note 1, 3, 6, 22
    noise_note 1, 0, -7, 22
    noise_note 1, 2, 7, 22
    noise_note 1, 0, -7, 22
    noise_note 1, 2, 7, 22
    noise_note 1, 0, -7, 22
    noise_note 1, 1, 7, 22
    noise_note 1, 0, -7, 22
    noise_note 1, 1, 7, 22
};

pub(super) const SINISTEA_BASE: MonCryBase = MonCryBase(
    &SINISTEA_CH5,
    &[],
    &[],
    &SINISTEA_CH8,
);

const POLTEAGEIST_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 0
    square_note 5, 0, 0, 0
    square_note 10, 5, -1, 10
    square_note 10, 15, 1, 10

    square_note 65, 0, 0, 0

    duty_cycle_pattern 2, 1, 2, 1
    square_note 5, 10, 0, 1700
    square_note 5, 0, 0, 0
    square_note 5, 10, 7, 1500
};
const POLTEAGEIST_CH8: &[Instruction] = sound_insns! {
    noise_note 15, 0, -1, 0
    noise_note 5, 10, -1, 29
    noise_note 25, 15, 1, 20
    noise_note 5, 0, -1, 14
    noise_note 5, 8, -1, 15
};

pub(super) const POLTEAGEIST_BASE: MonCryBase = MonCryBase(
    &POLTEAGEIST_CH5,
    &[],
    &[],
    &POLTEAGEIST_CH8,
);

const OCT_0: f32 = 1.00000000000000000000000000000000;
const OCT_1: f32 = 1.05946309435929530984310531493975;
const OCT_2: f32 = 1.12246204830937301721860421821475;
const OCT_3: f32 = 1.18920711500272102689734765590401;
const OCT_4: f32 = 1.25992104989487319066654436028330;
const OCT_5: f32 = 1.33483985417003436779737057804596;
const OCT_6: f32 = 1.41421356237309514547462185873883;
const OCT_7: f32 = 1.49830707687668152061633008997887;
const OCT_8: f32 = 1.58740105196819936139718265621923;
const OCT_9: f32 = 1.68179283050742900407215074665146;
const OCT_10: f32 = 1.78179743628067854821495075157145;
const OCT_11: f32 = 1.88774862536338683405290339578642;

const fn note(mut base_freq: f32, mut note: isize) -> u16 {
    while note < 0 {
        base_freq /= 2.0;
        note += 12;
    }
    while note >= 12 {
        base_freq *= 2.0;
        note -= 12;
    }
    let mult = match note {
        0 => OCT_0,
        1 => OCT_1,
        2 => OCT_2,
        3 => OCT_3,
        4 => OCT_4,
        5 => OCT_5,
        6 => OCT_6,
        7 => OCT_7,
        8 => OCT_8,
        9 => OCT_9,
        10 => OCT_10,
        11 => OCT_11,
        _ => unreachable!(),
    };
    (2048.5 - (131072.0 / (base_freq * mult))) as u16
}

const DRIFBLIM_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 0, 2, 0
    pitch_sweep 15, -5
    square_note 10, 15, 4, note(110.0, 2)
    square_note 2, 0, 0, 0
    square_note 6, 15, 4, note(110.0, 0)
    square_note 4, 0, 0, 0
    pitch_sweep 15, 6
    square_note 6, 15, 4, note(110.0, 3)
    square_note 2, 0, 0, 0
    pitch_sweep 15, -4
    square_note 7, 15, 4, note(110.0, 9)
    square_note 4, 0, 0, 0
    pitch_sweep 15, 6
    square_note 6, 15, 4, note(110.0, 5)
    square_note 3, 0, 0, 0
    pitch_sweep 15, 0
    square_note 6, 15, 4, note(110.0, 9)
    square_note 4, 0, 0, 0
    square_note 8, 15, 1, note(110.0, 12)
    square_note 2, 0, 0, 0
    square_note 8, 7, 1, note(110.0, 14)
};
const DRIFBLIM_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 2, 0, 1
    square_note 10, 10, 4, note(110.0, 2)
    square_note 2, 0, 0, 0
    square_note 6, 10, 4, note(110.0, 0)
    square_note 4, 0, 0, 0
    square_note 4, 10, 4, note(110.0, 1)
    square_note 4, 0, 0, 0
    square_note 7, 10, 4, note(110.0, 0)
    square_note 4, 0, 0, 0
    square_note 3, 10, 4, note(110.0, 9)
    square_note 2, 0, 0, 0
    square_note 10, 10, 4, note(110.0, 1)
    square_note 4, 0, 0, 0
    square_note 8, 5, 1, note(110.0, 24)
};
const DRIFBLIM_CH8: &[Instruction] = sound_insns! {
    noise_note 60, 8, 0, 127
    noise_note 1, 2, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 2, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 2, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 2, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 2, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 1, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 1, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 1, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 1, 1, 15
    noise_note 1, 0, 0, 0
    noise_note 1, 1, 1, 15
    noise_note 1, 0, 0, 0
};

pub(super) const DRIFBLIM_BASE: MonCryBase = MonCryBase(
    &DRIFBLIM_CH5,
    &DRIFBLIM_CH6,
    &[],
    &DRIFBLIM_CH8,
);

const DRIFLOON_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 1, 2, 1, 2
    pitch_sweep 1, -7
    square_note 10, 15, 7, note(880.0, 12)
    pitch_sweep 4, 7
    square_note 10, 10, -7, note(880.0, -5)
    pitch_sweep 15, 7
    square_note 10, 12, 0, note(880.0, 12)
    pitch_sweep 15, 0
    square_note 5, 0, 0, 0
    duty_cycle 1
    pitch_sweep 3, 7
    square_note 8, 2, -4, note(880.0, -5)
    square_note 2, 0, 0, 0
    square_note 8, 2, -4, note(880.0, -5)
    square_note 2, 0, 0, 0
    duty_cycle_pattern 1, 2, 1, 2
    pitch_sweep 2, 7
    square_note 5, 4, -2, note(880.0, 2)
    pitch_sweep 15, 0
    square_note 5, 6, 1, note(880.0, 15)
};
const DRIFLOON_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    square_note 10, 0, -1, note(880.0, -5)
    square_note 5, 0, 0, 0
    duty_cycle 3
    square_note 20, 0, -3, note(880.0, -9)
};

pub(super) const DRIFLOON_BASE: MonCryBase = MonCryBase(
    &DRIFLOON_CH5,
    &DRIFLOON_CH6,
    &[],
    &[],
);

const MISMAGIUS_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    pitch_sweep 5, -7
    square_note 5, 8, 7, note(440.0, 0)
    pitch_sweep 5, 7
    square_note 5, 8, -7, note(440.0, 0)
    pitch_sweep 5, -7
    square_note 5, 8, 7, note(440.0, 1)
    pitch_sweep 5, 7
    square_note 5, 8, -7, note(440.0, 1)
    pitch_sweep 5, -7
    square_note 5, 8, 7, note(440.0, 2)
    pitch_sweep 5, 7
    square_note 5, 8, -7, note(440.0, 2)
    pitch_sweep 5, -7
    square_note 5, 8, 7, note(440.0, 4)
    pitch_sweep 5, 7
    square_note 5, 8, -7, note(440.0, 4)
    pitch_sweep 5, -7
    square_note 5, 8, 7, note(440.0, 6)
    pitch_sweep 5, 7
    square_note 5, 8, -7, note(440.0, 6)
    square_note 20, 8, 2, note(440.0, 7)
};
const MISMAGIUS_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    square_note 10, 5, 0, note(220.0, 0)
    square_note 10, 5, 0, note(220.0, 1)
    square_note 10, 5, 0, note(220.0, 2)
    square_note 10, 5, 0, note(220.0, 4)
    square_note 10, 5, 0, note(220.0, 6)
    square_note 20, 5, 2, note(220.0, 7)
};

pub(super) const MISMAGIUS_BASE: MonCryBase = MonCryBase(
    &MISMAGIUS_CH5,
    &MISMAGIUS_CH6,
    &[],
    &[],
);

const SANDYGAST_CH5: &[Instruction] = sound_insns! {
    duty_cycle 2
    pitch_sweep 2, 5
    square_note 20, 0, -2, note(70.0, 0)
    pitch_sweep 15, 0
    square_note 20, 5, 0, note(110.0, 0)
    pitch_sweep 2, -5
    square_note 20, 5, 2, note(110.0, 0)
};
const SANDYGAST_CH8: &[Instruction] = sound_insns! {
    noise_note 8, 10, -5, 66
    noise_note 8, 11, 2, 65
    noise_note 8, 10, 0, 65
    noise_note 24, 10, -2, 64
    noise_note 8, 15, 2, 64
    noise_note 8, 12, 2, 65
    noise_note 24, 10, 4, 66
    noise_note 24, 5, 4, 67
};

pub(super) const SANDYGAST_BASE: MonCryBase = MonCryBase(
    &SANDYGAST_CH5,
    &[],
    &[],
    &SANDYGAST_CH8,
);

const PALOSSAND_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    pitch_sweep 3, 4
    square_note 20, 5, -2, note(70.0, 0)
    pitch_sweep 15, 0
    square_note 20, 10, -4, note(150.0, 0)
    pitch_sweep 15, -7
    square_note 20, 12, 0, note(150.0, 0)
    pitch_sweep 15, 7
    square_note 20, 12, 0, note(141.0, 0)
    pitch_sweep 1, -7
    square_note 20, 12, 3, note(150.0, 0)
};
const PALOSSAND_CH8: &[Instruction] = sound_insns! {
    noise_note 20, 10, 3, 52
    noise_note 40, 1, -4, 50
    noise_note 30, 10, -7, 51
    noise_note 40, 10, 3, 53
};

pub(super) const PALOSSAND_BASE: MonCryBase = MonCryBase(
    &PALOSSAND_CH5,
    &[],
    &[],
    &PALOSSAND_CH8,
);

const ZORUA_CH5: &[Instruction] = sound_insns! {
    duty_cycle 2
    pitch_sweep 15, 7
    square_note 7, 13, -1, 1807
    square_note 4, 0, 0, 0
    square_note 7, 13, -1, 1893
};
const ZORUA_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 0, 1, 0
    square_note 7, 8, -1, 1797
    square_note 4, 0, 0, 0
    square_note 7, 8, -1, 1883
};
const ZORUA_CH8: &[Instruction] = sound_insns! {
    noise_note 4, 5, -1, 33
    noise_note 3, 8, 1, 0
    noise_note 4, 0, 0, 0
    noise_note 3, 8, 0, 33
    noise_note 4, 8, 0, 16
};

pub(super) const ZORUA_BASE: MonCryBase = MonCryBase(
    &ZORUA_CH5,
    &ZORUA_CH6,
    &[],
    &ZORUA_CH8,
);

const ZOROARK_CH5: &[Instruction] = sound_insns! {
    duty_cycle 2
    pitch_sweep 8, 7
    square_note 30, 0, -3, note(60.0, 0)
    pitch_sweep 15, 0
    square_note 10, 6, 0, note(70.0, 0)
    pitch_sweep 2, 7
    square_note 30, 5, 1, note(70.0, 0)
};
const ZOROARK_CH8: &[Instruction] = sound_insns! {
    noise_note 2, 15, 1, 80
    noise_note 1, 8, -1, 80
    noise_note 2, 15, 1, 80
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 80
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 80
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 80
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 81
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 96
    noise_note 2, 15, 1, 96
    noise_note 1, 8, -1, 96
    noise_note 2, 12, 1, 96
    noise_note 1, 6, -1, 128
    noise_note 2, 10, 1, 96
    noise_note 1, 4, -1, 128
    noise_note 2, 8, 1, 128
    noise_note 1, 2, -1, 128
    noise_note 2, 5, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 5, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 5, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 3, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 3, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 3, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 3, 1, 128
    noise_note 1, 0, -1, 128
    noise_note 2, 3, 1, 128
    noise_note 1, 0, -1, 128
};

pub(super) const ZOROARK_BASE: MonCryBase = MonCryBase(
    &ZOROARK_CH5,
    &[],
    &[],
    &ZOROARK_CH8,
);

const VOLTORB_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 3, 3, 2, 2
    square_note 6, 8, 3, 583
    square_note 15, 6, 2, 550
    square_note 4, 5, 2, 581
    square_note 9, 6, 3, 518
    square_note 15, 8, 2, 549
    square_note 15, 4, 2, 519
};
const VOLTORB_CH8: &[Instruction] = sound_insns! {
    noise_note 8, 13, 4, 140
    noise_note 4, 14, 2, 156
    noise_note 15, 12, 6, 140
    noise_note 8, 14, 4, 172
    noise_note 15, 13, 7, 156
    noise_note 15, 15, 2, 172
};

pub(super) const VOLTORB_BASE: MonCryBase = MonCryBase(
    &VOLTORB_CH5,
    &[],
    &[],
    &VOLTORB_CH8,
);

const KRABBY_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 3, 3, 0, 0
    square_note 13, 15, 1, 1297
    square_note 13, 14, 1, 1301
    square_note 13, 14, 1, 1297
    square_note 8, 13, 1, 1297
};
const KRABBY_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 1, 1, 1
    square_note 12, 14, 1, 1292
    square_note 12, 13, 1, 1296
    square_note 14, 12, 1, 1292
    square_note 8, 12, 1, 1290
};
const KRABBY_CH8: &[Instruction] = sound_insns! {
    noise_note 14, 15, 2, 101
    noise_note 13, 14, 2, 85
    noise_note 14, 13, 2, 86
    noise_note 8, 13, 1, 102
};

pub(super) const KRABBY_BASE: MonCryBase = MonCryBase(
    &KRABBY_CH5,
    &KRABBY_CH6,
    &[],
    &KRABBY_CH8,
);

const METAPOD_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 3, 3, 1, 1
    square_note 7, 13, 6, 2017
    square_note 6, 12, 6, 2018
    square_note 9, 13, 6, 2017
    square_note 7, 12, 6, 2016
    square_note 5, 11, 6, 2018
    square_note 7, 12, 6, 2017
    square_note 6, 11, 6, 2016
    square_note 8, 10, 1, 2015
};
const METAPOD_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 1, 0, 1, 0
    square_note 6, 12, 3, 1993
    square_note 6, 11, 3, 1991
    square_note 10, 12, 4, 1987
    square_note 8, 11, 4, 1991
    square_note 6, 12, 3, 1993
    square_note 15, 10, 2, 1989
};
const METAPOD_CH8: &[Instruction] = sound_insns! {
    noise_note 13, 1, -1, 124
    noise_note 13, 15, 7, 140
    noise_note 12, 13, 6, 124
    noise_note 8, 12, 4, 108
    noise_note 15, 11, 3, 92
};

pub(super) const METAPOD_BASE: MonCryBase = MonCryBase(
    &METAPOD_CH5,
    &METAPOD_CH6,
    &[],
    &METAPOD_CH8,
);

const DUNSPARCE_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 2, 0, 2
    square_note 1, 15, 8, 1456
    square_note 1, 15, 8, 1204
    square_note 1, 15, 8, 1464
    square_note 3, 15, 8, 1472
    square_note 8, 12, 8, 1168
    square_note 8, 12, 8, 1152
    pitch_sweep 15, -6
    square_note 16, 12, 3, 1168
    pitch_sweep 8, 8
};
const DUNSPARCE_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 2, 0, 2
    square_note 8, 11, 8, 1224
    square_note 32, 11, 5, 1040
};
const DUNSPARCE_CH8: &[Instruction] = sound_insns! {
    noise_note 3, 15, -7, 75
    noise_note 3, 14, -7, 76
    noise_note 32, 11, 5, 95
};

pub(super) const DUNSPARCE_BASE: MonCryBase = MonCryBase(
    &DUNSPARCE_CH5,
    &DUNSPARCE_CH6,
    &[],
    &DUNSPARCE_CH8,
);

const LEDYBA_CH5: &[Instruction] = sound_insns! {
    pitch_offset 2
    duty_cycle 2
    square_note 3, 15, 8, 1937
    square_note 3, 13, 8, 1933
    square_note 2, 0, 0, 0
    square_note 1, 7, 8, 1729
    square_note 1, 15, 8, 1857
    square_note 4, 14, 1, 1873
};
const LEDYBA_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 0, 0, 1
    square_note 3, 15, 8, 1937
    square_note 3, 13, 8, 1933
    square_note 2, 0, 0, 0
    square_note 1, 7, 8, 1729
    square_note 1, 15, 8, 1857
    square_note 4, 14, 1, 1873
};
const LEDYBA_CH8: &[Instruction] = sound_insns! {
    noise_note 3, 5, -1, 33
    noise_note 3, 8, 1, 0
    noise_note 2, 2, 0, 0
    noise_note 1, 8, 0, 33
    noise_note 1, 8, 0, 16
    noise_note 4, 8, 7, 0
};

pub(super) const LEDYBA_BASE: MonCryBase = MonCryBase(
    &LEDYBA_CH5,
    &LEDYBA_CH6,
    &[],
    &LEDYBA_CH8,
);

const BLASTOISE_CH5: &[Instruction] = sound_insns! {
    duty_cycle_pattern 0, 3, 0, 3
    square_note 15, 15, 6, 1472
    square_note 8, 14, 3, 1468
    square_note 6, 13, 2, 1488
    square_note 6, 11, 2, 1504
    square_note 6, 12, 2, 1520
    square_note 8, 11, 1, 1536
};
const BLASTOISE_CH6: &[Instruction] = sound_insns! {
    duty_cycle_pattern 2, 1, 2, 1
    square_note 14, 12, 6, 1201
    square_note 7, 12, 3, 1197
    square_note 5, 11, 2, 1217
    square_note 8, 9, 2, 1233
    square_note 6, 10, 2, 1249
    square_note 8, 9, 1, 1265
};
const BLASTOISE_CH8: &[Instruction] = sound_insns! {
    noise_note 10, 14, 6, 92
    noise_note 10, 13, 6, 108
    noise_note 4, 12, 2, 76
    noise_note 6, 13, 3, 92
    noise_note 8, 11, 3, 76
    noise_note 8, 10, 1, 92
};

pub(super) const BLASTOISE_BASE: MonCryBase = MonCryBase(
    &BLASTOISE_CH5,
    &BLASTOISE_CH6,
    &[],
    &BLASTOISE_CH8,
);
